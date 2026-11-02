//! The trusted list `TRUSTED` (docs/decisions/M7-02.md 1.7) and the third-party notices file made
//! from it and the licence texts in `vendor/` (M7-16).

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

/// One library or file of `TRUSTED`.
pub struct Row<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub licence: &'a str,
    pub runs: &'a str,
    pub purpose: &'a str,
}

impl Row<'_> {
    /// A library taken from the system, never vendored or shipped.
    pub fn is_system(&self) -> bool {
        self.version == "system"
    }

    /// A file in the repository, named by its path.
    pub fn is_file(&self) -> bool {
        self.name.contains('/')
    }

    /// The directory holding the licence texts: the file's own, or the crate's in `vendor/`.
    fn dir(&self, root: &Path) -> PathBuf {
        if self.is_file() {
            return root.join(self.name).parent().unwrap().into();
        }
        let versioned = root.join(format!("vendor/{}-{}", self.name, self.version));
        match versioned.is_dir() {
            true => versioned,
            false => root.join("vendor").join(self.name),
        }
    }
}

/// The rows of a `TRUSTED` text, its blank and `#` lines left out: fields two or more spaces apart.
pub fn rows(text: &str) -> Result<Vec<Row<'_>>, String> {
    let mut rows = vec![];
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = (line.split("  ").map(str::trim))
            .filter(|f| !f.is_empty())
            .collect();
        let [name, version, licence, runs, purpose] = fields[..] else {
            let shape = "a name, version, licence, runs in and purpose two spaces apart";
            return Err(format!("TRUSTED line {}: not {shape}", n + 1));
        };
        if runs != "programs" && runs != "toolchain" {
            let why = format!("runs in is programs or toolchain, not {runs}");
            return Err(format!("TRUSTED line {}: {why}", n + 1));
        }
        rows.push(Row {
            name,
            version,
            licence,
            runs,
            purpose,
        });
    }
    Ok(rows)
}

/// The notices file for the `TRUSTED` in `root`: each row programs run, in order, with the
/// licence texts beside it.
pub fn notices(root: &Path) -> Result<String, String> {
    let text = fs::read_to_string(root.join("TRUSTED")).map_err(|e| format!("TRUSTED: {e}"))?;
    let mut out = String::from(
        "Third-party notices: each library and file in TRUSTED that a program contains or loads, \
         in that\norder, with the licence texts its package ships.\n",
    );
    for row in rows(&text)?.iter().filter(|r| r.runs == "programs") {
        let (name, version) = (row.name, row.version);
        write!(out, "\n==== {name} {version}\n").unwrap();
        writeln!(out, "licence: {}\npurpose: {}", row.licence, row.purpose).unwrap();
        if row.is_system() {
            out += "(from the system, not shipped)\n";
            continue;
        }
        let texts = licences(root, &row.dir(root))?;
        if texts.is_empty() {
            out += "(no licence text in its package)\n";
        }
        for (file, text) in texts {
            writeln!(out, "---- {file}\n{}", text.trim_end_matches('\n')).unwrap();
        }
    }
    Ok(out)
}

/// The licence files directly in `dir`, by name in byte order, and their texts.
fn licences(root: &Path, dir: &Path) -> Result<Vec<(String, String)>, String> {
    let shown = dir.strip_prefix(root).unwrap_or(dir).display();
    let failed = |e: std::io::Error| format!("{shown}: {e}");
    let mut texts = vec![];
    for entry in fs::read_dir(dir).map_err(failed)? {
        let path = entry.map_err(failed)?.path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_file() && is_licence(&name) {
            texts.push((name, fs::read_to_string(&path).map_err(failed)?));
        }
    }
    texts.sort();
    Ok(texts)
}

fn is_licence(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    let starts = [
        "LICENSE",
        "LICENCE",
        "COPYING",
        "COPYRIGHT",
        "NOTICE",
        "UNLICENSE",
        "OFL",
    ];
    starts.iter().any(|s| name.starts_with(s))
}
