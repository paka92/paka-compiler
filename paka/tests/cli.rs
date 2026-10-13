use std::process::{Command, Output};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n";

fn paka(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_paka"))
        .args(args)
        .output()
        .unwrap()
}

fn assert_usage(args: &[&str]) {
    let out = paka(args);
    assert_eq!(out.status.code(), Some(2), "args {args:?}");
    assert!(out.stdout.is_empty(), "args {args:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), USAGE, "args {args:?}");
}

#[test]
fn bad_arguments_print_usage_and_exit_2() {
    assert_usage(&[]);
    assert_usage(&["run"]);
    assert_usage(&["run", "a.paka", "b.paka"]);
    assert_usage(&["build", "a.paka"]);
    assert_usage(&["build", "a.paka", "-o"]);
    assert_usage(&["build", "a.paka", "out", "-o"]);
    assert_usage(&["build", "a.paka", "-o", "out", "extra"]);
    assert_usage(&["fmt", "a.paka"]);
}

fn assert_unreadable(args: &[&str], path: &str) {
    let out = paka(args);
    assert_eq!(out.status.code(), Some(1), "args {args:?}");
    assert!(out.stdout.is_empty(), "args {args:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let prefix = format!("{path}: error[file-unreadable]: ");
    let rest = stderr
        .strip_prefix(&prefix)
        .unwrap_or_else(|| panic!("{stderr:?}"));
    let message = rest
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("{stderr:?}"));
    assert!(!message.is_empty() && !message.contains('\n'), "{stderr:?}");
}

#[test]
fn missing_file_is_one_line_and_exit_1() {
    assert_unreadable(&["run", "no/such/file.paka"], "no/such/file.paka");
    let args = ["build", "no/such/file.paka", "-o", "out"];
    assert_unreadable(&args, "no/such/file.paka");
}

#[test]
fn directory_is_unreadable() {
    let dir = env!("CARGO_MANIFEST_DIR");
    assert_unreadable(&["run", dir], dir);
}

#[test]
fn lexical_reject_is_one_diagnostic_and_exit_1() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/cases-pending/lex/comment"
    );
    for args in [
        vec!["run", "main.paka"],
        vec!["build", "main.paka", "-o", "unused"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_paka"))
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "args {args:?}");
        assert!(out.stdout.is_empty(), "args {args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        let rest = stderr.strip_prefix("main.paka:1:20: error[unsupported-comment]: ");
        let message = rest.and_then(|r| r.strip_suffix('\n'));
        assert!(
            message.is_some_and(|m| !m.is_empty() && !m.contains('\n')),
            "{stderr:?}"
        );
    }
}

#[test]
fn readable_file_is_silent_and_exit_0() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");
    for args in [vec!["run", file], vec!["build", file, "-o", "unused"]] {
        let out = paka(&args);
        assert_eq!(out.status.code(), Some(0), "args {args:?}");
        assert!(
            out.stdout.is_empty() && out.stderr.is_empty(),
            "args {args:?}"
        );
    }
}
