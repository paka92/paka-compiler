//! The trial loop: runs a batch of runs, each one `claude -p` conversation of up to eight
//! submissions, judged on the task's cases, with a raw log per attempt (M3-01, M3-02, M3-03).
//!
//! Folders, all plain files:
//!
//! ```text
//! <root>/<task>-<lang>-<rep>/<attempt>/log       the raw log (see `log`)
//! <root>/<task>-<lang>-<rep>/<attempt>/claude/   the process's working directory, with
//!                                                stream.log, stderr.log and claude.pid
//! ```
//!
//! A run whose last attempt ended is never run again. An attempt cut short by a kill is closed as
//! void when the batch resumes. A void run is run again; a third void attempt pauses the batch
//! (M3-03 section 1.1). An attempt stopped by the usage cap does not count towards the three.

use crate::Error;
use crate::client::{self, Config, Reply, Run};
use crate::commit;
use crate::json::{self, quote};
use crate::log::{self, Line, Log, State};
use crate::prompt::{self, SUBMISSIONS};
use crate::runner::{CaseResult, Lang, Report, Runner, Verdict};
use crate::task::{PRACTICE, Task};
use crate::usage::{Ledger, Tokens};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Output tokens one reply may use, thinking included; a longer reply counts as cut off.
pub const REPLY_LIMIT: u64 = 32_000;
/// Void attempts per run: the first and two reruns.
const ATTEMPTS: u32 = 3;
const CUT_SHORT: &str = "the harness stopped before the run ended";

/// What every run of a batch shares.
pub struct Setup {
    /// The `claude` command.
    pub claude: PathBuf,
    pub model: String,
    pub effort: String,
    /// The Paka language reference, sent with the system text of Paka runs only.
    pub reference: String,
    pub runner: Runner,
    pub ledger: Ledger,
    /// The watchdog's wall-clock limit on one model call.
    pub call_limit: Duration,
    /// Where the runs' folders go.
    pub root: PathBuf,
    /// The checkout a counted batch must come from (see `commit`); `None` skips the check.
    pub repo: Option<PathBuf>,
}

/// One run: a task, a language and a repetition.
pub struct Spec {
    /// The task folder.
    pub task: PathBuf,
    pub lang: Lang,
    pub rep: u32,
    /// The first call's expected input tokens (M3-01 section 1.7); `None` skips the guard.
    pub first_call_input: Option<u64>,
}

impl Spec {
    pub fn id(&self) -> String {
        let task = self.task.file_name().unwrap_or_default().to_string_lossy();
        format!("{task}-{}-{}", self.lang.name(), self.rep)
    }

    fn is_practice(&self) -> bool {
        let task = self.task.file_name().unwrap_or_default().to_string_lossy();
        PRACTICE.contains(&&*task)
    }
}

/// Runs every run of `specs` that has not ended, in order, and gives the tokens the batch used.
/// Stops at the usage caps and when a run is void for the third time.
pub fn run_batch(setup: &Setup, specs: &[Spec]) -> Result<Tokens, Error> {
    if let Some(repo) = &setup.repo
        && specs.iter().any(|s| !s.is_practice())
    {
        commit::check(repo)?;
    }
    let before = setup.ledger.used()?;
    for spec in specs {
        settle(setup, spec)?;
    }
    let after = setup.ledger.used()?;
    Ok(Tokens {
        input: after.input - before.input,
        output: after.output - before.output,
    })
}

/// Brings one run to an end, resuming after the attempts already logged.
fn settle(setup: &Setup, spec: &Spec) -> Result<(), Error> {
    let dir = setup.root.join(spec.id());
    let task = Task::load(&spec.task)?;
    let mut faults = 0;
    for n in 1.. {
        let attempt = dir.join(n.to_string());
        let state = if attempt.exists() {
            resume(&attempt)?
        } else if faults < ATTEMPTS {
            setup.ledger.check()?;
            run(setup, spec, &task, &attempt)?
        } else {
            break;
        };
        match state {
            State::Ended => return Ok(()),
            State::Void { fault } => faults += u32::from(fault),
            State::Cut => unreachable!("attempts end or are closed as void"),
        }
    }
    Err(Error::Paused(spec.id()))
}

/// How an attempt logged before ended. One cut short is closed as void, after its process group
/// is killed if it is still running.
fn resume(attempt: &Path) -> Result<State, Error> {
    let log = attempt.join("log");
    let state = log::state(&log)?;
    if state == State::Cut {
        kill_leftover(&attempt.join("claude"));
        log::close_cut(&log, CUT_SHORT)?;
        return Ok(State::Void { fault: true });
    }
    Ok(state)
}

/// Kills the process group of a `claude` process that a killed harness left running in `dir`.
fn kill_leftover(dir: &Path) {
    let Some(pid) = fs::read_to_string(dir.join("claude.pid"))
        .ok()
        .and_then(|pid| pid.trim().parse().ok())
    else {
        return;
    };
    // A process id can be reused: only a process still working in this folder is ours.
    let cwd = fs::read_link(format!("/proc/{pid}/cwd"));
    if matches!((cwd, fs::canonicalize(dir)), (Ok(cwd), Ok(dir)) if cwd == dir) {
        client::kill_group(pid);
    }
}

/// Runs one attempt in `dir` and logs it. An error only when the log cannot be written.
fn run(setup: &Setup, spec: &Spec, task: &Task, dir: &Path) -> Result<State, Error> {
    let claude = dir.join("claude");
    fs::create_dir_all(&claude)?;
    let mut log = Log::create(&dir.join("log"))?;
    let system = prompt::system(spec.lang, &setup.reference);
    log.write(
        Line::new("start")
            .text("run", &spec.id())
            .text("model", &setup.model)
            .text("effort", &setup.effort)
            .text("system", &system)
            .now(),
    )?;
    let config = Config {
        program: setup.claude.clone(),
        model: setup.model.clone(),
        effort: setup.effort.clone(),
        system_prompt: system,
        dir: claude,
        first_call_input: spec.first_call_input,
        call_limit: setup.call_limit,
    };
    let (line, state) = match converse(setup, &config, spec.lang, task, &mut log) {
        Ok(tally) => (tally.line(), State::Ended),
        Err(e) => {
            // Reaching the usage cap is a budget stop, not a fault of the run.
            let fault = !matches!(e, Error::CapReached { .. });
            let line = Line::new("void")
                .text("reason", &e.to_string())
                .raw("fault", fault);
            (line, State::Void { fault })
        }
    };
    log.write(line.now())?;
    Ok(state)
}

/// What a finished run counts (M3-01 section 2, M3-02 section 1.9).
#[derive(Default)]
struct Tally {
    submissions: u32,
    passed: bool,
    tokens: Tokens,
    cut_off: u32,
}

impl Tally {
    fn line(&self) -> Line {
        // A run that never passes counts as cap plus one (M3-02 section 1.3).
        let edits = if self.passed {
            self.submissions - 1
        } else {
            SUBMISSIONS
        };
        Line::new("end")
            .raw("passed", self.passed)
            .raw("first_try", self.passed && self.submissions == 1)
            .raw("submissions", self.submissions)
            .raw("edits", edits)
            .raw("input", self.tokens.input)
            .raw("output", self.tokens.output)
            .raw("cut_off", self.cut_off)
    }
}

/// The conversation: up to eight submissions, each judged, the feedback sent back. Any error
/// voids the run.
fn converse(
    setup: &Setup,
    config: &Config,
    lang: Lang,
    task: &Task,
    log: &mut Log,
) -> Result<Tally, Error> {
    let mut run = Run::start(config, &setup.ledger)?;
    let mut tally = Tally::default();
    let mut message = prompt::first(task);
    for n in 1..=SUBMISSIONS {
        let reply = call(&mut run, n, &message, log)?;
        let cut_off = reply.cut_off || reply.tokens.output > REPLY_LIMIT;
        tally.submissions = n;
        tally.tokens.input += reply.tokens.input;
        tally.tokens.output += reply.tokens.output;
        tally.cut_off += u32::from(cut_off);
        let source = if cut_off {
            Err("the reply was cut off at the reply limit")
        } else {
            program(&reply.text)
        };
        message = match source {
            Err(why) => {
                log.write(Line::new("submission").raw("n", n).text("error", why))?;
                prompt::no_program(why, n)
            }
            Ok(source) => {
                log.write(Line::new("submission").raw("n", n).text("source", &source))?;
                let report = setup.runner.judge(lang, &source, task)?;
                log.write(report_line(n, &report))?;
                if report.passed() {
                    tally.passed = true;
                    break;
                }
                prompt::feedback(task, &report, n)
            }
        };
    }
    Ok(tally)
}

/// One model call, logged before it is sent and when its reply is in.
fn call(run: &mut Run, n: u32, message: &str, log: &mut Log) -> Result<Reply, Error> {
    log.write(Line::new("call").raw("n", n).text("sent", message).now())?;
    let start = Instant::now();
    let reply = run.send(message)?;
    log.write(
        Line::new("reply")
            .raw("n", n)
            .text("text", &reply.text)
            .raw("input", reply.tokens.input)
            .raw("output", reply.tokens.output)
            .raw("cut_off", reply.cut_off)
            .raw("ms", start.elapsed().as_millis())
            .now(),
    )?;
    Ok(reply)
}

/// The program in a reply: its one fenced code block. Fences are backticks only. An opening fence
/// may name a language; a closing fence is backticks only, at least as many as opened it; a block
/// left open runs to the end of the reply.
pub fn program(reply: &str) -> Result<String, &'static str> {
    let mut blocks: Vec<String> = vec![];
    let mut open: Option<usize> = None;
    for line in reply.lines() {
        let trimmed = line.trim();
        let ticks = trimmed.len() - trimmed.trim_start_matches('`').len();
        match open {
            None if ticks >= 3 => {
                open = Some(ticks);
                blocks.push(String::new());
            }
            None => {}
            Some(n) if ticks >= n && ticks == trimmed.len() => open = None,
            Some(_) => {
                let body = blocks.last_mut().expect("a block is open");
                *body += line;
                body.push('\n');
            }
        }
    }
    match blocks.len() {
        0 => Err("the reply has no code block"),
        1 => Ok(blocks.remove(0)),
        _ => Err("the reply has more than one code block"),
    }
}

/// The judge's report: the build's outcome with its full text, and every case, hidden ones
/// included.
fn report_line(n: u32, report: &Report) -> Line {
    let line = Line::new("report")
        .raw("n", n)
        .raw("passed", report.passed());
    let line = match &report.build {
        Ok(()) => line.text("build", "ok"),
        Err(text) => line.text("build", "failed").text("text", text).raw(
            "codes",
            json::array(text.lines().filter_map(code).map(quote)),
        ),
    };
    let visible = report.visible.iter().map(|c| case_json(c, false));
    let hidden = report.hidden.iter().map(|c| case_json(c, true));
    line.raw("cases", json::array(visible.chain(hidden)))
}

/// The diagnostic code of a compiler error line: `error[code]` from Paka and Rust, `error TSnnnn`
/// from tsc. Only at the start of the line or after a location without spaces, so source lines the
/// compiler quotes do not count.
fn code(line: &str) -> Option<&str> {
    let rest = match line.split_once(": error") {
        Some((at, rest)) if !at.contains(char::is_whitespace) => rest,
        _ => line.strip_prefix("error")?,
    };
    if let Some(rest) = rest.strip_prefix('[') {
        return rest.split_once(']').map(|(code, _)| code);
    }
    let (code, _) = rest.strip_prefix(' ')?.split_once(':')?;
    let digits = code.strip_prefix("TS")?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some(code)
}

fn case_json(case: &CaseResult, hidden: bool) -> String {
    let verdict = match case.verdict {
        Verdict::Pass => "pass",
        Verdict::Timeout => "timeout",
        Verdict::Stdout => "stdout",
        Verdict::Status => "status",
        Verdict::Files => "files",
    };
    json::object([
        ("name", quote(&case.name)),
        ("hidden", hidden.to_string()),
        ("verdict", quote(verdict)),
        (
            "status",
            case.status.map_or("null".into(), |s| s.to_string()),
        ),
        ("stdout", quote(&String::from_utf8_lossy(&case.stdout))),
        ("stderr", quote(&String::from_utf8_lossy(&case.stderr))),
    ])
}

#[cfg(test)]
mod tests {
    use super::code;

    #[test]
    fn codes_come_from_error_lines_only() {
        let text = "main.paka:3:14: error[bad-state]: the state is not a record\n\
                    error[E0308]: mismatched types\n\
                    5 |     let s = \"error[E0599]: not this\";\n\
                    main.ts(3,5): error TS2322: Type 'string' is not assignable\n\
                    error TS5023: Unknown compiler option\n\
                    main.paka:1:1: warning[unused-name]: unused\n\
                    error: aborting due to 1 previous error";
        let codes: Vec<&str> = text.lines().filter_map(code).collect();
        assert_eq!(codes, ["bad-state", "E0308", "TS2322", "TS5023"]);
    }
}
