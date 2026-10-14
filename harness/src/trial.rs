//! The trial loop: runs a batch of runs, each one `claude -p` conversation of up to eight
//! submissions, judged on the task's cases, with a raw log per attempt (M3-01, M3-02, M3-03).
//!
//! Folders, all plain files:
//!
//! ```text
//! <root>/<task>-<lang>-<rep>/<attempt>/log       the raw log (see `log`)
//! <root>/<task>-<lang>-<rep>/<attempt>/claude/   the process's working directory, with
//!                                                stream.log and stderr.log
//! ```
//!
//! A run whose last attempt ended is never run again. An attempt cut short by a kill is closed as
//! void when the batch resumes. A void run is run again, at most twice; a third void attempt
//! pauses the batch (M3-03 section 1.1).

use crate::Error;
use crate::client::{Config, Reply, Run};
use crate::log::{self, Line, Log, State};
use crate::prompt::{self, SUBMISSIONS};
use crate::runner::{CaseResult, Lang, Report, Runner, Verdict};
use crate::task::Task;
use crate::usage::{Ledger, Tokens};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Output tokens one reply may use, thinking included; a longer reply counts as cut off.
pub const REPLY_LIMIT: u64 = 32_000;
/// Attempts per run: the first and two reruns.
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
    /// Wall-clock limits of the watchdog.
    pub call_limit: Duration,
    pub run_limit: Duration,
    /// Where the runs' folders go.
    pub root: PathBuf,
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
}

/// Runs every run of `specs` that has not ended, in order, and gives the tokens the batch used.
/// Stops at the usage caps and when a run is void for the third time.
pub fn run_batch(setup: &Setup, specs: &[Spec]) -> Result<Tokens, Error> {
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
    let mut attempt = 1;
    while dir.join(attempt.to_string()).exists() {
        let log = dir.join(attempt.to_string()).join("log");
        match log::state(&log)? {
            State::Ended => return Ok(()),
            State::Void => {}
            State::Cut => log::close_cut(&log, CUT_SHORT)?,
        }
        attempt += 1;
    }
    let task = Task::load(&spec.task)?;
    for attempt in attempt..=ATTEMPTS {
        setup.ledger.check()?;
        if run(setup, spec, &task, &dir.join(attempt.to_string()))? {
            return Ok(());
        }
    }
    Err(Error::Paused(spec.id()))
}

/// Runs one attempt in `dir` and logs it; false when it is void. An error only when the log
/// cannot be written.
fn run(setup: &Setup, spec: &Spec, task: &Task, dir: &Path) -> Result<bool, Error> {
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
        run_limit: setup.run_limit,
    };
    let (line, ended) = match converse(setup, &config, spec.lang, task, &mut log) {
        Ok(tally) => (tally.line(), true),
        Err(e) => (Line::new("void").text("reason", &e.to_string()), false),
    };
    log.write(line.now())?;
    Ok(ended)
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
        let edits = match self.passed {
            true => self.submissions - 1,
            false => SUBMISSIONS,
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
        let source = match cut_off {
            true => Err("the reply was cut off at the reply limit"),
            false => program(&reply.text),
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

/// The program in a reply: its one fenced code block. An opening fence may name a language; a
/// closing fence is backticks only, at least as many as opened it; a block left open runs to the
/// end of the reply.
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

/// The judge's report: the build's outcome and every case, hidden ones included.
fn report_line(n: u32, report: &Report) -> Line {
    let line = Line::new("report")
        .raw("n", n)
        .raw("passed", report.passed());
    let line = match &report.build {
        Ok(()) => line.text("build", "ok"),
        Err(text) => line
            .text("build", "failed")
            .text("text", text)
            .raw("codes", codes(text)),
    };
    let cases: Vec<String> = report
        .visible
        .iter()
        .map(|c| case_json(c, false))
        .chain(report.hidden.iter().map(|c| case_json(c, true)))
        .collect();
    line.raw("cases", format!("[{}]", cases.join(",")))
}

/// The diagnostic codes in a failed build's text, written `error[code]` by Paka and Rust, as a
/// JSON array.
fn codes(text: &str) -> String {
    let found: Vec<String> = text
        .split("error[")
        .skip(1)
        .filter_map(|rest| rest.split_once(']'))
        .map(|(code, _)| crate::json::quote(code))
        .collect();
    format!("[{}]", found.join(","))
}

fn case_json(case: &CaseResult, hidden: bool) -> String {
    let verdict = match case.verdict {
        Verdict::Pass => "pass",
        Verdict::Timeout => "timeout",
        Verdict::Stdout => "stdout",
        Verdict::Status => "status",
        Verdict::Files => "files",
    };
    let status = case.status.map_or("null".into(), |s| s.to_string());
    Line::new("case")
        .text("name", &case.name)
        .raw("hidden", hidden)
        .text("verdict", verdict)
        .raw("status", status)
        .text("stdout", &String::from_utf8_lossy(&case.stdout))
        .text("stderr", &String::from_utf8_lossy(&case.stderr))
        .json()
}
