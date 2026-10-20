//! The differential harness in dev mode (M4-40): each run case of the conformance suite runs on
//! the interpreter and on the dev back end, each in a child process, and the two runs are compared
//! by M2-06 section 1.2's meaning fields.

use std::fs;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;

/// What a run left: its output streams and its status, `None` when a signal ended it.
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
}

impl From<Output> for Outcome {
    fn from(o: Output) -> Outcome {
        let status = o.status.code().filter(|_| o.status.signal().is_none());
        Outcome {
            stdout: o.stdout,
            stderr: o.stderr,
            status,
        }
    }
}

/// How the dev run of `b` differs in meaning from the reference run `a`; `None` when they agree.
pub fn difference(a: &Outcome, b: &Outcome) -> Option<String> {
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    if a.stdout != b.stdout {
        return Some(format!(
            "stdout {:?}, dev {:?}",
            text(&a.stdout),
            text(&b.stdout)
        ));
    }
    let (Some(x), Some(y)) = (a.status, b.status) else {
        return Some(format!(
            "ended by a signal: {:?}, dev {:?}",
            a.status, b.status
        ));
    };
    if x != y {
        return Some(format!("status {x}, dev {y}"));
    }
    let (m, n) = (meaning(a), meaning(b));
    (m != n).then(|| format!("stderr {m:?}, dev {n:?}"))
}

/// The meaning of a run's standard error: all of it, unless the run trapped. A trap's meaning is
/// its first line; only the kind for `stack` and `memory`, and the name for `hole`.
fn meaning(o: &Outcome) -> String {
    let text = String::from_utf8_lossy(&o.stderr);
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let limit = [" trap stack at ", " trap memory at "]
        .into_iter()
        .find(|k| first.contains(k));
    match (o.status, limit) {
        (Some(3), Some(kind)) => first[..first.find(kind).unwrap() + kind.len()].into(),
        (Some(3), None) if first.contains(" trap hole at ") => {
            format!("{first}\n{}", lines.next().unwrap_or(""))
        }
        (Some(3), None) => first.into(),
        _ => text.into_owned(),
    }
}

/// The run cases under `root`, `<area>/<name>/`, in order: those that hold a program and a
/// status, with no `command` of their own.
pub fn cases(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut cases = vec![];
    for area in folders(root)? {
        for case in folders(&area)? {
            let has = |name| case.join(name).exists();
            if has("main.paka") && has("status") && !has("command") {
                cases.push(case);
            }
        }
    }
    Ok(cases)
}

fn folders(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut folders = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            folders.push(path);
        }
    }
    folders.sort();
    Ok(folders)
}

/// Copies case `case` into the empty folder `to`: its own files, then the contents of `files/`
/// (tests/conformance/README.md).
pub fn stage(case: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(case)? {
        let path = entry?.path();
        if path.is_file() {
            fs::copy(&path, to.join(path.file_name().unwrap()))?;
        }
    }
    let files = case.join("files");
    match files.is_dir() {
        true => copy_tree(&files, to),
        false => Ok(()),
    }
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let path = entry?.path();
        let target = to.join(path.file_name().unwrap());
        match path.is_dir() {
            true => copy_tree(&path, &target)?,
            false => drop(fs::copy(&path, target)?),
        }
    }
    Ok(())
}
