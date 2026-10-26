//! The differential harness in dev mode (M4-40): each run case of the conformance suite runs on
//! the interpreter and on the dev back end, each in a child process, and the two runs are compared
//! by M2-06 section 1.2's meaning fields. UI examples are compared action by action (M5-26).

use crate::dev::{Dev, Played};
use paka::core_form::{Fn, Program};
use paka::interp::{Left, ui_log};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Dropped;
use std::fs;
use std::io;
use std::ops::Range;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

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
    difference_in(a, b, "dev")
}

/// How the run of `b` in mode `mode` differs in meaning from the reference run `a`.
pub fn difference_in(a: &Outcome, b: &Outcome, mode: &str) -> Option<String> {
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    if a.stdout != b.stdout {
        return Some(format!(
            "stdout {:?}, {mode} {:?}",
            text(&a.stdout),
            text(&b.stdout)
        ));
    }
    let (Some(x), Some(y)) = (a.status, b.status) else {
        return Some(format!(
            "ended by a signal: {:?}, {mode} {:?}",
            a.status, b.status
        ));
    };
    if x != y {
        return Some(format!("status {x}, {mode} {y}"));
    }
    let (m, n) = (meaning(a), meaning(b));
    (m != n).then(|| format!("stderr {m:?}, {mode} {n:?}"))
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

/// Shrinks the program `text` while `keeps` holds of it (M4-41): drops a line, a block whole, a
/// block's braces with its body kept, or one item of a bracketed list, until no drop keeps it.
pub fn shrink(text: &str, mut keeps: impl FnMut(&str) -> bool) -> String {
    let joined = |lines: &[String]| lines.join("\n") + "\n";
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    loop {
        let before = joined(&lines);
        for i in (0..lines.len()).rev() {
            if let Some(kept) = drops(&lines, i).into_iter().find(|c| keeps(&joined(c))) {
                lines = kept;
            }
        }
        if joined(&lines) == before {
            return before;
        }
    }
}

/// The programs one drop at line `i` makes, if the line is still there.
fn drops(lines: &[String], i: usize) -> Vec<Vec<String>> {
    let Some(line) = lines.get(i) else {
        return vec![];
    };
    let without = |cut: Vec<usize>| -> Vec<String> {
        (lines.iter().enumerate())
            .filter(|(k, _)| !cut.contains(k))
            .map(|(_, l)| l.clone())
            .collect()
    };
    let items = items(line).into_iter().map(|r| {
        let mut edited = lines.to_vec();
        edited[i].replace_range(r, "");
        edited
    });
    cuts(lines, i)
        .into_iter()
        .map(without)
        .chain(items)
        .collect()
}

/// The lines a drop at line `i` cuts: its block whole, or the line alone; then the block's
/// braces. A block that follows another's `}`, as `} else {` does, keeps the last `}`.
fn cuts(lines: &[String], i: usize) -> Vec<Vec<usize>> {
    let line = &lines[i];
    let close = (line.trim_end().ends_with('{'))
        .then(|| closing(&lines[i + 1..]).map(|j| i + 1 + j))
        .flatten();
    let Some(close) = close else {
        return vec![vec![i]];
    };
    let whole = match line.trim_start().starts_with('}') {
        true => (i..close).collect(),
        false => (i..=close).collect(),
    };
    vec![whole, vec![i, close]]
}

/// The line among `lines` that closes a block opened just before them.
fn closing(lines: &[String]) -> Option<usize> {
    let mut depth = 1;
    lines.iter().position(|l| {
        l.chars().any(|c| {
            depth += i32::from(c == '{') - i32::from(c == '}');
            depth == 0
        })
    })
}

/// The byte ranges of `line` that each hold one item of a bracketed list with its comma.
fn items(line: &str) -> Vec<Range<usize>> {
    let (mut ranges, mut groups) = (vec![], vec![]);
    let (mut quoted, mut escaped) = (false, false);
    for (k, b) in line.bytes().enumerate() {
        match (quoted, b) {
            (true, _) if escaped => escaped = false,
            (true, b'\\') => escaped = true,
            (_, b'"') => quoted = !quoted,
            (true, _) => {}
            (false, b'(' | b'[' | b'{') => groups.push((b, vec![k])),
            (false, b',') => {
                if let Some((_, marks)) = groups.last_mut() {
                    marks.push(k);
                }
            }
            (false, b')' | b']' | b'}') => {
                let Some((open, mut marks)) = groups.pop() else {
                    continue;
                };
                marks.push(k);
                if open != b'{' {
                    ranges.extend(spans(line, &marks));
                }
            }
            _ => {}
        }
    }
    ranges
}

/// The items between `marks`, an opening bracket, its commas and its close: the first with the
/// comma and space after it, each other with the comma before it.
fn spans(line: &str, marks: &[usize]) -> Vec<Range<usize>> {
    let (open, close) = (marks[0], marks[marks.len() - 1]);
    if marks.len() == 2 {
        return (close > open + 1)
            .then(|| open + 1..close)
            .into_iter()
            .collect();
    }
    let space = usize::from(line.as_bytes()[marks[1] + 1] == b' ');
    let first = open + 1..marks[1] + 1 + space;
    let rest = marks[1..].windows(2).map(|w| w[0]..w[1]);
    [first].into_iter().chain(rest).collect()
}

/// Turns off core dumps of this process and the programs it starts: a fault that crashes a run
/// would hand one to the system's crash reporter for every program the shrinker tries. A core
/// limit of 1 byte stops a piped dump too, where 0 does not.
pub fn no_core_dumps() {
    unsafe extern "C" {
        fn prctl(option: i32, arg: u64, ...) -> i32;
        fn getrlimit(resource: i32, limit: *mut [u64; 2]) -> i32;
        fn setrlimit(resource: i32, limit: *const [u64; 2]) -> i32;
    }
    const PR_SET_DUMPABLE: i32 = 4;
    const RLIMIT_CORE: i32 = 4;
    let mut limit = [0; 2];
    // SAFETY: `PR_SET_DUMPABLE` takes one integer, 0 or 1, and touches no memory of ours; the
    // limit calls read or write one `rlimit`, two words.
    unsafe {
        prctl(PR_SET_DUMPABLE, 0);
        if getrlimit(RLIMIT_CORE, &mut limit) == 0 && limit[1] >= 1 {
            setrlimit(RLIMIT_CORE, &[1, limit[1]]);
        }
    }
}

/// Plays each UI example of `core`, installed in `dev`, on both back ends: the events of every
/// action it takes, replayed on dev code from the start on a heap of `heap` bytes, must leave
/// what the action left in the interpreter, and free all they made. Each example's name and the
/// number of actions compared, or how they differ.
pub fn ui_examples(
    core: &Program,
    dev: &Dev,
    heap: u64,
    limit: Duration,
) -> Vec<(String, Result<usize, String>)> {
    let compare = |f: &Fn| {
        let log = ui_log(core, f).map_err(|why| format!("interpreter: {why}"))?;
        for (k, (events, want)) in log.iter().enumerate() {
            let played = dev.play(&[], &mut Dropped, heap, limit, events.clone());
            let got = left(&played);
            if got != *want {
                return Err(format!("action {k}: {}", unlike(want, &got)));
            }
            let (leaks, faults) = played.ledger;
            if leaks + faults > 0 {
                return Err(format!(
                    "action {k}: ledger {leaks} leaks, {faults} bad frees"
                ));
            }
        }
        Ok(log.len())
    };
    let mut all: Vec<_> = core
        .ui_examples
        .iter()
        .map(|f| (f.name.clone(), compare(f)))
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

/// What a dev run left, in the interpreter's terms.
fn left(played: &Played) -> Left {
    let first = |r: &String| r.lines().next().unwrap_or_default().to_owned();
    match &played.end {
        _ if !played.traps.is_empty() => Left::Trapped(
            played
                .traps
                .iter()
                .map(first)
                .collect::<Vec<_>>()
                .join("; "),
        ),
        None => Left::Tree(played.tree.clone()),
        Some(Ok((_, line))) if line.is_empty() => Left::Exited(played.tree.clone()),
        Some(Ok((_, line))) => Left::Ended(line.clone()),
        Some(Err(Stopped::Time)) => Left::Ended("stopped: time".into()),
        Some(Err(Stopped::Trap)) => unreachable!("`play` reports a trap that ends the app"),
    }
}

/// How `got` differs from `want`; for two trees, by their first unequal lines.
fn unlike(want: &Left, got: &Left) -> String {
    if let (Left::Tree(a), Left::Tree(b)) = (want, got)
        && let Some((x, y)) = a.lines().zip(b.lines()).find(|(x, y)| x != y)
    {
        return format!("tree line {x:?}, dev {y:?}");
    }
    format!("{want:?}, dev {got:?}")
}
