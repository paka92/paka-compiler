//! The approval store of docs/decisions/M7-01.md: the file `approvals` holds a serial and the
//! approved text of every record, `approvals.sig` signs it whole, `approvals.pub` is the key.
//! Signing and checking run the system's ssh-keygen by a fixed path with an empty environment.

use crate::ast::{Def, Entry, ForeignLine};
use crate::diagnostic::Diagnostic;
use crate::hash::hex;
use crate::parser::{parse_def, split};
use crate::print;
use crate::sha256::sha256;
use crate::source::Source;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs, mem, process};

const SSH_KEYGEN: &str = "/usr/bin/ssh-keygen";
const NAMESPACE: &str = "paka-approval";
const SIGNER: &str = "owner";

#[derive(Debug, Default, PartialEq)]
pub struct Store {
    pub serial: u64,
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    /// One app's grant list, and the `foreign` declarations behind its grants (M7-02).
    Grants {
        program: String,
        app: String,
        grants: BTreeSet<String>,
        foreign: Vec<Foreign>,
    },
    /// An example's canonical text.
    Example {
        program: String,
        name: String,
        text: String,
    },
    /// An owned file's text.
    File { path: String, text: String },
}

/// A declaration's text and each file it names with its SHA-256 in hex.
#[derive(Clone, Debug, PartialEq)]
pub struct Foreign {
    pub text: String,
    pub files: Vec<(String, String)>,
}

/// What the store holds and what the source holds now, when they differ; `None` on one side
/// for a record never approved or one whose example, app or file is gone.
#[derive(Debug, PartialEq)]
pub struct Change {
    pub approved: Option<Record>,
    pub now: Option<Record>,
}

impl Record {
    /// Whether both are of one kind and name; the program is compared by the caller.
    fn names(&self, other: &Record) -> bool {
        mem::discriminant(self) == mem::discriminant(other) && self.name() == other.name()
    }

    pub fn program(&self) -> Option<&str> {
        match self {
            Record::Grants { program, .. } | Record::Example { program, .. } => Some(program),
            Record::File { .. } => None,
        }
    }

    fn name(&self) -> &str {
        match self {
            Record::Grants { app, .. } => app,
            Record::Example { name, .. } => name,
            Record::File { path, .. } => path,
        }
    }
}

impl Store {
    /// The store in `dir`, when `approvals.sig` signs `approvals` with the public key at `key`.
    pub fn open(dir: &Path, key: &Path) -> Result<Store, String> {
        let text = fs::read(dir.join("approvals")).map_err(|e| format!("approvals: {e}"))?;
        let key = fs::read_to_string(key).map_err(|e| format!("{}: {e}", key.display()))?;
        let key = key.lines().next().filter(|k| !k.is_empty());
        let signers = Signers::new(key.ok_or("approvals.pub: no key")?)?;
        let mut verify = Command::new(SSH_KEYGEN);
        verify.args(["-Y", "verify", "-I", SIGNER, "-n", NAMESPACE, "-f"]);
        verify
            .arg(&signers.0)
            .arg("-s")
            .arg(dir.join("approvals.sig"));
        if !run(verify, &text)?.status.success() {
            return Err("approvals: the signature does not verify".into());
        }
        Store::parse(&String::from_utf8(text).map_err(|_| "approvals: not UTF-8")?)
    }

    /// Writes the store into `dir` and signs it with the private key at `key`; ssh-keygen asks
    /// for the passphrase itself.
    pub fn save(&self, dir: &Path, key: &Path) -> Result<(), String> {
        let text = self.text();
        let mut sign = Command::new(SSH_KEYGEN);
        sign.args(["-Y", "sign", "-n", NAMESPACE, "-f"]).arg(key);
        let out = run(sign, text.as_bytes())?;
        if !out.status.success() {
            return Err(format!(
                "signing failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let write = |name, bytes: &[u8]| fs::write(dir.join(name), bytes);
        write("approvals", text.as_bytes())
            .and_then(|()| write("approvals.sig", &out.stdout))
            .map_err(|e| format!("approvals: {e}"))
    }

    /// Whether this store may replace `base`: only unchanged or with a higher serial, so an
    /// older signed store cannot be put back.
    pub fn follows(&self, base: &Store) -> bool {
        self == base || self.serial > base.serial
    }

    /// The records of `program`, a path relative to the store, that its source no longer
    /// matches, then each of its apps with no grants record.
    pub fn changes(&self, program: &str, source: &Source) -> Result<Vec<Change>, Diagnostic> {
        let mut now = records(program, source)?;
        let approved = self.records.iter().filter(|r| r.program() == Some(program));
        let mut changes: Vec<_> = approved
            .filter_map(|r| {
                let at = now.iter().position(|n| n.names(r));
                let now = at.map(|i| now.swap_remove(i));
                (now.as_ref() != Some(r)).then(|| Change {
                    approved: Some(r.clone()),
                    now,
                })
            })
            .collect();
        let new = now
            .into_iter()
            .filter(|r| matches!(r, Record::Grants { .. }));
        changes.extend(new.map(|r| Change {
            approved: None,
            now: Some(r),
        }));
        Ok(changes)
    }

    /// Each owned file under `root` that differs from its record, has none or is gone: every
    /// recorded file, every file under `.github/workflows/`, and `TRUSTED` (M7-01, M7-02).
    pub fn file_changes(&self, root: &Path) -> Result<Vec<Change>, String> {
        let mut changes = vec![];
        for path in self.owned_files(root)? {
            let approved = (self.records.iter())
                .find(|r| matches!(r, Record::File { path: p, .. } if *p == path))
                .cloned();
            let now = match fs::read(root.join(&path)) {
                Ok(bytes) => {
                    let text = String::from_utf8(bytes).map_err(|_| format!("{path}: not text"))?;
                    Some(Record::File { path, text })
                }
                Err(e) if e.kind() == ErrorKind::NotFound => None,
                Err(e) => return Err(format!("{path}: {e}")),
            };
            if approved != now {
                changes.push(Change { approved, now });
            }
        }
        Ok(changes)
    }

    /// The owned files: those recorded, every file under `.github/workflows`, and `TRUSTED`.
    pub fn owned_files(&self, root: &Path) -> Result<BTreeSet<String>, String> {
        let mut paths: BTreeSet<_> = (self.records.iter())
            .filter(|r| matches!(r, Record::File { .. }))
            .map(|r| r.name().to_string())
            .collect();
        walk(root, ".github/workflows", &mut paths)?;
        if root.join("TRUSTED").is_file() {
            paths.insert("TRUSTED".into());
        }
        Ok(paths)
    }

    /// Takes `change` in: its approved record gives way to what the source holds now.
    pub fn apply(&mut self, change: Change) {
        let at = (change.approved).and_then(|a| self.records.iter().position(|r| *r == a));
        match (at, change.now) {
            (Some(i), Some(now)) => self.records[i] = now,
            (Some(i), None) => _ = self.records.remove(i),
            (None, Some(now)) => self.records.push(now),
            (None, None) => {}
        }
    }

    /// The programs the store gates, in record order.
    pub fn programs(&self) -> Vec<&str> {
        let mut programs: Vec<&str> = vec![];
        for program in self.records.iter().filter_map(Record::program) {
            if !programs.contains(&program) {
                programs.push(program);
            }
        }
        programs
    }

    /// The names of `program`'s examples the store holds.
    pub fn owned(&self, program: &str) -> BTreeSet<String> {
        (self.records.iter())
            .filter_map(|r| match r {
                Record::Example {
                    program: p, name, ..
                } if p == program => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn text(&self) -> String {
        let mut out = format!("serial {}\n", self.serial);
        for record in &self.records {
            match record {
                Record::Grants {
                    program,
                    app,
                    grants,
                    foreign,
                } => {
                    let grants: Vec<_> = grants.iter().map(String::as_str).collect();
                    out += &format!("grants {program} {app} [{}]\n", grants.join(", "));
                    for f in foreign {
                        block(&mut out, "  foreign", &f.text, "    ");
                        for (file, hash) in &f.files {
                            out += &format!("  sha256 {file} {hash}\n");
                        }
                    }
                }
                Record::Example {
                    program,
                    name,
                    text,
                } => block(&mut out, &format!("example {program} {name}"), text, "  "),
                Record::File { path, text } => block(&mut out, &format!("file {path}"), text, "  "),
            }
        }
        out
    }

    pub fn parse(text: &str) -> Result<Store, String> {
        let lines: Vec<_> = (text.strip_suffix('\n'))
            .ok_or("approvals: no line feed at the end")?
            .split('\n')
            .collect();
        let mut r = Reader { lines, at: 0 };
        let serial = r
            .line()
            .strip_prefix("serial ")
            .and_then(|s| s.parse().ok());
        let serial = serial.ok_or_else(|| r.bad())?;
        let mut records = vec![];
        while r.at < r.lines.len() {
            records.push(r.record()?);
        }
        Ok(Store { serial, records })
    }
}

/// Each line of `text` under `head` and its count, indented; an empty line stays empty.
fn block(out: &mut String, head: &str, text: &str, indent: &str) {
    let lines: Vec<_> = text.split('\n').collect();
    out.push_str(&format!("{head} {}\n", lines.len()));
    for line in lines {
        if !line.is_empty() {
            out.push_str(indent);
        }
        out.push_str(line);
        out.push('\n');
    }
}

struct Reader<'a> {
    lines: Vec<&'a str>,
    at: usize,
}

impl<'a> Reader<'a> {
    fn line(&mut self) -> &'a str {
        self.at += 1;
        self.lines.get(self.at - 1).copied().unwrap_or_default()
    }

    fn bad(&self) -> String {
        format!("approvals:{}: not a record", self.at)
    }

    fn record(&mut self) -> Result<Record, String> {
        let words: Vec<_> = self.line().splitn(4, ' ').collect();
        match words[..] {
            ["grants", program, app, list] => {
                let list = list.strip_prefix('[').and_then(|l| l.strip_suffix(']'));
                let list = list.ok_or_else(|| self.bad())?;
                let grants = list.split(", ").filter(|g| !g.is_empty()).map(String::from);
                let mut foreign = vec![];
                while let Some(count) = self.peek("  foreign ") {
                    self.at += 1;
                    let text = self.block(count, "    ")?;
                    let mut files = vec![];
                    while let Some(file) = self.peek("  sha256 ") {
                        self.at += 1;
                        let (file, hash) = file.split_once(' ').ok_or_else(|| self.bad())?;
                        files.push((file.into(), hash.into()));
                    }
                    foreign.push(Foreign { text, files });
                }
                Ok(Record::Grants {
                    program: program.into(),
                    app: app.into(),
                    grants: grants.collect(),
                    foreign,
                })
            }
            ["example", program, name, count] => Ok(Record::Example {
                program: program.into(),
                name: name.into(),
                text: self.block(count, "  ")?,
            }),
            ["file", path, count] => Ok(Record::File {
                path: path.into(),
                text: self.block(count, "  ")?,
            }),
            _ => Err(self.bad()),
        }
    }

    /// The rest of the next line when it starts with `prefix`.
    fn peek(&self, prefix: &str) -> Option<&'a str> {
        self.lines.get(self.at)?.strip_prefix(prefix)
    }

    fn block(&mut self, count: &str, indent: &str) -> Result<String, String> {
        let count: usize = count.parse().map_err(|_| self.bad())?;
        let lines: Result<Vec<_>, _> = (0..count)
            .map(|_| match self.line() {
                "" if self.at <= self.lines.len() => Ok(""),
                line => line.strip_prefix(indent).ok_or_else(|| self.bad()),
            })
            .collect();
        Ok(lines?.join("\n"))
    }
}

/// The grants record of each app and the example record of each example in `source`, as it
/// stands, for `program`.
pub fn records(program: &str, source: &Source) -> Result<Vec<Record>, Diagnostic> {
    let (mut records, mut declared) = (vec![], BTreeMap::new());
    for file in &source.files {
        let printed = print::program(&file.name, &source.text[file.bytes.clone()])?;
        for (name, text) in split(&file.name, printed.as_bytes())? {
            let (program, name) = (program.to_string(), name.to_string());
            match parse_def(&file.name, text)? {
                Def::Example { .. } => records.push(Record::Example {
                    program,
                    name,
                    text: with_comments(&printed, text).into(),
                }),
                Def::App { entries, .. } => {
                    let grants = entries.iter().flat_map(|e| match e {
                        Entry::Grants(_, names) => names.as_slice(),
                        _ => &[],
                    });
                    let grants = grants.map(|g| g.text.to_string()).collect();
                    records.push(Record::Grants {
                        program,
                        app: name,
                        grants,
                        foreign: vec![],
                    });
                }
                Def::Foreign { lines, .. } => {
                    let dir = Path::new(&file.path).parent().unwrap_or(Path::new(""));
                    let text = String::from_utf8_lossy(text).into();
                    let files = built_files(dir, &lines);
                    declared.insert(name, Foreign { text, files });
                }
                _ => {}
            }
        }
    }
    for record in &mut records {
        if let Record::Grants {
            grants, foreign, ..
        } = record
        {
            *foreign = grants
                .iter()
                .filter_map(|g| declared.get(g).cloned())
                .collect();
        }
    }
    Ok(records)
}

/// Each `source:` and `header:` file in `dir` with its SHA-256, or `none` when it is not a regular
/// file; a library is covered by its name alone.
fn built_files(dir: &Path, lines: &[ForeignLine]) -> Vec<(String, String)> {
    let hash = |name: &str| {
        let path = dir.join(name);
        let regular = fs::symlink_metadata(&path).is_ok_and(|m| m.is_file());
        match fs::read(&path) {
            Ok(bytes) if regular => hex(&sha256(&bytes)),
            _ => "none".into(),
        }
    };
    (lines.iter())
        .filter_map(|line| match line {
            ForeignLine::File(key, file) if key.text != "library" => {
                Some((file.text.to_string(), hash(file.text)))
            }
            _ => None,
        })
        .collect()
}

/// Adds each file under `dir`, a path relative to `root`, to `paths`; none when `dir` is not
/// there.
fn walk(root: &Path, dir: &str, paths: &mut BTreeSet<String>) -> Result<(), String> {
    let failed = |e: io::Error| format!("{dir}: {e}");
    let entries = match fs::read_dir(root.join(dir)) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        entries => entries.map_err(failed)?,
    };
    for entry in entries {
        let entry = entry.map_err(failed)?;
        let name = entry.file_name().into_string();
        let path = format!(
            "{dir}/{}",
            name.map_err(|_| format!("{dir}: a name is not text"))?
        );
        match entry.file_type().map_err(failed)?.is_dir() {
            true => walk(root, &path, paths)?,
            false => _ = paths.insert(path),
        }
    }
    Ok(())
}

/// Definition `def`, a slice of canonical text `printed`, with the comment lines right above it.
fn with_comments<'a>(printed: &'a str, def: &[u8]) -> &'a str {
    let start = def.as_ptr() as usize - printed.as_ptr() as usize;
    let mut from = start;
    for line in printed[..start].split_inclusive('\n').rev() {
        if !line.starts_with("//") {
            break;
        }
        from -= line.len();
    }
    &printed[from..start + def.len()]
}

/// The directory of the store that governs `program`, the nearest at or above it holding
/// `approvals`, and the program's path relative to it.
pub fn find(program: &Path) -> Option<(PathBuf, String)> {
    nearest(program, "approvals")
}

/// The nearest directory at or above `program` holding the file `name`, and the program's path
/// relative to it.
pub fn nearest(program: &Path, name: &str) -> Option<(PathBuf, String)> {
    let program = fs::canonicalize(program).ok()?;
    let start = if program.is_dir() {
        &program
    } else {
        program.parent()?
    };
    let dir = start.ancestors().find(|d| d.join(name).is_file())?;
    let relative = program.strip_prefix(dir).ok()?.to_str()?.to_string();
    Some((dir.into(), relative))
}

/// The store `paka approve` starts from for `program`, its directory and the program's path
/// relative to it: the one found by the walk-up rule, verified against the `approvals.pub` beside
/// it; with none, an empty store beside the nearest `approvals.pub`, the first one.
pub fn to_approve(program: &Path) -> Result<(PathBuf, String, Store), String> {
    if let Some((dir, program)) = find(program) {
        let store = Store::open(&dir, &dir.join("approvals.pub"))?;
        return Ok((dir, program, store));
    }
    let found = nearest(program, "approvals.pub");
    let (dir, program) = found.ok_or("no approvals or approvals.pub at or above the program")?;
    Ok((dir, program, Store::default()))
}

/// The names of the human-owned examples of `program`: none without a store; an error when its
/// store does not verify against the `approvals.pub` beside it.
pub fn owned(program: &Path) -> Result<BTreeSet<String>, String> {
    let Some((dir, program)) = find(program) else {
        return Ok(BTreeSet::new());
    };
    Ok(Store::open(&dir, &dir.join("approvals.pub"))?.owned(&program))
}

/// Runs ssh-keygen with no environment, `input` on its standard input.
fn run(mut command: Command, input: &[u8]) -> Result<Output, String> {
    command.env_clear().stdin(Stdio::piped());
    let mut child = (command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn())
    .map_err(|e| format!("{SSH_KEYGEN}: {e}"))?;
    let written = child.stdin.take().expect("piped").write_all(input);
    let out = child
        .wait_with_output()
        .map_err(|e| format!("{SSH_KEYGEN}: {e}"))?;
    written.map_err(|e| format!("{SSH_KEYGEN}: {e}"))?;
    Ok(out)
}

/// The allowed-signers file ssh-keygen verifies with: the fixed signer and one key. It is
/// removed on drop.
struct Signers(PathBuf);

impl Signers {
    fn new(key: &str) -> Result<Signers, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("paka-signers-{}-{n}", process::id()));
        let failed = |e| format!("{}: {e}", path.display());
        let mut file = fs::File::create_new(&path).map_err(failed)?;
        let written = writeln!(file, "{SIGNER} {key}").map_err(failed);
        let signers = Signers(path);
        written.map(|()| signers)
    }
}

impl Drop for Signers {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
