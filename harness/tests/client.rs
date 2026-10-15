//! The client against a fake `claude` that prints recorded stream-json lines (tests/streams).

use harness::Error;
use harness::client::{Config, Run};
use harness::usage::{Ledger, Tokens};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FAKE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_claude/claude");
const STREAMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/streams");

const TURN_1: &str = "Here is the file.\n```paka\napp main\n```";
const TURN_2: &str = "Fixed.\n```paka\napp main2\n```";
const BIG: Tokens = tokens(1_000_000, 1_000_000);

const fn tokens(input: u64, output: u64) -> Tokens {
    Tokens { input, output }
}

/// A scratch directory holding the usage file and the run's working directory with its script.
struct Rig {
    base: PathBuf,
    config: Config,
    ledger: Ledger,
}

impl Rig {
    fn new(name: &str, stream: &str, caps: Tokens) -> Rig {
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
            call_limit: Duration::from_secs(30),
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
    assert!(!first.cut_off);
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
fn a_failed_result_voids_the_run_and_its_tokens_are_counted() {
    let rig = Rig::new("failed", "failed.txt", BIG);
    let mut run = rig.start().unwrap();
    let err = run.send("hello").unwrap_err();
    assert!(
        matches!(&err, Error::Provider(m) if m.contains("529")),
        "{err:?}"
    );
    assert_eq!(rig.used(), tokens(27, 9));
    assert_eq!(rig.run_file("stdin").lines().count(), 1);
}

#[test]
fn an_error_during_execution_voids_the_run() {
    let rig = Rig::new("execution", "error_during_execution.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Provider(_)), "{err:?}");
    assert_eq!(rig.used(), tokens(0, 0));
}

#[test]
fn a_failed_result_without_usage_counts_nothing() {
    let rig = Rig::new("no-usage", "failed_no_usage.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Provider(_)), "{err:?}");
    assert_eq!(rig.used(), tokens(0, 0));
}

#[test]
fn after_an_error_the_run_reads_and_writes_nothing() {
    let rig = Rig::new("poison", "failed.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap_err();
    for _ in 0..2 {
        let err = run.send("again").unwrap_err();
        assert!(matches!(err, Error::Voided), "{err:?}");
    }
    assert_eq!(rig.run_file("stdin").lines().count(), 1);
    assert_eq!(rig.used(), tokens(27, 9));
}

#[test]
fn a_run_past_the_cap_is_refused_before_the_process_starts() {
    for used in [
        r#"{"input":1000000,"output":0}"#,
        r#"{"input":0,"output":1000000}"#,
    ] {
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
        Tokens {
            input: 1203,
            output: 1_000_000,
        },
        Tokens {
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
    for text in [
        "",
        "garbage",
        r#"{"input":1}"#,
        r#"{"input":"1","output":2}"#,
        r#"{"input":-1,"output":2}"#,
    ] {
        fs::write(rig.base.join("usage.json"), text).unwrap();
        assert!(
            matches!(rig.ledger.used(), Err(Error::Malformed(_))),
            "{text:?}"
        );
        assert!(
            matches!(rig.start().err(), Some(Error::Malformed(_))),
            "{text:?}"
        );
    }
}

#[test]
fn compaction_voids_the_run() {
    for stream in ["compact_boundary.txt", "compacting.txt"] {
        let rig = Rig::new("compact", stream, BIG);
        let err = rig.start().unwrap().send("hello").unwrap_err();
        assert!(matches!(err, Error::Compacted), "{stream}: {err:?}");
        assert_eq!(rig.used(), tokens(1203, 50), "{stream}");
    }
}

#[test]
fn another_model_voids_the_run_but_its_tokens_are_counted() {
    let rig = Rig::new("model", "wrong_model.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    match err {
        Error::WrongModel { expected, got } => {
            assert_eq!(
                (expected.as_str(), got.as_str()),
                ("claude-sonnet-5-5", "claude-haiku-5-5")
            );
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
    ] {
        let rig = Rig::new("shape", stream, BIG);
        let err = rig.start().unwrap().send("hello").unwrap_err();
        assert!(matches!(err, Error::Malformed(_)), "{stream}: {err:?}");
        assert_eq!(rig.used(), tokens(0, 0), "{stream}");
    }
}

#[test]
fn a_reply_without_an_assistant_message_is_an_error_but_is_counted() {
    let rig = Rig::new("no-model", "no_model.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Malformed(_)), "{err:?}");
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn one_assistant_message_from_another_model_voids_the_turn() {
    let rig = Rig::new("mixed", "mixed_models.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(
        matches!(&err, Error::WrongModel { got, .. } if got == "claude-haiku-5-5"),
        "{err:?}"
    );
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn a_refusal_voids_the_run_and_is_counted() {
    let rig = Rig::new("refusal", "refusal.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Refused), "{err:?}");
    assert_eq!(rig.used(), tokens(1203, 50));
}

#[test]
fn a_reply_cut_at_the_token_limit_is_flagged() {
    let rig = Rig::new("cut-off", "cut_off.txt", BIG);
    let reply = rig.start().unwrap().send("hello").unwrap();
    assert!(reply.cut_off);
    assert_eq!(reply.text, TURN_1);
}

#[test]
fn a_key_source_other_than_none_is_refused() {
    let rig = Rig::new("api-key", "api_key.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(
        matches!(&err, Error::ApiKey(s) if s == "ANTHROPIC_API_KEY"),
        "{err:?}"
    );
}

#[test]
fn the_child_gets_a_cleared_environment() {
    const INNER: &str = "HARNESS_ENV_CHECK";
    if std::env::var_os(INNER).is_none() {
        let this = std::env::current_exe().unwrap();
        let status = std::process::Command::new(this)
            .args([
                "--exact",
                "the_child_gets_a_cleared_environment",
                "--nocapture",
            ])
            .env(INNER, "1")
            .env("ANTHROPIC_API_KEY", "not-a-key")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let rig = Rig::new("env", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap();
    run.close().unwrap();
    let env = rig.run_file("env");
    assert!(!env.contains("ANTHROPIC_API_KEY"), "{env}");
    assert!(!env.contains(INNER), "{env}");
    assert!(env.lines().any(|l| l.starts_with("PATH=")), "{env}");
}

#[test]
fn the_childs_standard_error_is_kept_in_the_run_directory() {
    let rig = Rig::new("stderr", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap();
    run.close().unwrap();
    assert_eq!(rig.run_file("stderr.log"), "fake-stderr\n");
}

#[test]
fn dropping_a_run_kills_and_reaps_the_process() {
    let rig = Rig::new("drop", "ok.txt", BIG);
    let mut run = rig.start().unwrap();
    run.send("hello").unwrap();
    let pid = rig.run_file("pid").trim().to_string();
    assert!(Path::new(&format!("/proc/{pid}")).exists());
    drop(run);
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn closing_a_run_whose_process_exits_with_an_error_fails() {
    let mut rig = Rig::new("close-status", "ok.txt", BIG);
    rig.config.program = "false".into();
    let err = rig.start().unwrap().close().unwrap_err();
    assert!(matches!(err, Error::Died), "{err:?}");
}

#[test]
fn a_process_that_exits_before_reading_is_a_dead_process() {
    let mut rig = Rig::new("early-exit", "ok.txt", BIG);
    rig.config.program = "false".into();
    let mut run = rig.start().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    let err = run.send("hello").unwrap_err();
    assert!(matches!(err, Error::Died), "{err:?}");
}

#[test]
fn a_dead_process_is_an_error() {
    let rig = Rig::new("died", "died.txt", BIG);
    let err = rig.start().unwrap().send("hello").unwrap_err();
    assert!(matches!(err, Error::Died), "{err:?}");
}

#[test]
fn a_write_the_process_never_reads_is_bounded_by_the_call_limit() {
    let mut rig = Rig::new("blocked-write", "ok.txt", BIG);
    fs::write(rig.config.dir.join("script"), "@sleep 30\n").unwrap();
    rig.config.call_limit = Duration::from_millis(500);
    let mut run = rig.start().unwrap();
    let start = Instant::now();
    // Far more than a pipe holds, so the write blocks until the process reads.
    let err = run.send(&"x".repeat(1 << 20)).unwrap_err();
    assert!(matches!(err, Error::TimedOut), "{err:?}");
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_missing_program_is_an_error() {
    let mut rig = Rig::new("missing", "ok.txt", BIG);
    rig.config.program = rig.base.join("no-such-claude");
    assert!(matches!(rig.start().err(), Some(Error::Io(_))));
}

#[test]
fn the_first_call_guard_allows_fifty_tokens_on_the_first_call_only() {
    for (expected, ok) in [
        (1203, true),
        (1253, true),
        (1153, true),
        (1254, false),
        (1152, false),
    ] {
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
