//! `paka build` end to end: spec/m1.md sections 10 and 13, docs/abi/m1.md section 9.

#[allow(dead_code)]
mod common;

use common::{LIGHT, PAKA, Scratch, build_native, build_runtime};
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output};

const NO_APP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests/cases/check/no_app/main.paka"
);

const LIGHT_IR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/abi/light.ll");
const FAKE_CLANG_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_clang");
const SYSTEM_LIBS: [&str; 7] = [
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

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
    let (empty, dir) = (Scratch::new("empty-path"), Scratch::new("no-clang"));
    let path = empty.path("");
    let out = build_with(
        Path::new(PAKA),
        LIGHT,
        &dir.path("out"),
        path.to_str().unwrap(),
    );
    assert_tool_error(&out, LIGHT, "clang-missing");
    assert!(dir.entries().is_empty());
}

#[test]
fn failing_clang_is_one_diagnostic_and_leaks_nothing() {
    build_runtime();
    let dir = Scratch::new("failing");
    let out = build_with(Path::new(PAKA), LIGHT, &dir.path("out"), FAKE_CLANG_DIR);
    assert_tool_error(&out, LIGHT, "clang-failed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("fake-clang"), "{stderr:?}");
    assert!(dir.entries().is_empty(), "left behind: {:?}", dir.entries());
}

#[test]
fn clang_gets_the_documents_command() {
    let archive = common::runtime_archive();
    let (log, dir) = (Scratch::new("log"), Scratch::new("args"));
    let (output, log_file) = (dir.path("out"), log.path("args"));
    let out = Command::new(PAKA)
        .args(["build", LIGHT, "-o"])
        .arg(&output)
        .env("PATH", FAKE_CLANG_DIR)
        .env("FAKE_CLANG_LOG", &log_file)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert!(dir.entries().is_empty(), "temporary file left behind");
    let recorded = fs::read_to_string(log_file).unwrap();
    let args: Vec<_> = recorded.lines().collect();
    let output = output.to_str().unwrap();
    let ir = args[1]
        .strip_prefix(output)
        .and_then(|r| r.strip_prefix('.'));
    let pid = ir.and_then(|r| r.strip_suffix(".ll"));
    assert!(
        pid.is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit())),
        "{args:?}"
    );
    let archive = fs::canonicalize(archive).unwrap();
    assert_eq!(fs::canonicalize(args[2]).unwrap(), archive);
    assert_eq!(args[0], "-O2");
    assert_eq!(args[3..10], SYSTEM_LIBS);
    assert_eq!(args[10..], ["-o", output]);
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

#[test]
fn rejected_program_comes_before_missing_runtime() {
    let (bin, dir) = (
        Scratch::new("copy-rejected"),
        Scratch::new("rejected-first"),
    );
    fs::hard_link(PAKA, bin.path("paka")).unwrap();
    let out = build_with(&bin.path("paka"), NO_APP, &dir.path("out"), "");
    assert_tool_error(&out, NO_APP, "no-app");
    assert!(dir.entries().is_empty());
}

#[test]
fn abi_document_ir_links_and_runs() {
    let archive = common::runtime_archive();
    let dir = Scratch::new("abi-ir");
    let status = Command::new("clang")
        .args(["-O2", LIGHT_IR])
        .arg(archive)
        .args(SYSTEM_LIBS)
        .arg("-o")
        .arg(dir.path("light"))
        .status()
        .unwrap();
    assert!(status.success());
    let run = Command::new(dir.path("light")).output().unwrap();
    assert_eq!(run.stdout, b"let there be light\n");
    assert!(run.stderr.is_empty());
    assert_eq!(run.status.code(), Some(0));
}

#[test]
fn output_name_may_start_with_a_dash() {
    build_runtime();
    let dir = Scratch::new("dash");
    let out = Command::new(PAKA)
        .args(["build", LIGHT, "-o", "-dash"])
        .current_dir(dir.path(""))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    assert_eq!(dir.entries(), ["-dash"]);
    let run = Command::new(dir.path("-dash")).output().unwrap();
    assert_eq!(run.stdout, b"let there be light\n");
}

#[test]
fn existing_temporary_name_is_never_written_through_or_removed() {
    build_runtime();
    let (victim, dir) = (Scratch::new("victim"), Scratch::new("planted"));
    fs::write(victim.path("file"), "victim").unwrap();
    // The shell plants a symlink under the name its own process id gives, then becomes paka.
    let script = r#"ln -s "$VICTIM" "$OUT.$$.ll" && exec "$PAKA" build "$SRC" -o "$OUT""#;
    let out = Command::new("sh")
        .args(["-c", script])
        .env("VICTIM", victim.path("file"))
        .env("OUT", dir.path("out"))
        .env("PAKA", PAKA)
        .env("SRC", LIGHT)
        .output()
        .unwrap();
    assert_tool_error(&out, LIGHT, "clang-failed");
    assert_eq!(fs::read_to_string(victim.path("file")).unwrap(), "victim");
    let [planted] = &dir.entries()[..] else {
        panic!("{:?}", dir.entries())
    };
    assert!(
        fs::symlink_metadata(dir.path(planted))
            .unwrap()
            .is_symlink()
    );
}

/// A path need not be UTF-8: it is opened as given and printed byte for byte.
#[test]
fn file_names_need_not_be_utf8() {
    let dir = Scratch::new("bytes");
    let source = dir.path("").join(OsStr::from_bytes(b"l\xFFght.paka"));
    fs::copy(LIGHT, &source).unwrap();
    let run = Command::new(PAKA).arg("run").arg(&source).output().unwrap();
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(run.stdout, b"let there be light\n");

    let output = dir.path("").join(OsStr::from_bytes(b"\xFFout"));
    build_runtime();
    let args = [source.as_os_str(), OsStr::new("-o"), output.as_os_str()];
    let build = Command::new(PAKA).arg("build").args(args).output().unwrap();
    assert_eq!(build.status.code(), Some(0), "{build:?}");
    assert_eq!(Command::new(&output).output().unwrap().stdout, run.stdout);

    let missing = OsStr::from_bytes(b"no/such/\xFF.paka");
    let out = Command::new(PAKA).arg("run").arg(missing).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let mut prefix = missing.as_bytes().to_vec();
    prefix.extend(b": error[file-unreadable]: ");
    assert!(out.stderr.starts_with(&prefix), "{out:?}");
}

/// A line feed in the output path does not reach the message: the diagnostic stays one line.
#[test]
fn line_feed_in_output_path_keeps_one_line() {
    let dir = Scratch::new("linefeed");
    let out = build_native(LIGHT, &dir.path("no/such\ndir/out"));
    assert_tool_error(&out, LIGHT, "clang-failed");
    assert_eq!(out.stderr.iter().filter(|&&b| b == b'\n').count(), 1);
}
