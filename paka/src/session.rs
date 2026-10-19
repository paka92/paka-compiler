//! The program a server holds (M4-17): its definitions in a workspace and its files on disk, kept
//! equal after each edit (docs/decisions/M4-03.md section 3.3, M4-48).
//!
//! A definition's version is the first 12 hex digits of the SHA-256 of its canonical text. An edit
//! stores the canonical text of the one definition it changes and leaves the rest as written.

use crate::diagnostic::Diagnostic;
use crate::parser::split;
use crate::print;
use crate::serve::{Call, Failure, Reply};
use crate::sha256::sha256;
use crate::source::Source;
use crate::workspace::Workspace;
use paka_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

const PATH: &str = "main.paka";

pub struct Session {
    /// The program's directory; `None` for a single file.
    dir: Option<PathBuf>,
    files: Vec<File>,
    workspace: Workspace,
    /// Held while the server runs. For a single file it moves to each new copy.
    lock: fs::File,
}

struct File {
    path: PathBuf,
    text: String,
}

impl Session {
    /// The program at `path`, read into `source` and `workspace`, with its `lock` held.
    pub fn new(path: &OsStr, source: &Source, workspace: Workspace, lock: fs::File) -> Session {
        let files = (source.files.iter())
            .map(|f| File {
                path: PathBuf::from(&f.path),
                text: String::from_utf8_lossy(&source.text[f.bytes.clone()]).into(),
            })
            .collect();
        let dir = Path::new(path).is_dir().then(|| path.into());
        Session {
            dir,
            files,
            workspace,
            lock,
        }
    }

    pub fn handle(&mut self, call: &Call) -> Reply {
        match call.name {
            "edit" => self.edit(call),
            name => Err(Failure::new(
                "unknown_call",
                format!("{name} is not served yet"),
            )),
        }
    }

    fn edit(&mut self, call: &Call) -> Reply {
        let name = call.str("name").expect("an edit names its definition");
        let text = call.str("text");
        self.base_fits(name, call.str("base"), text.is_some())?;
        let printed = text.map(|t| one_definition(name, t)).transpose()?;
        let (path, content) = self.planned(name, printed.as_deref())?;
        let old = self.workspace.def(name).map(|(t, _)| t.to_owned());
        let failing = self.failing();
        let stored = printed.as_deref().map(definition);
        if let Err(d) = self.workspace.edit(name, stored) {
            let message = format!("{}; nothing was stored", d.message);
            return Err(Failure::new("conflict", message));
        }
        if let Err(e) = self.write(&path, content) {
            (self.workspace.edit(name, old.as_deref()))
                .expect("the program before the edit checks");
            let message = format!("cannot write {}: {e}; nothing was stored", path.display());
            return Err(Failure::new("io", message));
        }
        Ok(self.edited(name, &failing))
    }

    /// Refuses a `base` that is not the current version of `name`, or null when `name` exists.
    fn base_fits(&self, name: &str, base: Option<&str>, adds: bool) -> Result<(), Failure> {
        let current = self.workspace.def(name).map(|(t, _)| canonical(t));
        match (base, current) {
            (None, None) if adds => Ok(()),
            (_, None) => Err(Failure::new("not_found", format!("no definition `{name}`"))),
            (None, Some(_)) => {
                let message =
                    format!("{name} already exists; send its version as base to replace it");
                Err(Failure::new("name_taken", message))
            }
            (Some(base), Some(current)) if base != version(&current) => {
                let mut stale = Failure::new("stale_base", format!("{name} has moved"));
                stale.extra = vec![
                    ("version", version(&current).into()),
                    ("text", current.into()),
                ];
                Err(stale)
            }
            _ => Ok(()),
        }
    }

    /// The reply to an edit of `name`, given the definitions `failing` before it.
    fn edited(&self, name: &str, failing: &BTreeSet<String>) -> Vec<(&'static str, Value)> {
        let now = self.failing();
        let broke: Vec<_> = (now.difference(failing))
            .filter(|n| *n != name)
            .cloned()
            .collect();
        let (version, text, errors) = match self.workspace.def(name) {
            Some((text, check)) => {
                let error = check.error.map(|d| d.value(text.as_bytes(), None));
                (Some(version(text)), Some(text), error.into_iter().collect())
            }
            None => (None, None, vec![]),
        };
        vec![
            ("name", name.into()),
            ("version", version.into()),
            ("text", text.into()),
            ("errors", errors.into()),
            ("broke", broke.into()),
            ("installed", now.is_empty().into()),
        ]
    }

    /// The names of the definitions with an error.
    fn failing(&self) -> BTreeSet<String> {
        (self.workspace.checks().into_iter())
            .filter(|c| c.error.is_some())
            .map(|c| c.name)
            .collect()
    }

    /// The file that changes when `name` takes `printed`, and its new text; `None` removes it.
    fn planned(
        &self,
        name: &str,
        printed: Option<&str>,
    ) -> Result<(PathBuf, Option<String>), Failure> {
        if let Some((file, range)) = self.find(name) {
            let text = &file.text;
            let new = match printed {
                Some(printed) => [&text[..range.start], printed, &text[range.end..]].concat(),
                None => removed(text, range),
            };
            let emptied = split(PATH, new.as_bytes()).is_ok_and(|defs| defs.is_empty());
            let keep = self.dir.is_none() || !emptied;
            return Ok((file.path.clone(), keep.then_some(new)));
        }
        let path = match &self.dir {
            None => self.files[0].path.clone(),
            Some(dir) => {
                let file = format!("{name}.paka");
                if file.len() > 255 {
                    let message = format!("the file name {file} passes 255 bytes");
                    return Err(Failure::new("bad_request", message));
                }
                dir.join(file)
            }
        };
        let old = self.files.iter().find(|f| f.path == path);
        let added = format!("{}\n", printed.expect("an add has text"));
        Ok((
            path,
            Some(joined(old.map_or("", |f| f.text.trim_end()), &added)),
        ))
    }

    /// The file that holds the definition `name`, and its bytes there.
    fn find(&self, name: &str) -> Option<(&File, Range<usize>)> {
        self.files.iter().find_map(|f| {
            let defs = split(PATH, f.text.as_bytes()).ok()?;
            let (_, def) = defs.into_iter().find(|(id, _)| *id == name)?;
            let start = def.as_ptr() as usize - f.text.as_ptr() as usize;
            Some((f, start..start + def.len()))
        })
    }

    /// Replaces the file at `path` by `text` through a copy renamed over it, or removes it.
    fn write(&mut self, path: &Path, text: Option<String>) -> io::Result<()> {
        let at = self.files.iter().position(|f| f.path == path);
        let Some(text) = text else {
            fs::remove_file(path)?;
            self.files.remove(at.expect("a removed file was read"));
            return Ok(());
        };
        let mut copy = path.as_os_str().to_owned();
        copy.push(".tmp");
        let mut file = fs::File::create(&copy)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        if self.dir.is_none() {
            file.lock()?;
        }
        fs::rename(&copy, path)?;
        if self.dir.is_none() {
            self.lock = file;
        }
        match at {
            Some(i) => self.files[i].text = text,
            None => self.files.push(File {
                path: path.into(),
                text,
            }),
        }
        Ok(())
    }
}

/// The canonical text of `text`, which holds the one definition `name`; or why it is refused.
fn one_definition(name: &str, text: &str) -> Result<String, Failure> {
    let unparsed = |d: Diagnostic| {
        let mut failure = Failure::new("parse", "the text does not parse; nothing was stored");
        failure.extra = vec![("errors", vec![d.value(text.as_bytes(), None)].into())];
        failure
    };
    let defs = split(PATH, text.as_bytes()).map_err(unparsed)?;
    let refused = |kind, message| Err(Failure::new(kind, message));
    match defs[..] {
        [(id, _)] if id == name => {}
        [(id, _)] => {
            return refused(
                "name_mismatch",
                format!("the text defines {id}, not {name}"),
            );
        }
        [] => {
            return refused(
                "not_one_definition",
                "the text holds no definition; an edit holds exactly one".into(),
            );
        }
        _ => {
            let message = format!(
                "the text holds {} definitions; an edit holds exactly one",
                defs.len()
            );
            return refused("not_one_definition", message);
        }
    }
    let printed = print::program(PATH, text.as_bytes()).map_err(unparsed)?;
    Ok(printed.trim_end().into())
}

/// The definition in a canonical text, from its first token to its last.
fn definition(printed: &str) -> &str {
    let defs = split(PATH, printed.as_bytes()).expect("canonical text parses");
    std::str::from_utf8(defs[0].1).expect("a slice of a str")
}

fn canonical(text: &str) -> String {
    let printed = print::program(PATH, text.as_bytes()).expect("a stored definition prints");
    definition(&printed).into()
}

fn version(text: &str) -> String {
    let hash = sha256(text.as_bytes());
    hash[..6].iter().map(|b| format!("{b:02x}")).collect()
}

/// `text` without the definition at `range`, the comment lines right above it and the rest of its
/// last line.
fn removed(text: &str, range: Range<usize>) -> String {
    let mut start = line_start(text, range.start);
    while start > 0 {
        let above = line_start(text, start - 1);
        if !text[above..start].trim_start().starts_with("//") {
            break;
        }
        start = above;
    }
    let end = text[range.end..]
        .find('\n')
        .map_or(text.len(), |i| range.end + i + 1);
    let after = &text[end..];
    let blank = after.len() - after.trim_start().len();
    let after = &after[after[..blank].rfind('\n').map_or(0, |i| i + 1)..];
    joined(text[..start].trim_end(), after)
}

fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// `before` and `after` with one blank line between when both hold text.
fn joined(before: &str, after: &str) -> String {
    match (before.is_empty(), after.is_empty()) {
        (true, _) => after.into(),
        (false, true) => format!("{before}\n"),
        (false, false) => format!("{before}\n\n{after}"),
    }
}
