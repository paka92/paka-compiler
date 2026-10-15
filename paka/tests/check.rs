//! `paka check` against the hand-written expectations in `tests/errors/<name>/`:
//! `expected.txt` for the text form, `expected.json` for `--json`; and the `unused-name` warning
//! of docs/decisions/M2-02.md section 5.

#[allow(dead_code)]
mod common;

use common::Scratch;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/errors");

fn cases() -> Vec<PathBuf> {
    let mut dirs: Vec<_> = fs::read_dir(ROOT)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    dirs
}

fn paka(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_paka"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn assert_rejected(out: &Output, want: &str) {
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(stderr(out), want);
}

#[test]
fn text_form_matches_the_expected_file() {
    for dir in cases() {
        let want = fs::read_to_string(dir.join("expected.txt")).unwrap();
        assert_rejected(&paka(&dir, &["check", "main.paka"]), &want);
    }
}

#[test]
fn json_form_matches_the_expected_file() {
    for dir in cases() {
        let want = fs::read_to_string(dir.join("expected.json")).unwrap();
        assert_rejected(&paka(&dir, &["check", "main.paka", "--json"]), &want);
    }
}

#[test]
fn first_text_line_is_what_run_prints() {
    for dir in cases() {
        let run = stderr(&paka(&dir, &["run", "main.paka"]));
        let check = stderr(&paka(&dir, &["check", "main.paka"]));
        assert_eq!(check.lines().next().map(|l| format!("{l}\n")), Some(run));
    }
}

/// An accepted program with no hole and no unused name.
#[test]
fn accepted_program_prints_nothing_and_exits_0() {
    let used = "enum Msg { Start }\n\nfn update(m: Msg) -> List<Effect<Msg>> {\n  match m {\n    \
                Start => [Exit]\n  }\n}\n";
    let scratch = program_in("used", used);
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/check");
    for dir in [scratch.path(""), cases.join("empty_file")] {
        for flags in [&[][..], &["--json"]] {
            let out = paka(&dir, &[&["check", "main.paka"], flags].concat());
            assert_eq!(out.status.code(), Some(0), "{dir:?}");
            assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{dir:?}");
        }
    }
}

/// M2-15 section 4: `check/no_app` checks, with the warning for its `m` (M2-02 section 5).
#[test]
fn no_app_checks_with_the_warning_for_m() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/check/no_app");
    let text = "main.paka:3:11: warning[unused-name]: m is never used\n  definition: update\n  \
                bytes: 30..31\n  text: \"m\"\n";
    let out = paka(&dir, &["check", "main.paka"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert_eq!(stderr(&out), text);
}

#[test]
fn light_warns_that_m_is_never_used() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let text = "light.paka:3:11: warning[unused-name]: m is never used\n  definition: update\n  \
                bytes: 30..31\n  text: \"m\"\n";
    let json = r#"{"kind":"unused-name","definition":"update","message":"m is never used","file":"light.paka","start":30,"end":31,"line":3,"column":11,"text":"m","expected":null,"found":null,"fix":null}"#;
    for (flags, want) in [
        (&[][..], text.to_string()),
        (&["--json"], format!("{json}\n")),
    ] {
        let out = paka(&dir, &[&["check", "light.paka"], flags].concat());
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty());
        assert_eq!(stderr(&out), want);
    }
}

/// Unused names around a hole that is never reached: `pick`'s `unused` and `spare`, then the hole,
/// then `update`'s `m`.
const UNUSED: &str = "enum Msg { Start }

fn pick(n: Int, unused: Int) -> Int {
  let spare = 1
  n + ??more
}

fn update(m: Msg) -> List<Effect<Msg>> {
  [Exit]
}

app t {
  start:  Start
  update: update
  grants: []
}
";

fn program_in(name: &str, source: &str) -> Scratch {
    let dir = Scratch::new(name);
    fs::write(dir.path("main.paka"), source).unwrap();
    dir
}

#[test]
fn warnings_and_notes_come_in_position_order() {
    let dir = program_in("unused-order", UNUSED);
    let out = paka(&dir.path(""), &["check", "main.paka"]);
    assert_eq!(out.status.code(), Some(0));
    let heads: Vec<_> = (stderr(&out).lines())
        .filter(|l| !l.starts_with("  "))
        .map(String::from)
        .collect();
    let want = [
        "main.paka:3:17: warning[unused-name]: unused is never used",
        "main.paka:4:7: warning[unused-name]: spare is never used",
        "main.paka:5:7: note[hole]: ??more takes Int",
        "main.paka:8:11: warning[unused-name]: m is never used",
    ];
    assert_eq!(heads, want);
    let out = paka(&dir.path(""), &["check", "main.paka", "--json"]);
    let kinds: Vec<_> = (stderr(&out).lines())
        .map(|l| l.split(',').next().unwrap().to_string())
        .collect();
    let kind = |k: &str| format!("{{\"kind\":\"{k}\"");
    let want = ["unused-name", "unused-name", "hole", "unused-name"].map(kind);
    assert_eq!(kinds, want);
}

#[test]
fn only_check_prints_warnings() {
    let dir = program_in("unused-run", UNUSED);
    for command in ["run", "examples"] {
        let out = paka(&dir.path(""), &[command, "main.paka"]);
        assert_eq!(out.status.code(), Some(0), "{command}");
        assert!(out.stderr.is_empty(), "{command}: {}", stderr(&out));
    }
}

#[test]
fn an_error_hides_every_warning() {
    let source = UNUSED.replace("let spare = 1", "let spare = nope");
    let dir = program_in("unused-error", &source);
    let out = paka(&dir.path(""), &["check", "main.paka"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stderr(&out);
    assert!(
        text.starts_with("main.paka:4:15: error[unknown-name]"),
        "{text}"
    );
    assert!(!text.contains("warning"), "{text}");
}

#[test]
fn unreadable_file_is_a_json_error_without_positions() {
    let out = paka(Path::new(ROOT), &["check", "no/such.paka", "--json"]);
    let text = stderr(&out);
    let head = r#"{"kind":"file-unreadable","definition":null,"message":""#;
    let tail = r#"","file":"no/such.paka","start":null,"end":null,"line":null,"column":null,"text":null,"expected":null,"found":null,"fix":null}"#;
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text.starts_with(head) && text.ends_with(&format!("{tail}\n")),
        "{text}"
    );
}

#[test]
fn check_usage_is_checked() {
    for args in [
        vec!["check"],
        vec!["check", "a", "b"],
        vec!["check", "--json", "a"],
    ] {
        assert_eq!(
            paka(Path::new(ROOT), &args).status.code(),
            Some(2),
            "{args:?}"
        );
    }
}
