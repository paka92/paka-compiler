//! The client against a fake `claude` that prints recorded stream-json lines (tests/streams).

use harness::client::{Config, Run};
use harness::usage::{Caps, Ledger, Tokens};
use harness::Error;
use std::fs;
use std::path::{Path, PathBuf};

const FAKE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_claude/claude");
const STREAMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/streams");

const TURN_1: &str = "Here is the file.\n```paka\napp main\n```";
const TURN_2: &str = "Fixed.\n```paka\napp main2\n```";
const BIG: Caps = Caps {
    input: 1_000_000,
    output: 1_000_000,
};

fn tokens(input: u64, output: u64) -> Tokens {
    Tokens { input, output }
}

/// A scratch directory holding the usage file and the run's working directory with its script.
struct Rig {
    base: PathBuf,
    config: Config,
    ledger: Ledger,
}

impl Rig {
    fn new(name: &str, stream: &str, caps: Caps) -> Rig {
        let base = std::env::temp_dir().join(format!("harness-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let dir = base.join("run");
        fs::create_dir_all(&dir).unwrap();
        fs::copy(Path::new(STREAMS).join(stream), dir.join("script")).unwrap();
        let config = Config {
            program: FAKE.into(),
            model: "claude-sonnet-5-5".into(),
            effort: "medium".into(),
            system_prompt: "rules of the run".into(),
            dir,
            first_call_input: None,
        };
        let ledger = Ledger::new(base.join("usage.json"), caps);
        Rig {
            base,
            config,
            ledger,
        }
    }

    fn start(&self) -> Result<Run, Error> {
        Run::start(&self.config, &self.ledger)
    }

    fn used(&self) -> Tokens {
        self.ledger.used().unwrap()
    }

    fn run_file(&self, name: &str) -> String {
        fs::read_to_string(self.config.dir.join(name)).unwrap()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn reply_and_token_counts_are_read_from_the_result() {
    let rig = Rig::new("reply", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    let first = run.send("hello").unwrap();
    assert_eq!(first.text, TURN_1);
    assert_eq!(first.tokens, tokens(3 + 1000 + 200, 50));
    assert_eq!(first.model, "claude-sonnet-5-5");
    assert_eq!(first.attempts, 1);
    let second = run.send("again").unwrap();
    assert_eq!(second.text, TURN_2);
    assert_eq!(second.tokens, tokens(2 + 80 + 1200, 70));
    run.close().unwrap();
    assert_eq!(rig.used(), tokens(1203 + 1282, 120));
}

#[test]
fn every_call_carries_the_fixed_flags() {
    let rig = Rig::new("args", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap();
    run.close().unwrap();
    let want = [
        "-p",
        "--model",
        "claude-sonnet-5-5",
        "--effort",
        "medium",
        "--system-prompt",
        "rules of the run",
        "--tools",
        "",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--disable-slash-commands",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ];
    assert_eq!(rig.run_file("args").lines().collect::<Vec<_>>(), want);
}

#[test]
fn a_turn_is_written_as_one_json_line() {
    let rig = Rig::new("stdin", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("say \"hi\"\nplease").unwrap();
    run.send("two").unwrap();
    run.close().unwrap();
    let want = concat!(
        r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"say \"hi\"\nplease"}]}}"#,
        "\n",
        r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"two"}]}}"#,
        "\n",
    );
    assert_eq!(rig.run_file("stdin"), want);
}

#[test]
fn a_retried_turn_is_counted_once() {
    let rig = Rig::new("retry", "retry.txt", BIG);
    let mut run = rig.start().unwrap();
    let reply = run.send("hello").unwrap();
    assert_eq!(reply.attempts, 2);
    assert_eq!(reply.tokens, tokens(1203, 50));
    assert_eq!(rig.used(), tokens(1203, 50));
    let next = run.send("again").unwrap();
    assert_eq!(next.attempts, 1);
    assert_eq!(rig.used(), tokens(1203 + 1282, 120));
    run.close().unwrap();
    let lines = rig.run_file("stdin");
    let lines: Vec<_> = lines.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], lines[1]);
}

#[test]
fn an_error_during_execution_is_retried_too() {
    let rig = Rig::new("execution", "error_during_execution.txt", BIG);
    let reply = rig.start().unwrap().send("hello").unwrap();
    assert_eq!((reply.attempts, reply.tokens), (2, tokens(1203, 50)));
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn retries_stop_after_three_and_count_nothing() {
    let rig = Rig::new("exhausted", "exhausted.txt", BIG);
    let mut run = rig.start().unwrap();
    let err = run.send("hello").unwrap_err();
    assert!(matches!(err, Error::Provider(_)), "{err:?}");
    assert_eq!(rig.used(), tokens(0, 0));
    assert_eq!(rig.run_file("stdin").lines().count(), 4);
}

#[test]
fn a_run_past_the_cap_is_refused_before_the_process_starts() {
    for used in [r#"{"input":1000000,"output":0}"#, r#"{"input":0,"output":1000000}"#] {
        let rig = Rig::new("refused", "ok.txt", BIG);
        fs::write(rig.base.join("usage.json"), used).unwrap();
        let err = rig.start().err().unwrap();
        assert!(matches!(err, Error::CapReached { .. }), "{err:?}");
        assert!(!rig.config.dir.join("args").exists());
    }
}

#[test]
fn reaching_the_cap_mid_run_stops_the_next_call() {
    for caps in [
        Caps {
            input: 1203,
            output: 1_000_000,
        },
        Caps {
            input: 1_000_000,
            output: 50,
        },
    ] {
        let rig = Rig::new("midrun", "ok.txt", caps);
        let mut run = rig.start().unwrap();
        run.send("hello").unwrap();
        let err = run.send("again").unwrap_err();
        assert!(matches!(err, Error::CapReached { .. }), "{err:?}");
        assert_eq!(rig.run_file("stdin").lines().count(), 1);
        assert_eq!(rig.used(), tokens(1203, 50));
    }
}

#[test]
fn the_total_persists_across_processes() {
    let rig = Rig::new("persist", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap();
    run.close().unwrap();
    assert_eq!(
        fs::read_to_string(rig.base.join("usage.json")).unwrap(),
        "{\"input\":1203,\"output\":50}\n"
    );
    let again = Ledger::new(rig.base.join("usage.json"), BIG);
    assert_eq!(again.used().unwrap(), tokens(1203, 50));
    let mut run = Run::start(&rig.config, &again).unwrap();
    run.send("hello").unwrap();
    assert_eq!(again.used().unwrap(), tokens(2406, 100));
}

#[test]
fn a_missing_usage_file_is_zero_and_a_broken_one_is_an_error() {
    let rig = Rig::new("usage-file", "ok.txt", BIG);
    assert_eq!(rig.used(), tokens(0, 0));
    for text in ["", "garbage", r#"{"input":1}"#, r#"{"input":"1","output":2}"#, r#"{"input":-1,"output":2}"#] {
        fs::write(rig.base.join("usage.json"), text).unwrap();
        assert!(matches!(rig.ledger.used(), Err(Error::Malformed(_))), "{text:?}");
        assert!(matches!(rig.start().err(), Some(Error::Malformed(_))), "{text:?}");
    }
}

#[test]
fn compaction_voids_the_run() {
    for stream in ["compact_boundary.txt", "compacting.txt"] {
        let rig = Rig::new("compact", stream, BIG);
        let err = rig.start().unwrap().send("hello").unwrap_err();
        assert!(matches!(err, Error::Compacted), "{stream}: {err:?}");
    }
}

#[test]
fn another_model_voids_the_run_but_its_tokens_are_counted() {
    let rig = Rig::new("model", "wrong_model.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    match err {
        Error::WrongModel { expected, got } => {
            assert_eq!((expected.as_str(), got.as_str()), ("claude-sonnet-5-5", "claude-haiku-5-5"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn an_empty_reply_is_an_error() {
    let rig = Rig::new("empty", "empty_reply.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::EmptyReply), "{err:?}");
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn an_unknown_shape_is_an_error_and_counts_nothing() {
    for stream in [
        "missing_usage_field.txt",
        "string_usage_field.txt",
        "fraction_usage_field.txt",
        "unknown_event.txt",
        "not_json.txt",
        "no_model.txt",
    ] {
        let rig = Rig::new("shape", stream, BIG);
        let err = rig.start().unwrap().send("hello").unwrap_err();
        assert!(matches!(err, Error::Malformed(_)), "{stream}: {err:?}");
        assert_eq!(rig.used(), tokens(0, 0), "{stream}");
    }
}

#[test]
fn a_dead_process_is_an_error() {
    let rig = Rig::new("died", "died.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Died), "{err:?}");
}

#[test]
fn a_missing_program_is_an_error() {
    let mut rig = Rig::new("missing", "ok.txt", BIG);
    rig.config.program = rig.base.join("no-such-claude");
    assert!(matches!(rig.start().err(), Some(Error::Io(_))));
}

#[test]
fn the_first_call_guard_allows_fifty_tokens_on_the_first_call_only() {
    for (expected, ok) in [(1203, true), (1253, true), (1153, true), (1254, false), (1152, false)] {
        let mut rig = Rig::new("guard", "ok.txt", BIG);
        rig.config.first_call_input = Some(expected);
        let mut run = rig.start().unwrap();
        let first = run.send("hello");
        if !ok {
            let err = first.unwrap_err();
            assert!(matches!(err, Error::FirstCall { got: 1203, .. }), "{err:?}");
            assert_eq!(rig.used(), tokens(1203, 50));
            continue;
        }
        first.unwrap();
        run.send("again").unwrap();
    }
}
