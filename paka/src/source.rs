//! A program's text: one file, or every regular `.paka` file directly in a directory (M2-02
//! section 1.3).

use crate::diagnostic::Diagnostic;
use std::ffi::{OsStr, OsString};
use std::ops::Range;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// The files in byte order of their names, each followed by a line feed so the next starts a
/// line. Positions count lines across the files until `locate` places them in one.
#[derive(Default)]
pub struct Source {
    pub text: Vec<u8>,
    pub files: Vec<File>,
    lines: usize,
    /// The path a diagnostic without a position names: the argument, or the file not read.
    named: OsString,
}

/// One file: its path as given, its own name, and its bytes and first line in the program's text.
pub struct File {
    path: OsString,
    pub name: String,
    pub bytes: Range<usize>,
    pub line: usize,
}

/// The file of `files` that holds `line`, a line counted across them, and the line in that file;
/// `first` gives a file's first line.
pub fn in_file<T>(files: &[T], first: impl Fn(&T) -> usize, line: usize) -> (&T, usize) {
    let file = &files[files.partition_point(|f| first(f) <= line) - 1];
    (file, line + 1 - first(file))
}

impl Source {
    pub fn file(path: &str, text: Vec<u8>) -> Source {
        let mut source = Source::default();
        source.add(path.into(), text);
        source
    }

    /// Adds the file at `file`, or each file `paka_files` finds when it is a directory.
    fn load(&mut self, file: &OsStr) -> Result<(), (PathBuf, io::Error)> {
        let paths = match Path::new(file).is_dir() {
            true => paka_files(file)?,
            false => vec![file.into()],
        };
        for path in paths {
            let text = fs::read(&path).map_err(|e| (path.clone(), e))?;
            self.add(path.into(), text);
        }
        Ok(())
    }

    fn add(&mut self, path: OsString, text: Vec<u8>) {
        let name = path
            .as_bytes()
            .rsplit(|&b| b == b'/')
            .next()
            .unwrap_or_default();
        let name = String::from_utf8_lossy(name).into();
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

    /// `d` with the path and position of the file it is in, that path as given, and that file's
    /// text.
    pub fn locate(&self, mut d: Diagnostic) -> (Diagnostic, &OsStr, &[u8]) {
        let Some((line, col)) = d.position else {
            d.path = self.named.to_string_lossy().into();
            return (d, &self.named, &[]);
        };
        let (file, line) = in_file(&self.files, |f| f.line, line);
        d.path = file.path.to_string_lossy().into();
        d.position = Some((line, col));
        (d, &file.path, &self.text[file.bytes.clone()])
    }
}

/// The program at `file`, and `file-unreadable` for the path that could not be read, which `locate`
/// then names. A file in a directory is `file`, a `/` and its own name.
pub fn read(file: &OsStr) -> (Source, Result<(), Diagnostic>) {
    let mut source = Source {
        named: file.into(),
        ..Source::default()
    };
    match source.load(file) {
        Ok(()) => (source, Ok(())),
        Err((path, e)) => {
            let d = Diagnostic::tool(&path.to_string_lossy(), "file-unreadable", e);
            source.named = path.into();
            (source, Err(d))
        }
    }
}

/// The regular `.paka` files directly in `dir`, in byte order of their names, each as `dir`, a `/`
/// and its name; or the path that could not be listed or looked at.
fn paka_files(dir: &OsStr) -> Result<Vec<PathBuf>, (PathBuf, io::Error)> {
    let root = dir.as_bytes().strip_suffix(b"/").unwrap_or(dir.as_bytes());
    let entries = fs::read_dir(dir).and_then(|d| d.collect::<Result<Vec<_>, _>>());
    let mut names: Vec<_> = (entries.map_err(|e| (dir.into(), e))?.into_iter())
        .map(|e| e.file_name())
        .filter(|n| n.as_bytes().ends_with(b".paka"))
        .collect();
    names.sort();
    let mut files = vec![];
    for name in names {
        let path = PathBuf::from(OsString::from_vec([root, b"/", name.as_bytes()].concat()));
        match fs::metadata(&path) {
            Ok(m) if m.is_file() => files.push(path),
            Ok(_) => {}
            Err(e) => return Err((path, e)),
        }
    }
    Ok(files)
}
