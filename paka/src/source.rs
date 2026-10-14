//! A program's text: one file, or every `.paka` file directly in a directory (M2-02 section 1.3).

use paka::diagnostic::Diagnostic;
use std::ffi::OsStr;
use std::ops::Range;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{fs, io};

/// The files in byte order of their names, each followed by a line feed so the next starts a
/// line. Positions count lines across the files until `locate` places them in one.
#[derive(Default)]
pub struct Source {
    pub text: Vec<u8>,
    pub files: Vec<File>,
    lines: usize,
}

/// One file: its path as diagnostics show it, its own name, and its bytes and first line in the
/// program's text.
pub struct File {
    path: String,
    pub name: String,
    pub bytes: Range<usize>,
    pub line: usize,
}

impl Source {
    pub fn file(path: &str, text: Vec<u8>) -> Source {
        let mut source = Source::default();
        source.add(path.into(), text);
        source
    }

    fn add(&mut self, path: String, text: Vec<u8>) {
        let name = path.rsplit('/').next().unwrap_or_default().into();
        let line = self.lines + 1;
        self.lines += 1 + text.iter().filter(|&&b| b == b'\n').count();
        let start = self.text.len();
        self.text.extend(text);
        let bytes = start..self.text.len();
        self.text.push(b'\n');
        let file = File {
            path,
            name,
            bytes,
            line,
        };
        self.files.push(file);
    }

    /// `d` with the path and position of the file it is in, and that file's text.
    pub fn locate(&self, mut d: Diagnostic) -> (Diagnostic, &[u8]) {
        let Some((line, col)) = d.position else {
            return (d, &[]);
        };
        let file = (self.files.iter().rfind(|f| f.line <= line)).expect("a position is in a file");
        d.path.clone_from(&file.path);
        d.position = Some((line + 1 - file.line, col));
        (d, &self.text[file.bytes.clone()])
    }
}

/// The program at `file`; `path` is `file` as diagnostics show it, bytes that are not UTF-8
/// replaced. A file in a directory is shown as `path`, a `/` and its own name.
pub fn read(file: &OsStr, path: &str) -> Result<Source, Diagnostic> {
    let unreadable = |path: &str, e: io::Error| Diagnostic::tool(path, "file-unreadable", e);
    if !Path::new(file).is_dir() {
        let text = fs::read(file).map_err(|e| unreadable(path, e))?;
        return Ok(Source::file(path, text));
    }
    let entries = fs::read_dir(file).and_then(|dir| dir.collect::<Result<Vec<_>, _>>());
    let mut files: Vec<_> = (entries.map_err(|e| unreadable(path, e))?.into_iter())
        .map(|e| e.path())
        .filter(|f| f.as_os_str().as_bytes().ends_with(b".paka") && f.is_file())
        .collect();
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    let (dir, mut source) = (path.strip_suffix('/').unwrap_or(path), Source::default());
    for file in files {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let shown = format!("{dir}/{name}");
        let text = fs::read(&file).map_err(|e| unreadable(&shown, e))?;
        source.add(shown, text);
    }
    Ok(source)
}
