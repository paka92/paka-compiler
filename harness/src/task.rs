//! A task folder: statement, visible cases, hidden cases (see harness/README.md).

use crate::Error;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

/// Practice tasks: they never enter the verdict or need the `run` tag (M3-04).
pub const PRACTICE: [&str; 2] = ["X1", "X2"];

/// A file's name and bytes.
pub type File = (String, Vec<u8>);

pub struct Task {
    pub statement: String,
    pub visible: Vec<Case>,
    pub hidden: Vec<Case>,
}

pub struct Case {
    pub name: String,
    pub args: Vec<String>,
    /// Input files, put in the working directory before the run.
    pub input: Vec<File>,
    pub stdout: Vec<u8>,
    pub status: i32,
    /// Files the program must have written, with their exact content.
    pub output: Vec<File>,
}

impl Task {
    pub fn load(dir: &Path) -> Result<Task, Error> {
        let statement = read_text(&dir.join("statement.md"))?;
        if statement.trim().is_empty() {
            return Err(bad(dir.join("statement.md"), "is empty"));
        }
        Ok(Task {
            statement,
            visible: cases(&dir.join("cases"))?,
            hidden: cases(&dir.join("hidden"))?,
        })
    }
}

fn bad(path: PathBuf, why: impl Display) -> Error {
    Error::Malformed(format!("{}: {why}", path.display()))
}

/// The entries of a directory, by name.
fn entries(dir: &Path) -> Result<Vec<(String, PathBuf)>, Error> {
    let mut found = vec![];
    for entry in fs::read_dir(dir).map_err(|e| bad(dir.into(), e))? {
        let entry = entry.map_err(|e| bad(dir.into(), e))?;
        found.push((
            entry.file_name().to_string_lossy().into_owned(),
            entry.path(),
        ));
    }
    found.sort();
    Ok(found)
}

fn read(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|e| bad(path.into(), e))
}

fn read_text(path: &Path) -> Result<String, Error> {
    String::from_utf8(read(path)?).map_err(|_| bad(path.into(), "is not UTF-8"))
}

/// The files directly inside `dir`; a folder in it is malformed.
fn files(dir: &Path) -> Result<Vec<File>, Error> {
    let mut found = vec![];
    for (name, path) in entries(dir)? {
        if !path.is_file() {
            return Err(bad(path, "is not a file"));
        }
        found.push((name, read(&path)?));
    }
    Ok(found)
}

/// Every case folder in `dir`; there must be at least one.
fn cases(dir: &Path) -> Result<Vec<Case>, Error> {
    let found = entries(dir)?
        .into_iter()
        .map(|(name, path)| case(name, &path))
        .collect::<Result<Vec<_>, _>>()?;
    if found.is_empty() {
        return Err(bad(dir.into(), "holds no case"));
    }
    Ok(found)
}

fn case(name: String, dir: &Path) -> Result<Case, Error> {
    let mut case = Case {
        name,
        args: vec![],
        input: vec![],
        stdout: vec![],
        status: 0,
        output: vec![],
    };
    let mut has_stdout = false;
    for (entry, path) in entries(dir)? {
        match entry.as_str() {
            "args" => case.args = read_text(&path)?.lines().map(String::from).collect(),
            "status" => {
                let text = read_text(&path)?;
                case.status = text
                    .trim()
                    .parse()
                    .map_err(|_| bad(path, "is not a number"))?;
            }
            "stdout" => (case.stdout, has_stdout) = (read(&path)?, true),
            "in" => case.input = files(&path)?,
            "out" => case.output = files(&path)?,
            _ => return Err(bad(path, "is not part of a case")),
        }
    }
    if !has_stdout {
        return Err(bad(dir.join("stdout"), "is missing"));
    }
    Ok(case)
}
