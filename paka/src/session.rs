//! The program a server holds (M4-17): its definitions in a workspace and its files on disk, kept
//! equal after each edit (docs/decisions/M4-03.md section 3.3, M4-48).
//!
//! The workspace holds each definition in canonical text, and its version is the first 12 hex
//! digits of the SHA-256 of that text. The files keep their layout except where an edit writes.

use crate::approvals::{self, Store};
use crate::ast::{Def, Entry};
use crate::core_form::Program;
use crate::diagnostic::Diagnostic;
use crate::parser::{parse_def, split};
use crate::pending::{self, Item};
use crate::print;
use crate::serve::{Call, Failure, Reply};
use crate::sha256::sha256;
use crate::source::Source;
use crate::workspace::Workspace;
use paka_json::Value;
use paka_rt::route::Event;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod find;
mod look;
mod runner;
mod stored;
pub mod visual;

pub use runner::Runner;
use stored::Stored;
use visual::{Carry, Shown, Visual};

const PATH: &str = "main.paka";

/// A run's time limit in milliseconds when the call gives none; a migrate's too.
const LIMIT_MS: usize = 10_000;

/// The definitions of the program `text` in canonical text; or its first error, placed in `text`.
pub fn workspace(text: &[u8]) -> Result<Workspace, Diagnostic> {
    let printed = print::program(PATH, text)?;
    Workspace::new(printed.as_bytes()).or_else(|_| Workspace::new(text))
}

pub struct Session {
    /// The program's directory; `None` for a single file.
    dir: Option<PathBuf>,
    files: Vec<File>,
    /// The file of each definition by name.
    owners: BTreeMap<String, PathBuf>,
    workspace: Workspace,
    /// Held while the server runs. For a single file it moves to each new copy.
    lock: fs::File,
    runner: Runner,
    /// The last version in which every definition checks: its generation, files and apps.
    runnable: Option<Runnable>,
    /// The visual app running on the runnable version with no window, or whether the runner
    /// shows it in one; and the state choice waiting for its swap.
    visual: Option<Visual>,
    windowed: bool,
    choice: Option<String>,
}

/// What an install did: took the program, telling what became of the running app's state; held
/// it back; or held it back for a waiting migrate whose check failed, with the error, or that
/// trapped, with the report.
enum Outcome {
    Installed(&'static str),
    Held,
    Wrong(Value),
    Trapped(String),
}

struct Runnable {
    generation: u64,
    files: Vec<(String, String)>,
    apps: Vec<String>,
    stored: Stored,
    /// Whether its app has a view.
    visual: bool,
}

struct File {
    path: PathBuf,
    text: String,
}

impl Session {
    /// The program at `path`, read into `source` and `workspace`, with its `lock` held; its runs
    /// go to `runner`.
    pub fn new(
        path: &OsStr,
        source: &Source,
        workspace: Workspace,
        lock: fs::File,
        runner: Runner,
    ) -> Session {
        let files = (source.files.iter())
            .map(|f| File {
                path: PathBuf::from(&f.path),
                text: String::from_utf8_lossy(&source.text[f.bytes.clone()]).into(),
            })
            .collect();
        let dir = Path::new(path).is_dir().then(|| path.into());
        let mut session = Session {
            dir,
            files,
            owners: BTreeMap::new(),
            workspace,
            lock,
            runner,
            runnable: None,
            visual: None,
            windowed: false,
            choice: None,
        };
        for f in &session.files {
            session.owners.extend(owned(f));
        }
        session.installed(Duration::from_millis(LIMIT_MS as u64));
        session
    }

    pub fn handle(&mut self, call: &Call) -> Reply {
        match call.name {
            "edit" => self.edit(call),
            "find" => self.find(call),
            "view" => self.view(call),
            "run" => {
                let order = order(|key| call.get(key));
                Ok(vec![("run", self.run(&order)?)])
            }
            name if name.starts_with("look_") => self.look(call),
            name => Err(Failure::new(
                "unknown_call",
                format!("{name} is not served yet"),
            )),
        }
    }

    fn edit(&mut self, call: &Call) -> Reply {
        let name = call.str("name").expect("an edit names its definition");
        let text = call.str("text");
        let carried = call.str("state");
        if carried.is_some_and(|c| c != "reset" && !c.starts_with("migrate")) {
            let message = "`state` must be `reset` or a migrate";
            return Err(Failure::new("bad_request", message));
        }
        self.base_fits(name, call.str("base"), text.is_some())?;
        if self.windowed && !self.runner.shows() {
            (self.windowed, self.choice) = (false, None);
        }
        let order = (call.get("run")).map(|run| order(|key| run.get(key)));
        if let Some(app) = order.as_ref().and_then(|o| o.get("app")?.as_str()) {
            let apps = self.apps();
            if app != name && !apps.iter().any(|a| a == app) {
                return Err(no_app(app, &apps));
            }
        }
        let printed = text.map(|t| one_definition(name, t)).transpose()?;
        let (path, content) = self.planned(name, printed.as_deref())?;
        let old = self.workspace.def(name).map(|(t, _)| t.to_owned());
        let failing = self.failing();
        let stored = printed.as_deref().map(definition);
        if let Err(d) = self.workspace.edit(name, stored) {
            let message = format!("{}; nothing was stored", d.message);
            return Err(Failure::new("conflict", message));
        }
        if let Err(refused) = self.state_fits(carried.is_some()) {
            (self.workspace.edit(name, old.as_deref()))
                .expect("the program before the edit checks");
            return Err(refused);
        }
        if let Err(e) = self.write(&path, content) {
            (self.workspace.edit(name, old.as_deref()))
                .expect("the program before the edit checks");
            let message = format!("cannot write {}: {e}; nothing was stored", path.display());
            return Err(Failure::new("io", message));
        }
        if let Some(c) = carried.filter(|_| self.running()) {
            self.choice = Some(c.into());
        }
        let limit =
            (order.as_ref().and_then(|o| o.get("limit_ms")?.as_u64())).unwrap_or(LIMIT_MS as u64);
        let outcome = self.installed(Duration::from_millis(limit));
        let installed = matches!(outcome, Outcome::Installed(_));
        let mut reply = self.edited(name, &failing, &outcome);
        if let Some(state) = self.state_kept(carried.is_some(), &outcome) {
            reply.push(("state", state.into()));
        }
        if let Outcome::Trapped(report) = outcome {
            reply.push(("trap", report.into()));
        }
        if installed && let Some(order) = order {
            reply.push(("run", self.run(&order).unwrap_or(Value::Null)));
        } else if order.is_some() {
            reply.push(("run", Value::Null));
        }
        let pending = self.pending();
        if !pending.is_empty() {
            reply.extend(pending::fields(&pending));
        }
        Ok(reply)
    }

    /// What awaits the owner in this program: each record of the store its source no longer
    /// matches, the store read without checking its signature (M7-01). None when not gated.
    fn pending(&self) -> Vec<Item> {
        let at = self.dir.as_deref().unwrap_or(&self.files[0].path);
        let Some((dir, program)) = approvals::find(at) else {
            return vec![];
        };
        let store = (fs::read_to_string(dir.join("approvals")))
            .map_err(|e| format!("approvals: {e}"))
            .and_then(|text| Store::parse(&text));
        let store = match store {
            Ok(store) if store.programs().contains(&program.as_str()) => store,
            Ok(_) => return vec![],
            Err(why) => return vec![Item::Refused(why)],
        };
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_key(|f| &f.path);
        let mut source = Source::default();
        for f in files {
            source.add(f.path.clone().into(), f.text.clone().into_bytes());
        }
        match store.changes(&program, &source) {
            Ok(changes) => changes.into_iter().map(Item::Change).collect(),
            Err(d) => vec![Item::Refused(format!("program {program}: {}", d.message))],
        }
    }

    /// Launches the runnable version's visual app with no window and gives its first tree. While
    /// it runs, edits to its stored types need a state choice (docs/decisions/M5-07.md).
    pub fn launch(&mut self, args: Vec<OsString>) -> Result<String, Failure> {
        Ok(self.started(args)?.tree)
    }

    fn started(&mut self, args: Vec<OsString>) -> Result<Shown, Failure> {
        let core = core(self.runnable.as_ref().ok_or_else(not_runnable)?);
        if !visual_app(&core) {
            return Err(Failure::new("no_app", "the program has no visual app"));
        }
        let (visual, first) = Visual::start(core, args, false);
        self.visual = first.end.is_none().then_some(visual);
        self.choice = None;
        Ok(first)
    }

    /// Sends `events` to the running visual app and gives its tree after them; `None` when no app
    /// runs.
    pub fn send(&mut self, events: Vec<Event>) -> Option<String> {
        let shown = self.visual.as_ref()?.send(events);
        Some(self.after(shown).tree)
    }

    /// `shown`, the app dropped if it ended; a waiting state choice ends with it.
    fn after(&mut self, shown: Shown) -> Shown {
        if shown.end.is_some() {
            (self.visual, self.choice) = (None, None);
        }
        shown
    }

    /// Refuses an edit that changes the running app's stored types with no state choice carried
    /// or waiting.
    fn state_fits(&self, carried: bool) -> Result<(), Failure> {
        let (true, Some(r), false) = (self.running(), &self.runnable, carried) else {
            return Ok(());
        };
        let changes = r.stored.changes(&Stored::of(&self.workspace));
        if self.choice.is_some() || changes.is_empty() {
            return Ok(());
        }
        let message = format!(
            "the running app's stored types change: {}; send `state` with `reset` or a migrate; \
             nothing was stored",
            changes.join("; ")
        );
        let mut refused = Failure::new("state_changed", message);
        refused.extra = vec![("changed", changes.into())];
        Err(refused)
    }

    /// What became of the running app's state, given whether a choice was `carried` and the
    /// install's `outcome`; with no app running, only that a carried choice was ignored.
    fn state_kept(&self, carried: bool, outcome: &Outcome) -> Option<&'static str> {
        if !self.running() {
            return carried.then_some("ignored");
        }
        Some(match outcome {
            Outcome::Installed(took) => took,
            Outcome::Trapped(_) => "migrate_trapped",
            _ if self.choice.is_some() => "waiting",
            _ if !self.failing().is_empty() => "kept",
            _ => "missing",
        })
    }

    /// Takes the program as the runnable version when every definition checks, and swaps it into
    /// the running visual app, with no window or in the runner's. There a waiting choice is
    /// applied and used up: a reset, or a migrate, checked against the running and the new version
    /// and run on the state; a version with other stored types and no choice is held back, as is
    /// one whose migrate fails its check or traps, or does not finish within `limit`
    /// (docs/decisions/M5-07.md section 1.6).
    fn installed(&mut self, limit: Duration) -> Outcome {
        if !self.failing().is_empty() {
            return Outcome::Held;
        }
        let stored = Stored::of(&self.workspace);
        let changed = self.runnable.as_ref().is_some_and(|r| r.stored != stored);
        let files = (self.files.iter())
            .map(|f| (f.path.to_string_lossy().into_owned(), f.text.clone()))
            .collect();
        let runnable = Runnable {
            generation: self.runnable.as_ref().map_or(0, |r| r.generation + 1),
            files,
            apps: self.apps(),
            stored,
            visual: has_view(&self.workspace),
        };
        let mut took = "kept";
        if self.running()
            && let Some(running) = &self.runnable
        {
            if changed && self.choice.is_none() {
                return Outcome::Held;
            }
            let carry = Carry::of(self.choice.as_deref());
            let shown = match &self.visual {
                Some(visual) => visual.install(&running.files, &runnable.files, &carry, limit),
                None => (self.runner).install(&runnable.files, self.choice.as_deref(), limit),
            };
            let shown = match shown {
                Ok(shown) => shown,
                Err(error) => return Outcome::Wrong(error),
            };
            if let Some(trap) = shown.trap {
                return Outcome::Trapped(trap);
            }
            took = carry.took();
            self.choice = None;
            if shown.end.is_some() {
                (self.visual, self.windowed) = (None, false);
            }
        }
        self.runnable = Some(runnable);
        Outcome::Installed(took)
    }

    /// The names of the program's apps.
    fn apps(&self) -> Vec<String> {
        (self.workspace.defs())
            .filter(|(_, text)| text.starts_with("app "))
            .map(|(names, _)| names[0].clone())
            .collect()
    }

    /// The runnable version's run that `order` asks for (M4-03 section 3.4).
    fn run(&mut self, order: &Value) -> Result<Value, Failure> {
        let r = self.runnable.as_ref().ok_or_else(not_runnable)?;
        let app = order.get("app").and_then(Value::as_str).unwrap();
        if !r.apps.iter().any(|a| a == app) {
            return Err(no_app(app, &r.apps));
        }
        // With a display the runner opens the app's window (docs/decisions/M5-09.md).
        if r.visual && env::var_os("WAYLAND_DISPLAY").is_none() {
            return self.run_visual(app, order);
        }
        self.visual = None;
        let run = match self.runner.run((r.generation, &r.files), order)? {
            Value::Null => return Err(no_app(app, &r.apps)),
            run => run,
        };
        let shown = run.get("window") == Some(&true.into());
        (self.windowed, self.choice) = (shown && run.get("status") == Some(&Value::Null), None);
        Ok(run)
    }

    /// Whether a visual app runs, with no window or in the runner's.
    fn running(&self) -> bool {
        self.visual.is_some() || self.windowed
    }

    /// Refuses a `base` that is not the current version of `name`, or null when `name` exists.
    fn base_fits(&self, name: &str, base: Option<&str>, adds: bool) -> Result<(), Failure> {
        match (base, self.workspace.def(name).map(|(t, _)| t)) {
            (None, None) if adds => Ok(()),
            (_, None) => Err(Failure::new("not_found", format!("no definition `{name}`"))),
            (None, Some(_)) => {
                let message =
                    format!("{name} already exists; send its version as base to replace it");
                Err(Failure::new("name_taken", message))
            }
            (Some(base), Some(current)) if base != version(current) => {
                let mut stale = Failure::new("stale_base", format!("{name} has moved"));
                stale.extra = vec![
                    ("version", version(current).into()),
                    ("text", current.into()),
                ];
                Err(stale)
            }
            _ => Ok(()),
        }
    }

    /// The reply to an edit of `name`, given the definitions `failing` before it and the
    /// install's `outcome`. A waiting migrate that fails its check is among what the edit broke.
    fn edited(
        &self,
        name: &str,
        failing: &BTreeSet<String>,
        outcome: &Outcome,
    ) -> Vec<(&'static str, Value)> {
        let now = self.failing();
        let mut broke: Vec<_> = (now.difference(failing))
            .filter(|n| *n != name)
            .cloned()
            .collect();
        let (version, text, mut errors) = match self.workspace.def(name) {
            Some((text, check)) => {
                let error = check.error.map(|d| d.value(text.as_bytes(), None));
                (Some(version(text)), Some(text), error.into_iter().collect())
            }
            None => (None, None, vec![]),
        };
        if let Outcome::Wrong(error) = outcome {
            broke.push("migrate".into());
            errors.push(error.clone());
        }
        let installed = matches!(outcome, Outcome::Installed(_));
        vec![
            ("name", name.into()),
            ("version", version.into()),
            ("text", text.into()),
            ("errors", errors.into()),
            ("broke", broke.into()),
            ("installed", installed.into()),
        ]
    }

    fn failing(&self) -> BTreeSet<String> {
        self.workspace.failing()
    }

    /// The file that changes when `name` takes `printed`, and its new text; `None` removes it.
    fn planned(
        &self,
        name: &str,
        printed: Option<&str>,
    ) -> Result<(PathBuf, Option<String>), Failure> {
        if let Some((file, range)) = self.located(name) {
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
    fn located(&self, name: &str) -> Option<(&File, Range<usize>)> {
        let path = self.owners.get(name)?;
        let f = self.files.iter().find(|f| f.path == *path)?;
        let defs = split(PATH, f.text.as_bytes()).ok()?;
        let (_, def) = defs.into_iter().find(|(id, _)| *id == name)?;
        let start = def.as_ptr() as usize - f.text.as_ptr() as usize;
        Some((f, start..start + def.len()))
    }

    /// Replaces the file at `path` by `text` through a copy renamed over it, or removes it.
    fn write(&mut self, path: &Path, text: Option<String>) -> io::Result<()> {
        let at = self.files.iter().position(|f| f.path == path);
        let Some(text) = text else {
            fs::remove_file(path)?;
            self.files.remove(at.expect("a removed file was read"));
            self.owners.retain(|_, p| p != path);
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
        let i = at.unwrap_or_else(|| {
            self.files.push(File {
                path: path.into(),
                text: String::new(),
            });
            self.files.len() - 1
        });
        self.files[i].text = text;
        self.owners.retain(|_, p| p != path);
        self.owners.extend(owned(&self.files[i]));
        Ok(())
    }
}

/// Each definition of `f` by name, with its path.
fn owned(f: &File) -> Vec<(String, PathBuf)> {
    let defs = split(PATH, f.text.as_bytes()).unwrap_or_default();
    (defs.into_iter())
        .map(|(name, _)| (name.to_string(), f.path.clone()))
        .collect()
}

/// The runner's request for a run with the fields `get` gives, defaults filled in.
fn order<'a>(get: impl Fn(&str) -> Option<&'a Value>) -> Value {
    let or = |key, default: Value| (key, get(key).cloned().unwrap_or(default));
    Value::object([
        or("app", Value::Null),
        or("args", Value::Array(vec![])),
        or("limit_ms", LIMIT_MS.into()),
        or("max_bytes", 16_000usize.into()),
    ])
}

fn not_runnable() -> Failure {
    let message = "no version of this program has checked yet";
    Failure::new("not_runnable", message)
}

fn no_app(app: &str, apps: &[String]) -> Failure {
    let message = format!("no app `{app}`; apps are {}", apps.join(", "));
    Failure::new("no_app", message)
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

/// The checked program of version `r`.
fn core(r: &Runnable) -> Program {
    crate::check(PATH, &source(&r.files))
        .expect("the runnable version checks")
        .0
}

fn source(files: &[(String, String)]) -> Source {
    let mut source = Source::default();
    for (path, text) in files {
        source.add(path.into(), text.as_bytes().to_vec());
    }
    source
}

fn visual_app(core: &Program) -> bool {
    core.app.as_ref().is_some_and(|a| a.view.is_some())
}

/// Whether the app of `workspace` has a view, read from its text.
fn has_view(workspace: &Workspace) -> bool {
    let mut apps = workspace
        .defs()
        .filter(|(_, text)| text.starts_with("app "));
    apps.any(|(_, text)| match parse_def(PATH, text.as_bytes()) {
        Ok(Def::App { entries, .. }) => entries.iter().any(|e| matches!(e, Entry::View(..))),
        _ => false,
    })
}
