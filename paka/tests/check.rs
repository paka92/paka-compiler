//! `paka check` against the hand-written expectations in `tests/errors/<name>/`:
//! `expected.txt` for the text form, `expected.json` for `--json`.

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

#[test]
fn accepted_program_prints_nothing_and_exits_0() {
    let dir = Path::new(ROOT).parent().unwrap().join("examples");
    for args in [
        &["check", "light.paka"][..],
        &["check", "light.paka", "--json"],
    ] {
        let out = paka(&dir, args);
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
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
