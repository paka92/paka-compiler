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
    pub fn dir(&self, root: &Path) -> PathBuf {
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

/// Standard texts for licences a package names but ships no text of, from a vendored crate that
/// ships them verbatim.
const STANDARD: [(&str, &str); 2] = [
    ("Apache-2.0", "vendor/ahash/LICENSE-APACHE"),
    ("MIT", "vendor/ahash/LICENSE-MIT"),
];

/// The notices file for the `TRUSTED` in `root`: each row programs run, in order, then each
/// distinct licence text once, after the rows it covers, in order of first use.
pub fn notices(root: &Path) -> Result<String, String> {
    let text = fs::read_to_string(root.join("TRUSTED")).map_err(|e| format!("TRUSTED: {e}"))?;
    let rows = rows(&text)?;
    let mut out = String::from(
        "Third-party notices: each library and file in TRUSTED that a program contains or loads, \
         in that\norder, then each distinct licence text once, after the list of those it covers.\n\n",
    );
    let mut texts: Vec<(String, Vec<String>)> = vec![];
    for row in rows.iter().filter(|r| r.runs == "programs") {
        write!(
            out,
            "{} {}  {}  {}",
            row.name, row.version, row.licence, row.purpose
        )
        .unwrap();
        if row.is_system() {
            out += "  (from the system, not shipped)\n";
            continue;
        }
        out += "\n";
        for (user, body) in row_texts(root, row)? {
            match texts.iter_mut().find(|(b, _)| *b == body) {
                Some((_, users)) => users.push(user),
                None => texts.push((body, vec![user])),
            }
        }
    }
    for (n, (body, users)) in texts.iter().enumerate() {
        let users = users.join("\n");
        write!(out, "\n==== text {}, for:\n{users}\n----\n{body}\n", n + 1).unwrap();
    }
    Ok(out)
}

/// A row's licence texts, each with the line naming whose and where from: the package's own, or
/// the standard text of each licence it names when it ships none.
fn row_texts(root: &Path, row: &Row) -> Result<Vec<(String, String)>, String> {
    let id = format!("{} {}", row.name, row.version);
    let dir = row.dir(root);
    let own = licences(root, &dir)?;
    if !own.is_empty() {
        let line = |(file, text): (String, String)| (format!("{id}  {file}"), trimmed(&text));
        return Ok(own.into_iter().map(line).collect());
    }
    let words = row
        .licence
        .split(|c: char| c.is_whitespace() || "()/".contains(c));
    let ids = words.filter(|w| !["", "OR", "AND", "WITH"].contains(w));
    ids.map(|spdx| supplied(root, &dir, &id, spdx)).collect()
}

/// The standard text of the licence `spdx` for the crate `id` in `dir`, and the line saying so.
fn supplied(root: &Path, dir: &Path, id: &str, spdx: &str) -> Result<(String, String), String> {
    let none = || format!("{id}: its package has no licence text, and none is supplied for {spdx}");
    let (_, from) = STANDARD.iter().find(|(s, _)| *s == spdx).ok_or_else(none)?;
    let text = fs::read_to_string(root.join(from)).map_err(|e| format!("{from}: {e}"))?;
    if spdx != "MIT" {
        let line = format!("{id}  {spdx}, not in its package: the standard text from {from}");
        return Ok((line, trimmed(&text)));
    }
    let toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    let authors = toml.lines().find_map(|l| l.strip_prefix("authors = ["));
    let authors: Vec<_> = authors.map_or(vec![], |a| a.split('"').skip(1).step_by(2).collect());
    if authors.is_empty() {
        return Err(format!(
            "{id}: no authors in its Cargo.toml for the MIT copyright line"
        ));
    }
    let (_, rest) = text
        .split_once('\n')
        .filter(|(first, _)| first.starts_with("Copyright"))
        .ok_or_else(|| format!("{from}: no copyright first line"))?;
    let text = format!("Copyright (c) {}\n{rest}", authors.join(", "));
    let line =
        format!("{id}  MIT, not in its package: {from} with the copyright line from its authors");
    Ok((line, trimmed(&text)))
}

fn trimmed(text: &str) -> String {
    text.trim_end_matches('\n').into()
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
