//! The differential harness in dev mode (M4-40): each run case of the conformance suite runs on
//! the interpreter and on the dev back end, each in a child process, and the two runs are compared
//! by M2-06 section 1.2's meaning fields. UI examples are compared action by action (M5-26), in
//! release through `--paka-replay` (M7-11c).

use crate::dev::{Dev, Played};
use paka::core_form::{Fn, Program};
use paka::interp::{Logged, ui_log};
pub use paka::release::{Outcome, in_release};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Dropped;
use paka_rt::replay::Left;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// `run` as stopped past its limit: status 4 and `stopped: time` its whole standard error.
pub fn stop(run: Outcome) -> Outcome {
    Outcome {
        stderr: b"stopped: time\n".to_vec(),
        status: Some(4),
        ..run
    }
}

/// Whether `run` was stopped past its limit, maybe just after it printed a trap.
pub fn stopped(run: &Outcome) -> bool {
    run.status == Some(4) && run.stderr.ends_with(b"stopped: time\n")
}

/// The time a compiled run gets after an interpreter run of `interp`: compiled code outruns the
/// interpreter, but a stack trap first fills the 64 MiB runner stack, and a busy machine stalls.
pub fn compiled_limit(interp: Duration) -> Duration {
    (interp * 4).max(Duration::from_secs(1))
}

/// A busy machine can stall a compiled run past its limit: a stopped one runs again with this many
/// times the limit.
pub const AGAIN: u32 = 10;

/// The compiled run to compare with `reference`, or why it is slow. `first`, stopped past `limit`
/// after a prefix of the reference's output, runs again by `again`, which gets the longer limit
/// and says how long the run took. That run stands unless it ended as the reference did but
/// needed more than `limit`.
pub fn verdict<T>(
    reference: &Outcome,
    first: (Outcome, T),
    limit: Duration,
    again: impl FnOnce(Duration) -> io::Result<((Outcome, T), Duration)>,
) -> io::Result<Result<(Outcome, T), String>> {
    if !stopped(&first.0) || !reference.stdout.starts_with(&first.0.stdout) {
        return Ok(Ok(first));
    }
    let (run, took) = again(limit * AGAIN)?;
    let other = run.0.stdout != reference.stdout || run.0.status != reference.status;
    if stopped(&run.0) || took <= limit || other {
        return Ok(Ok(run));
    }
    let ms = |d: Duration| d.as_millis();
    let (limit, took) = (ms(limit), ms(took));
    Ok(Err(format!("stopped past {limit} ms, then took {took} ms")))
}

/// Makes folder `to` a copy of folder `from`, so a run can start again from what was staged.
pub fn mirror(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }
    copy_tree(from, to)
}

/// How the dev run of `b` differs in meaning from the reference run `a`; `None` when they agree.
pub fn difference(a: &Outcome, b: &Outcome) -> Option<String> {
    difference_in(a, b, "dev")
}

/// The release verdict on `run` against the interpreter's `reference`: 0 agree, 1 differ or 2 not
/// compared, and its note.
pub fn release_verdict(reference: &Outcome, run: &Outcome) -> (usize, String) {
    let trap = String::from_utf8_lossy(&reference.stderr);
    let stack = trap
        .lines()
        .next()
        .is_some_and(|l| l.contains(" trap stack at "));
    match difference_in(reference, run, "release") {
        // Release may finish or run on where the interpreter's stack ran out, not crash (M6-01),
        // after the same output.
        _ if reference.status == Some(3)
            && stack
            && run.status.is_some()
            && run.stdout.starts_with(&reference.stdout) =>
        {
            (2, "not compared: a stack trap".into())
        }
        Some(why) => (1, why),
        None => (0, "agree".into()),
    }
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

/// Writes the expectations of run case `dir`: what `run` left. A run killed or stopped past its
/// limit shows speed, not meaning, and is refused.
pub fn expect(dir: &Path, run: &Outcome) -> io::Result<()> {
    let status = match run.status {
        None => return Err(io::Error::other("the interpreter was killed")),
        Some(4) => return Err(io::Error::other("the interpreter was stopped")),
        Some(status) => status,
    };
    fs::create_dir_all(dir)?;
    fs::write(dir.join("stdout"), &run.stdout)?;
    fs::write(dir.join("status"), format!("{status}\n"))?;
    if !run.stderr.is_empty() {
        fs::write(dir.join("stderr"), &run.stderr)?;
    }
    Ok(())
}

/// Fails when `dir` has no room for a build's files (16 MiB), so a full disk ends the run instead
/// of reading as a divergence.
pub fn room(dir: &Path) -> io::Result<()> {
    let probe = dir.join("room");
    let written = fs::write(&probe, vec![0; 16 << 20]);
    let _ = fs::remove_file(&probe);
    written
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
/// what the action left in the interpreter, and free all they made. Each example's name and its
/// log, or how they differ.
pub fn ui_examples(
    core: &Program,
    dev: &Dev,
    heap: u64,
    limit: Duration,
) -> Vec<(String, Result<Vec<Logged>, String>)> {
    let compare = |f: &Fn| {
        let log = ui_log(core, f).map_err(|why| format!("interpreter: {why}"))?;
        for (k, (_, events, want)) in log.iter().enumerate() {
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
        Ok(log)
    };
    let mut all: Vec<_> = core
        .ui_examples
        .iter()
        .map(|f| (f.name.clone(), compare(f)))
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

/// What a dev run left, in the interpreter's terms: its first trap, as the interpreter stops there.
fn left(played: &Played) -> Left {
    match &played.end {
        _ if !played.traps.is_empty() => {
            Left::Trapped(played.traps[0].lines().next().unwrap_or_default().into())
        }
        None => Left::Tree(played.tree.clone()),
        Some(Ok((status, line))) if line.is_empty() => Left::Exited(*status, played.tree.clone()),
        Some(Ok((status, line))) => Left::Ended(*status, line.clone()),
        Some(Err(Stopped::Time)) => Left::Ended(4, "stopped: time".into()),
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
