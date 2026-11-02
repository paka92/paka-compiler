//! Runs a release binary's examples, each in its own process (docs/decisions/M7-03.md 1.1): a
//! function example (M7-11b), trusting only the exact pass: status 0, standard output
//! `NAME pass` and nothing on standard error; and a UI example's histories through
//! `--paka-replay`, compared with the interpreter's log (M7-11c).

use crate::interp::Logged;
use paka_rt::replay::{Action, Left};
use std::fmt;
use std::io::{self, Read, Write};
use std::mem;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// How long one run may take.
pub const LIMIT: Duration = Duration::from_secs(10);

/// How a run of one example ended.
#[derive(Debug, PartialEq)]
pub enum Ran {
    Pass,
    Fail,
    /// The binary's trap line.
    Trapped(String),
    TimedOut,
    /// Any other end, as the status and output seen.
    Other(String),
}

impl fmt::Display for Ran {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ran::Pass => f.write_str("pass"),
            Ran::Fail => f.write_str("fail"),
            Ran::Trapped(line) => write!(f, "trapped: {line}"),
            Ran::TimedOut => f.write_str("timed out"),
            Ran::Other(seen) => f.write_str(seen),
        }
    }
}

/// What a run left: its output streams and its status, `None` when a signal ended it.
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
}

impl From<Output> for Outcome {
    fn from(o: Output) -> Outcome {
        Outcome {
            stdout: o.stdout,
            stderr: o.stderr,
            status: code(o.status),
        }
    }
}

impl Outcome {
    /// Whether the run was stopped past its limit, maybe just after it printed a trap.
    pub fn stopped(&self) -> bool {
        self.status == Some(4) && self.stderr.ends_with(b"stopped: time\n")
    }
}

/// The status a run ended with, `None` when a signal ended it.
fn code(status: ExitStatus) -> Option<i32> {
    status.code().filter(|_| status.signal().is_none())
}

/// Runs example `name` of `exe`, killed past `LIMIT`.
pub fn run(exe: &Path, name: &str) -> io::Result<Ran> {
    let command = &mut Command::new(exe);
    let Ok(o) = exec(command.args(["--paka-examples", name]), b"", LIMIT)? else {
        return Ok(Ran::TimedOut);
    };
    let [out, err] = [o.stdout, o.stderr].map(|b| String::from_utf8_lossy(&b).into_owned());
    let trap = (err.strip_suffix('\n')).filter(|l| !l.contains('\n') && l.contains(": trap "));
    Ok(match (o.status, out.as_str(), err.as_str(), trap) {
        (Some(0), o, "", _) if o == format!("{name} pass\n") => Ran::Pass,
        (Some(0), o, "", _) if o == format!("{name} fail\n") => Ran::Fail,
        (Some(3), "", _, Some(line)) => Ran::Trapped(line.into()),
        (code, o, e, _) => {
            let ended = code.map_or("killed by a signal".into(), |c| format!("status {c}"));
            let [o, e] = [o, e].map(tail);
            Ran::Other(format!(
                "{ended}, standard output {o:?}, standard error {e:?}"
            ))
        }
    })
}

/// Replays `actions` on `command`'s binary from its launch, killed past `LIMIT`.
pub fn replay(command: &mut Command, actions: &[Action]) -> Result<Outcome, String> {
    let lines: String = actions.iter().map(|a| format!("{a}\n")).collect();
    let exe = Path::new(command.get_program()).display().to_string();
    match exec(command.arg("--paka-replay"), lines.as_bytes(), LIMIT) {
        Ok(Ok(o)) => Ok(o),
        Ok(Err(_)) => Err("the replay timed out".into()),
        Err(e) => Err(format!("{exe}: {e}")),
    }
}

/// The most of each output stream a run keeps; the rest is read and dropped.
const KEPT: usize = 16 << 20;

/// Runs `command` with `input` on standard input: what it left, or as `Err` what it left by
/// `limit` when it ran past it and was killed. A pipe a grandchild holds open past `limit`
/// counts as running past it; that grandchild is left running.
pub fn exec(
    command: &mut Command,
    input: &[u8],
    limit: Duration,
) -> io::Result<Result<Outcome, Outcome>> {
    let deadline = Instant::now() + limit;
    let mut child = (command.stdin(Stdio::piped()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (mut stdin, input) = (child.stdin.take().expect("piped"), input.to_vec());
    // A binary may end before it reads all of its input.
    thread::spawn(move || _ = stdin.write_all(&input));
    let (done, ended) = mpsc::channel();
    let out = kept(child.stdout.take().expect("piped"), done.clone());
    let err = kept(child.stderr.take().expect("piped"), done);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() > deadline {
            child.kill()?;
            child.wait()?;
            break None;
        }
        thread::sleep(Duration::from_millis(2));
    };
    let drained = (0..2).all(|_| {
        let left = deadline.saturating_duration_since(Instant::now());
        ended.recv_timeout(left).is_ok()
    });
    let taken = |b: Arc<Mutex<Vec<u8>>>| mem::take(&mut *b.lock().expect("reader"));
    let o = Outcome {
        stdout: taken(out),
        stderr: taken(err),
        status: status.and_then(code),
    };
    Ok(if status.is_some() && drained {
        Ok(o)
    } else {
        Err(o)
    })
}

/// Reads `pipe` to its end on a thread of its own, keeping its first `KEPT` bytes, then says so
/// on `done`.
fn kept(mut pipe: impl Read + Send + 'static, done: mpsc::Sender<()>) -> Arc<Mutex<Vec<u8>>> {
    let bytes = Arc::new(Mutex::new(vec![]));
    let into = Arc::clone(&bytes);
    thread::spawn(move || {
        let mut chunk = [0; 1 << 16];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let mut kept = into.lock().expect("reader");
                    let room = KEPT.saturating_sub(kept.len());
                    kept.extend_from_slice(&chunk[..n.min(room)]);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        _ = done.send(());
    });
    bytes
}

/// How many characters of a run's output a refusal shows.
const TAIL: usize = 200;

/// The last `TAIL` characters of `text`, after a `…` when it is longer.
fn tail(text: &str) -> String {
    match text.char_indices().rev().nth(TAIL - 1) {
        Some((i, _)) if i > 0 => format!("…{}", &text[i..]),
        _ => text.into(),
    }
}

/// Compares release with `log`, each history its own replay from the launch by `run`: those no
/// other history extends, which hold the rest. The count of actions compared, or how they differ.
pub fn in_release(
    log: &[Logged],
    mut run: impl FnMut(&[Action]) -> Result<Outcome, String>,
) -> Result<usize, String> {
    let at = |h: &[Action]| log.iter().position(|(g, ..)| g == h);
    if let Some(k) = (0..log.len()).find(|&k| at(&log[k].0).is_some_and(|j| log[j].2 != log[k].2)) {
        return Err(format!(
            "interpreter: action {k} left another result for the same actions"
        ));
    }
    let n = log.len();
    let mut done: Vec<&[Action]> = vec![];
    for (k, (h, ..)) in log.iter().enumerate() {
        let extended = log
            .iter()
            .any(|(g, ..)| g.len() > h.len() && g.starts_with(h));
        if extended || at(h) != Some(k) {
            continue;
        }
        let want = (0..=h.len()).map(|i| at(&h[..i]).map(|j| &log[j].2));
        let want: Vec<_> = want
            .collect::<Option<_>>()
            .ok_or("the log skips an action")?;
        if let Err((i, why)) = replayed(&want, &run(h)?) {
            let seen = |g: &[Action]| done.iter().any(|d| d.starts_with(g));
            let compared = log
                .iter()
                .filter(|(g, ..)| seen(g) || g.len() < i && h.starts_with(g));
            let why = match h.get(..i).and_then(at) {
                Some(j) => format!("action {j}: {why}"),
                None => why,
            };
            return Err(format!(
                "release compared on {} of {n} actions: {why}",
                compared.count()
            ));
        }
        done.push(h);
    }
    Ok(n)
}

/// Compares a release replay with `want`, what its launch and each action left: each result on
/// standard output, then the end on standard error and the status. On a difference, how many
/// results agree before it, and how it differs.
fn replayed(want: &[&Left], got: &Outcome) -> Result<(), (usize, String)> {
    let (n, out) = (want.len(), String::from_utf8_lossy(&got.stdout));
    let err = String::from_utf8_lossy(&got.stderr);
    let mut rest = &out[..];
    for (k, want) in want.iter().enumerate() {
        let want = want.to_string();
        let Some(after) = rest.strip_prefix(&want) else {
            let why = match want.lines().zip(rest.lines()).find(|(a, b)| a != b) {
                Some((a, b)) => format!("line {a:?}, release {:?}", tail(b)),
                None => format!(
                    "release output ends, status {:?} and {:?}",
                    got.status,
                    tail(&err)
                ),
            };
            return Err((k, why));
        };
        rest = after;
    }
    if let Some(line) = rest.lines().next() {
        return Err((
            n,
            format!("release goes on after the last action: {:?}", tail(line)),
        ));
    }
    let (status, end) = match want.last() {
        Some(Left::Exited(status, _)) => (*status, ""),
        Some(Left::Ended(status, line)) => (*status, line.as_str()),
        _ => (0, ""),
    };
    if got.status == Some(i32::from(status)) && err == end {
        return Ok(());
    }
    Err((
        n,
        format!(
            "release ended with status {:?} and {:?}, want {status} and {end:?}",
            got.status,
            tail(&err)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(n: u8) -> String {
        format!("Col gap=0\n  Button #0 \"{n}\" on=Quit\n")
    }

    fn click(n: usize) -> Action {
        Action::parse(&format!("click #{n}")).unwrap()
    }

    /// A launch, a click and the `ExitWith(7)` the next one made.
    fn exited() -> Vec<Logged> {
        vec![
            (vec![], vec![], Left::Tree(tree(0))),
            (vec![click(0)], vec![], Left::Tree(tree(1))),
            (vec![click(0); 2], vec![], Left::Exited(7, tree(1))),
        ]
    }

    /// What a release replay writes that leaves `log`'s results and ends with `status`.
    fn written(log: &[Logged], status: i32) -> Outcome {
        let stdout: String = log.iter().map(|(.., l)| l.to_string()).collect();
        Outcome {
            stdout: stdout.into_bytes(),
            stderr: vec![],
            status: Some(status),
        }
    }

    #[test]
    fn a_replay_agrees_on_every_action() {
        let log = exited();
        assert_eq!(in_release(&log, |_| Ok(written(&log, 7))), Ok(3));
    }

    #[test]
    fn a_replay_cut_short_differs_with_its_end() {
        let log = exited();
        let mut out = written(&log[..2], 2);
        out.stderr = b"a: not a replay action: click\n".to_vec();
        let why = r#"release compared on 2 of 3 actions: action 2: release output ends, status Some(2) and "a: not a replay action: click\n""#;
        assert_eq!(in_release(&log, |_| Ok(out.clone())), Err(why.into()));
    }

    #[test]
    fn another_result_more_output_or_another_status_differs() {
        let log = exited();
        let out = written(&log, 7);
        let text = String::from_utf8(out.stdout.clone()).unwrap();
        let changed = Outcome {
            stdout: text.replacen("\"1\"", "\"2\"", 1).into_bytes(),
            ..out.clone()
        };
        let why = r#"release compared on 1 of 3 actions: action 1: line "  Button #0 \"1\" on=Quit", release "  Button #0 \"2\" on=Quit""#;
        assert_eq!(in_release(&log, |_| Ok(changed.clone())), Err(why.into()));
        let more = Outcome {
            stdout: [text.as_bytes(), b"Col gap=0\n"].concat(),
            ..out.clone()
        };
        let why = r#"release compared on 3 of 3 actions: release goes on after the last action: "Col gap=0""#;
        assert_eq!(in_release(&log, |_| Ok(more.clone())), Err(why.into()));
        let zero = Outcome {
            status: Some(0),
            ..out
        };
        let why = r#"release compared on 3 of 3 actions: release ended with status Some(0) and "", want 7 and """#;
        assert_eq!(in_release(&log, |_| Ok(zero.clone())), Err(why.into()));
    }

    /// Two values, one copied after a click: one replay per history no other extends.
    #[test]
    fn each_history_is_replayed_once_from_its_launch() {
        let left = |n| Left::Tree(tree(n));
        let log: Vec<Logged> = vec![
            (vec![], vec![], left(0)),
            (vec![click(0)], vec![], left(1)),
            (vec![], vec![], left(0)),
            (vec![click(0), click(0)], vec![], left(2)),
            (vec![click(0), click(1)], vec![], left(3)),
        ];
        let mut runs = vec![];
        let run = |h: &[Action]| {
            runs.push(h.to_vec());
            let last = if h[1] == click(0) { 3 } else { 4 };
            Ok(written(&[&log[..2], &log[last..=last]].concat(), 0))
        };
        assert_eq!(in_release(&log, run), Ok(5));
        assert_eq!(runs, [log[3].0.clone(), log[4].0.clone()]);
    }
}
