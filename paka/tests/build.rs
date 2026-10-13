//! `paka build` end to end: spec/m1.md sections 10 and 13, docs/abi/m1.md section 9.

mod common;

use common::{LIGHT, PAKA, Scratch, build_native, build_runtime};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

const NO_APP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests/cases/check/no_app/main.paka"
);

/// One line on standard error: `<path>: error[<code>]: <message>`; nothing else written; status 1.
fn assert_tool_error(out: &Output, path: &str, code: &str) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    let prefix = format!("{path}: error[{code}]: ");
    let message = stderr
        .strip_prefix(&prefix)
        .and_then(|r| r.strip_suffix('\n'));
    assert!(
        message.is_some_and(|m| !m.is_empty() && !m.contains('\n')),
        "{stderr:?}"
    );
    assert!(out.stdout.is_empty());
    assert_eq!(out.status.code(), Some(1));
}

/// Runs `paka_exe build source -o output` with `PATH` set to `path`.
fn build_with(paka_exe: &Path, source: &str, output: &Path, path: &str) -> Output {
    Command::new(paka_exe)
        .args(["build", source, "-o"])
        .arg(output)
        .env("PATH", path)
        .output()
        .unwrap()
}

#[test]
fn light_builds_and_runs() {
    let dir = Scratch::new("light");
    let out = build_native(LIGHT, &dir.path("light"));
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert_eq!(dir.entries(), ["light"], "temporary file left behind");
    let run = Command::new(dir.path("light")).output().unwrap();
    assert_eq!(run.stdout, b"let there be light\n");
    assert!(run.stderr.is_empty());
    assert_eq!(run.status.code(), Some(0));
}

#[test]
fn rejected_program_behaves_as_run_and_writes_nothing() {
    build_runtime();
    for source in [NO_APP, "no/such/file.paka"] {
        let dir = Scratch::new("rejected");
        let want = Command::new(PAKA).args(["run", source]).output().unwrap();
        let got = build_with(Path::new(PAKA), source, &dir.path("out"), "");
        assert_eq!(
            (got.status, &got.stdout, &got.stderr),
            (want.status, &want.stdout, &want.stderr)
        );
        assert_eq!(got.status.code(), Some(1));
        assert!(dir.entries().is_empty(), "{source}");
    }
}

#[test]
fn missing_clang_is_one_diagnostic() {
    build_runtime();
    let dir = Scratch::new("no-clang");
    let out = build_with(Path::new(PAKA), LIGHT, &dir.path("out"), "");
    assert_tool_error(&out, LIGHT, "clang-missing");
    assert!(dir.entries().is_empty());
}

#[test]
fn failing_clang_is_one_diagnostic() {
    build_runtime();
    let (bin, dir) = (Scratch::new("bin"), Scratch::new("failing"));
    symlink("/bin/false", bin.path("clang")).unwrap();
    let path = bin.path("");
    let out = build_with(
        Path::new(PAKA),
        LIGHT,
        &dir.path("out"),
        path.to_str().unwrap(),
    );
    assert_tool_error(&out, LIGHT, "clang-failed");
    assert!(dir.entries().is_empty(), "left behind: {:?}", dir.entries());
}

#[test]
fn unwritable_output_is_clang_failed() {
    let dir = Scratch::new("unwritable");
    let out = build_native(LIGHT, &dir.path("no/such/dir/out"));
    assert_tool_error(&out, LIGHT, "clang-failed");
    assert!(dir.entries().is_empty());
}

#[test]
fn missing_runtime_comes_before_missing_clang() {
    let (bin, dir) = (Scratch::new("copy"), Scratch::new("no-runtime"));
    fs::hard_link(PAKA, bin.path("paka")).unwrap();
    for path in ["", "/usr/bin:/bin"] {
        let out = build_with(&bin.path("paka"), LIGHT, &dir.path("out"), path);
        assert_tool_error(&out, LIGHT, "runtime-missing");
        assert!(dir.entries().is_empty());
    }
}
