//! The trial loop against the fake `claude` (tests/fake_claude) on a tiny task (tests/echo_task).
//! Each attempt takes the next script from a queue folder; the test writes the scripts.

mod common;

use harness::Error;
use harness::json::{self, Value, quote};
use harness::runner::Lang;
use harness::trial::{self, Setup, Spec};
use harness::usage::{Ledger, Tokens};
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const FAKE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_claude/claude");
const TASK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/echo_task");
const MODEL: &str = "claude-sonnet-5-5";
const INIT: &str =
    r#"{"type":"system","subtype":"init","model":"claude-sonnet-5-5","apiKeySource":"none"}"#;
/// Input tokens of every scripted turn: 10 plain, 100 cache write, 0 cache read.
const INPUT: u64 = 110;

const GOOD: &str = r#"type Blank { n: Int = 0 }
enum Msg { Start(word: String) }
fn update(s: inout Blank, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(word) => {
      if word == "" {
        [Print("error: empty"), ExitWith(20)]
      } else {
        [Print(word), Exit]
      }
    }
  }
}
app echo {
  state: Blank
  start: Start
  update: update
  grants: [Console]
}
"#;

fn bad() -> String {
    GOOD.replace("state: Blank", "state: Int")
}

fn wrong() -> String {
    GOOD.replace("Print(word)", "Print(\"no\")")
}

fn reply(source: &str) -> String {
    format!("Here it is.\n```paka\n{source}```\n")
}

/// One scripted turn: the assistant message and the result line.
fn turn(text: &str, output: u64, stop: &str) -> String {
    let text = quote(text);
    let usage = format!(
        r#"{{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":0,"output_tokens":{output}}}"#
    );
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"model\":\"{MODEL}\",\"content\":[{{\"type\":\"text\",\"text\":{text}}}],\"usage\":{usage}}}}}\n\
         {{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":{text},\"stop_reason\":\"{stop}\",\"usage\":{usage}}}"
    )
}

fn ok(text: &str, output: u64) -> String {
    turn(text, output, "end_turn")
}

/// A test's folder: the queue of scripts, the wrapper that points the fake at it, the runs.
struct Rig {
    base: PathBuf,
    setup: Setup,
    /// Scripts queued so far, which numbers the next one.
    queued: Cell<u32>,
}

impl Rig {
    fn new(name: &str, caps: Tokens) -> Rig {
        let base = common::scratch(&format!("trial-{name}"));
        Rig::at(base, caps)
    }

    fn at(base: PathBuf, caps: Tokens) -> Rig {
        let queue = base.join("queue");
        fs::create_dir_all(&queue).unwrap();
        let text = format!(
            "#!/bin/sh\nQUEUE='{}' exec '{FAKE}' \"$@\"\n",
            queue.display()
        );
        common::put(&base, "claude", &text);
        let wrapper = base.join("claude");
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        let setup = Setup {
            claude: wrapper,
            model: MODEL.into(),
            effort: "medium".into(),
            reference: "the reference".into(),
            runner: common::runner(),
            ledger: Ledger::new(base.join("usage.json"), caps),
            call_limit: Duration::from_secs(30),
            root: base.join("runs"),
        };
        Rig {
            base,
            setup,
            queued: Cell::new(0),
        }
    }

    /// Queues one attempt's script: the init line, then one block per turn.
    fn queue(&self, turns: &[String]) {
        let n = self.queued.replace(self.queued.get() + 1);
        let script = [INIT.to_string()]
            .iter()
            .chain(turns)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n");
        fs::write(self.base.join("queue").join(n.to_string()), script + "\n").unwrap();
    }

    fn run(&self, reps: u32) -> Result<Tokens, Error> {
        trial::run_batch(&self.setup, &specs(reps))
    }

    fn attempt(&self, rep: u32, attempt: u32) -> PathBuf {
        self.setup
            .root
            .join(format!("echo_task-Paka-{rep}"))
            .join(attempt.to_string())
    }
}

fn specs(reps: u32) -> Vec<Spec> {
    (1..=reps)
        .map(|rep| Spec {
            task: TASK.into(),
            lang: Lang::Paka,
            rep,
            first_call_input: None,
        })
        .collect()
}

fn big() -> Tokens {
    Tokens {
        input: 1_000_000,
        output: 1_000_000,
    }
}

fn events(dir: &Path) -> Vec<Value> {
    fs::read_to_string(dir.join("log"))
        .unwrap()
        .lines()
        .map(|line| json::parse(line).unwrap())
        .collect()
}

fn names(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .map(|e| e.get("event").and_then(Value::as_str).unwrap())
        .collect()
}

fn num(event: &Value, key: &str) -> u64 {
    event.get(key).and_then(Value::as_u64).unwrap()
}

fn text<'a>(event: &'a Value, key: &str) -> &'a str {
    event.get(key).and_then(Value::as_str).unwrap()
}

fn last(dir: &Path) -> Value {
    events(dir).pop().unwrap()
}

/// The user turns the fake read, one per line.
fn sent(dir: &Path) -> Vec<String> {
    let text = fs::read_to_string(dir.join("claude/stdin")).unwrap();
    text.lines()
        .map(|line| {
            let line = json::parse(line).unwrap();
            let message = line.get("message").unwrap().get("content").unwrap();
            let Value::Array(parts) = message else {
                panic!("content is not an array")
            };
            parts[0].get("text").and_then(Value::as_str).unwrap().into()
        })
        .collect()
}

#[test]
fn counts_edits_tokens_and_logs_everything() {
    let rig = Rig::new("counts", big());
    rig.queue(&[
        ok(&reply(&bad()), 100),
        ok("I am not sure.", 20),
        ok(&reply(&wrong()), 30),
        ok(&reply(GOOD), 40),
    ]);
    let used = rig.run(1).unwrap();
    assert_eq!(
        used,
        Tokens {
            input: 4 * INPUT,
            output: 190
        }
    );
    let dir = rig.attempt(1, 1);
    let log = events(&dir);
    assert_eq!(
        names(&log),
        [
            "start",
            "call",
            "reply",
            "submission",
            "report",
            "call",
            "reply",
            "submission",
            "call",
            "reply",
            "submission",
            "report",
            "call",
            "reply",
            "submission",
            "report",
            "end",
        ]
    );
    let end = log.last().unwrap();
    assert_eq!(end.get("passed"), Some(&Value::Bool(true)));
    assert_eq!(end.get("first_try"), Some(&Value::Bool(false)));
    assert_eq!(num(end, "submissions"), 4);
    assert_eq!(num(end, "edits"), 3);
    assert_eq!(num(end, "input"), 4 * INPUT);
    assert_eq!(num(end, "output"), 190);
    assert_eq!(num(end, "cut_off"), 0);
    for event in log
        .iter()
        .filter(|e| ["call", "reply"].contains(&text(e, "event")))
    {
        assert!(num(event, "at") > 0);
    }
    for event in log.iter().filter(|e| text(e, "event") == "reply") {
        event.get("ms").and_then(Value::as_u64).unwrap();
    }
    assert!(text(&log[0], "system").contains("You have 8 submissions."));
    assert!(text(&log[0], "system").ends_with("the reference"));
    assert_eq!(text(&log[4], "build"), "failed");
    assert_eq!(
        log[4].get("codes"),
        Some(&Value::Array(vec![Value::Str("bad-state".into())]))
    );
    assert_eq!(text(&log[7], "error"), "the reply has no code block");
    assert_eq!(text(&log[14], "source"), GOOD);

    // What the model saw: the statement and visible case, then the feedback, nothing hidden.
    let turns = sent(&dir);
    assert_eq!(turns.len(), 4);
    assert!(turns[0].starts_with("# Echo"));
    assert!(turns[0].contains("Arguments: \"hello\""));
    assert!(turns[1].starts_with("build: failed\n"));
    assert!(turns[1].contains("error[bad-state]"));
    assert!(turns[1].ends_with("submission 1 of 8\n"));
    assert_eq!(
        turns[2],
        "no program: the reply has no code block\nsubmission 2 of 8\n"
    );
    assert!(turns[3].starts_with("build: ok\nvisible case 1: FAIL\n"));
    assert!(turns[3].contains("hidden cases: 1 of 2 pass (1 wrong output)\n"));
    assert!(turns[3].ends_with("submission 3 of 8\n"));
    for file in ["stdin", "args"] {
        let seen = fs::read_to_string(dir.join("claude").join(file)).unwrap();
        assert!(!seen.contains("zebra-hidden-case"), "{file}");
        assert!(!seen.contains("walrus-working-notes"), "{file}");
    }

    // The raw stream: every line sent and received, in order.
    let stream = fs::read_to_string(dir.join("claude/stream.log")).unwrap();
    let users = stream
        .lines()
        .filter(|l| l.starts_with(r#"{"type":"user""#));
    assert_eq!(users.count(), 4);
    assert_eq!(
        stream.lines().filter(|l| l.contains("\"result\"")).count(),
        4
    );
    assert!(stream.lines().any(|l| l == INIT));
}

#[test]
fn first_try_counts_no_edit() {
    let rig = Rig::new("first-try", big());
    rig.queue(&[ok(&reply(GOOD), 40)]);
    rig.run(1).unwrap();
    let end = last(&rig.attempt(1, 1));
    assert_eq!(end.get("first_try"), Some(&Value::Bool(true)));
    assert_eq!(num(&end, "edits"), 0);
}

#[test]
fn stops_after_eight_submissions() {
    let rig = Rig::new("eight", big());
    rig.queue(&vec![ok(&reply(&bad()), 10); 9]);
    rig.run(1).unwrap();
    let dir = rig.attempt(1, 1);
    let end = last(&dir);
    assert_eq!(text(&end, "event"), "end");
    assert_eq!(end.get("passed"), Some(&Value::Bool(false)));
    assert_eq!(num(&end, "submissions"), 8);
    // A run that never passes counts as cap plus one.
    assert_eq!(num(&end, "edits"), 8);
    assert_eq!(num(&end, "output"), 80);
    assert_eq!(sent(&dir).len(), 8);
}

#[test]
fn passes_on_the_eighth_submission() {
    let rig = Rig::new("eighth", big());
    let mut turns = vec![ok(&reply(&bad()), 10); 7];
    turns.push(ok(&reply(GOOD), 40));
    rig.queue(&turns);
    rig.run(1).unwrap();
    let end = last(&rig.attempt(1, 1));
    assert_eq!(end.get("passed"), Some(&Value::Bool(true)));
    assert_eq!(num(&end, "submissions"), 8);
    assert_eq!(num(&end, "edits"), 7);
}

#[test]
fn several_code_blocks_are_a_failed_submission() {
    let rig = Rig::new("several", big());
    let two = format!("{}{}", reply(GOOD), reply(GOOD));
    rig.queue(&[ok(&two, 30), ok(&reply(GOOD), 40)]);
    rig.run(1).unwrap();
    let dir = rig.attempt(1, 1);
    let end = last(&dir);
    assert_eq!(num(&end, "submissions"), 2);
    assert_eq!(num(&end, "edits"), 1);
    assert_eq!(
        sent(&dir)[1],
        "no program: the reply has more than one code block\nsubmission 1 of 8\n"
    );
}

#[test]
fn cut_off_replies_are_failed_submissions() {
    let rig = Rig::new("cut-off", big());
    rig.queue(&[
        turn(&reply(GOOD), 50, "max_tokens"),
        ok(&reply(GOOD), trial::REPLY_LIMIT + 1),
        ok(&reply(GOOD), 40),
    ]);
    rig.run(1).unwrap();
    let dir = rig.attempt(1, 1);
    let end = last(&dir);
    assert_eq!(num(&end, "submissions"), 3);
    assert_eq!(num(&end, "edits"), 2);
    assert_eq!(num(&end, "cut_off"), 2);
    assert_eq!(num(&end, "output"), 90 + trial::REPLY_LIMIT + 1);
    assert!(sent(&dir)[1].starts_with("no program: the reply was cut off"));
}

#[test]
fn stops_at_the_usage_cap() {
    let caps = Tokens {
        input: 1_000_000,
        output: 150,
    };
    let mut rig = Rig::new("cap", caps);
    rig.queue(&vec![ok(&reply(&bad()), 100); 8]);
    let err = rig.run(2).unwrap_err();
    assert!(matches!(err, Error::CapReached { .. }), "{err}");
    let end = last(&rig.attempt(1, 1));
    assert_eq!(text(&end, "event"), "void");
    assert!(text(&end, "reason").starts_with("usage cap reached"));
    assert_eq!(end.get("fault"), Some(&Value::Bool(false)));
    assert_eq!(sent(&rig.attempt(1, 1)).len(), 2);
    assert!(!rig.attempt(1, 2).exists());
    assert!(!rig.attempt(2, 1).exists());

    // With the cap raised, the cap void does not use up an attempt: two faults still leave one.
    rig.setup.ledger = Ledger::new(rig.base.join("usage.json"), big());
    rig.queue(&[String::new()]);
    rig.queue(&[String::new()]);
    rig.queue(&[ok(&reply(GOOD), 40)]);
    rig.run(1).unwrap();
    for n in [2, 3] {
        assert_eq!(
            last(&rig.attempt(1, n)).get("fault"),
            Some(&Value::Bool(true))
        );
    }
    assert_eq!(text(&last(&rig.attempt(1, 4)), "event"), "end");
}

#[test]
fn void_runs_are_rerun() {
    let rig = Rig::new("void", big());
    rig.queue(&[String::new()]);
    rig.queue(&[format!(
        "{{\"type\":\"system\",\"subtype\":\"compact_boundary\"}}\n{}",
        ok(&reply(GOOD), 40)
    )]);
    rig.queue(&[ok(&reply(&bad()), 10), ok(&reply(GOOD), 40)]);
    rig.run(1).unwrap();
    let reason = |n| text(&last(&rig.attempt(1, n)), "reason").to_string();
    assert_eq!(reason(1), "the claude process ended");
    assert_eq!(reason(2), "the conversation was compacted");
    let end = last(&rig.attempt(1, 3));
    assert_eq!(text(&end, "event"), "end");
    // A void attempt's tokens count in the batch's usage, not in the run's.
    assert_eq!(num(&end, "output"), 50);
    assert_eq!(rig.setup.ledger.used().unwrap().output, 90);
}

#[test]
fn a_third_void_pauses() {
    let rig = Rig::new("pause", big());
    for _ in 0..3 {
        rig.queue(&[String::new()]);
    }
    rig.queue(&[ok(&reply(GOOD), 40)]);
    let err = rig.run(2).unwrap_err();
    assert!(
        matches!(&err, Error::Paused(run) if run == "echo_task-Paka-1"),
        "{err}"
    );
    assert!(!rig.attempt(1, 4).exists());
    assert!(!rig.attempt(2, 1).exists());
}

fn hang() -> String {
    format!("@sleep 5\n{}", ok(&reply(GOOD), 40))
}

#[test]
fn the_watchdog_bounds_a_call() {
    let mut rig = Rig::new("call-limit", big());
    rig.setup.call_limit = Duration::from_millis(500);
    rig.queue(&[hang()]);
    rig.queue(&[ok(&reply(GOOD), 40)]);
    let start = Instant::now();
    rig.run(1).unwrap();
    assert!(start.elapsed() < Duration::from_secs(4));
    let void = last(&rig.attempt(1, 1));
    assert_eq!(
        text(&void, "reason"),
        "the model call passed its time limit"
    );
    assert_eq!(text(&last(&rig.attempt(1, 2)), "event"), "end");
}

#[test]
fn reads_the_one_code_block() {
    assert_eq!(trial::program("a\n```\nx\n```\nb").unwrap(), "x\n");
    assert_eq!(trial::program("````rust\n```\n````").unwrap(), "```\n");
    assert_eq!(
        trial::program("```paka\nleft open\n").unwrap(),
        "left open\n"
    );
    assert!(trial::program("no code").is_err());
    assert!(trial::program("```\na\n```\n```\nb\n```").is_err());
}

const CHILD_ROOT: &str = "TRIAL_RESUME_ROOT";

fn wait_for(done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The batch the resume test kills: run 1 passes, run 2 hangs in its first call.
#[test]
#[ignore = "run by resumes_after_a_kill"]
fn resume_child() {
    let Some(base) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let rig = Rig::at(base.into(), big());
    let _ = rig.run(2);
}

#[test]
fn resumes_after_a_kill() {
    let rig = Rig::new("resume", big());
    rig.queue(&[ok(&reply(GOOD), 40)]);
    rig.queue(&[format!("@sleep 20\n{}", ok(&reply(GOOD), 40))]);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "resume_child", "--ignored", "--nocapture"])
        .env(CHILD_ROOT, &rig.base)
        .spawn()
        .unwrap();
    // Killed once run 2's process has taken its script and hangs in the first call.
    let hanging = rig.attempt(2, 1);
    wait_for(|| {
        hanging.join("claude/script").exists()
            && fs::read_to_string(hanging.join("log")).is_ok_and(|log| log.contains("\"call\""))
    });
    child.kill().unwrap();
    child.wait().unwrap();
    let orphan = fs::read_to_string(hanging.join("claude/claude.pid")).unwrap();
    let orphan = PathBuf::from(format!("/proc/{}", orphan.trim()));
    assert!(
        orphan.exists(),
        "the killed harness left its process running"
    );

    rig.queue(&[ok(&reply(GOOD), 40)]);
    rig.run(2).unwrap();
    // Resuming killed the leftover process; init reaps it.
    wait_for(|| !orphan.exists());
    // The finished run is not run again; the cut one is closed void and run again.
    assert_eq!(text(&last(&rig.attempt(1, 1)), "event"), "end");
    assert!(!rig.attempt(1, 2).exists());
    let cut = last(&rig.attempt(2, 1));
    assert_eq!(text(&cut, "event"), "void");
    assert_eq!(
        text(&cut, "reason"),
        "the harness stopped before the run ended"
    );
    assert_eq!(text(&last(&rig.attempt(2, 2)), "event"), "end");
    assert_eq!(fs::read_dir(rig.base.join("queue")).unwrap().count(), 0);
}
