//! The approval store of docs/decisions/M7-01.md: the file `approvals` holds a serial and the
//! approved text of every record, `approvals.sig` signs it whole, `approvals.pub` is the key.
//! Signing and checking run the system's ssh-keygen by a fixed path with an empty environment.

use crate::ast::{Def, Entry};
use crate::diagnostic::Diagnostic;
use crate::parser::{parse_def, split};
use crate::print;
use crate::source::Source;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs, mem, process};

const SSH_KEYGEN: &str = "/usr/bin/ssh-keygen";
const NAMESPACE: &str = "paka-approval";
const SIGNER: &str = "owner";

#[derive(Debug, PartialEq)]
pub struct Store {
    pub serial: u64,
    pub records: Vec<Record>,
}

#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
pub struct Foreign {
    pub text: String,
    pub files: Vec<(String, String)>,
}

/// A record the source no longer matches, and what the source holds now; `None` when its
/// example or app is gone.
#[derive(Debug, PartialEq)]
pub struct Change<'a> {
    pub approved: &'a Record,
    pub now: Option<Record>,
}

impl Record {
    /// Whether both are of one kind and name; the program is compared by the caller.
    fn names(&self, other: &Record) -> bool {
        mem::discriminant(self) == mem::discriminant(other) && self.name() == other.name()
    }

    fn program(&self) -> Option<&str> {
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
    /// matches.
    pub fn changes(&self, program: &str, source: &Source) -> Result<Vec<Change<'_>>, Diagnostic> {
        let mut now = records(program, source)?;
        let approved = self.records.iter().filter(|r| r.program() == Some(program));
        Ok(approved
            .filter_map(|r| {
                let at = now.iter().position(|n| n.names(r));
                let now = at.map(|i| now.swap_remove(i));
                (now.as_ref() != Some(r)).then_some(Change { approved: r, now })
            })
            .collect())
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
    let mut records = vec![];
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
                    let foreign = vec![];
                    records.push(Record::Grants {
                        program,
                        app: name,
                        grants,
                        foreign,
                    });
                }
                _ => {}
            }
        }
    }
    Ok(records)
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
    let program = fs::canonicalize(program).ok()?;
    let start = if program.is_dir() {
        &program
    } else {
        program.parent()?
    };
    let dir = start.ancestors().find(|d| d.join("approvals").is_file())?;
    let relative = program.strip_prefix(dir).ok()?.to_str()?.to_string();
    Some((dir.into(), relative))
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
