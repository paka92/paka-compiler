//! Every runnable case as a native executable against the case files and `paka run`
//! (TASKS.md M1-20), and the write failures of spec/m1.md section 13 on both back ends.

#[allow(dead_code)]
mod common;

use common::{LIGHT, PAKA, Scratch, build_native, case_args};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// What a run is judged by. A process ended by a signal has no code and equals no expectation.
#[derive(Debug, PartialEq)]
struct Seen {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

impl From<Output> for Seen {
    fn from(out: Output) -> Seen {
        Seen {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
        }
    }
}

/// The hand-written expectation of an accept case: `stdout` and `status` files, optional `stderr`.
fn expectation(case: &Path) -> Option<Seen> {
    let read = |name| fs::read(case.join(name));
    if case.join("reject").exists() {
        return None;
    }
    let status = String::from_utf8(read("status").ok()?).ok()?;
    Some(Seen {
        stdout: read("stdout").ok()?,
        stderr: read("stderr").unwrap_or_default(),
        code: Some(status.trim_end().parse().ok()?),
    })
}

fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    folders
}

fn accept_cases() -> Vec<(PathBuf, Seen)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases");
    let run_stage = |s: &PathBuf| {
        let name = s.file_name().and_then(|n| n.to_str());
        matches!(name, Some("lex" | "parse" | "check" | "run"))
    };
    let stages = subfolders(&root).into_iter().filter(run_stage);
    let cases = stages.flat_map(|s| subfolders(&s));
    cases
        .filter_map(|case| Some((case.clone(), expectation(&case)?)))
        .collect()
}

fn interpreted(source: &Path, dir: &Path, args: &[String]) -> Command {
    let mut command = Command::new(PAKA);
    command.arg("run").arg(source).args(args).current_dir(dir);
    command
}

#[test]
fn every_runnable_case_runs_natively() {
    let cases = accept_cases();
    assert!(!cases.is_empty(), "no runnable cases found");
    let scratch = Scratch::new("native-cases");
    let mut failures = vec![];
    for (case, want) in &cases {
        let name = case.file_name().unwrap().to_str().unwrap();
        let source = case.join("main.paka");
        let exe = scratch.path(name);
        let built = build_native(source.to_str().unwrap(), &exe);
        if !built.status.success() {
            failures.push(format!("{name}: build {built:?}"));
            continue;
        }
        let args = case_args(case);
        let mut native = Command::new(&exe);
        native.args(&args).current_dir(case);
        let runs = [
            ("interpreter", interpreted(&source, case, &args)),
            ("native", native),
        ];
        for (back_end, mut command) in runs {
            let seen = Seen::from(command.output().unwrap());
            if seen != *want {
                failures.push(format!("{name}: {back_end} {seen:?}, want {want:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Standard output on `/dev/full`.
fn on_full_device(mut command: Command) -> Seen {
    command.stdout(File::options().write(true).open("/dev/full").unwrap());
    command.output().unwrap().into()
}

/// Standard output a pipe whose read end is closed before the program starts: the program is
/// started by a shell that waits for the end of its standard input, which is sent only after the
/// read end is dropped, so the program cannot write earlier.
fn on_closed_pipe(command: Command) -> Seen {
    let mut gate = Command::new("sh");
    gate.args([
        "-c",
        "read _; exec \"$@\"",
        "sh",
        command.get_program().to_str().unwrap(),
    ]);
    gate.args(command.get_args());
    gate.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = gate.spawn().unwrap();
    drop(child.stdout.take());
    child.wait_with_output().unwrap().into()
}

fn failed_write(reason: &str) -> Seen {
    Seen {
        stdout: vec![],
        stderr: format!("light: effect Print failed: {reason}\n").into_bytes(),
        code: Some(3),
    }
}

#[test]
fn write_failures_end_with_status_3_on_both_back_ends() {
    let scratch = Scratch::new("native-writes");
    let exe = scratch.path("light");
    assert_eq!(build_native(LIGHT, &exe).status.code(), Some(0));
    let interp = || interpreted(Path::new(LIGHT), Path::new("."), &[]);
    let native = || Command::new(&exe);
    let full = failed_write("no space left on device");
    assert_eq!(on_full_device(native()), full);
    assert_eq!(on_full_device(interp()), full);
    let closed = failed_write("broken pipe");
    assert_eq!(on_closed_pipe(native()), closed);
    assert_eq!(on_closed_pipe(interp()), closed);
}

#[test]
fn a_closed_standard_output_fails_the_print_natively() {
    let scratch = Scratch::new("native-closed");
    let exe = scratch.path("light");
    assert_eq!(build_native(LIGHT, &exe).status.code(), Some(0));
    let run = format!("exec '{}' >&-", exe.display());
    let out = Command::new("sh").args(["-c", &run]).output().unwrap();
    assert_eq!(Seen::from(out), failed_write("write error"));
}

#[test]
fn comparer_rejects_mismatches() {
    let seen = |stdout: &str, stderr: &str, code| Seen {
        stdout: stdout.into(),
        stderr: stderr.into(),
        code,
    };
    let want = seen("a\n", "", Some(0));
    assert_eq!(want, seen("a\n", "", Some(0)));
    for got in [
        seen("b\n", "", Some(0)),
        seen("a\n", "e", Some(0)),
        seen("a\n", "", Some(3)),
        seen("a\n", "", Some(4)),
        seen("a\n", "", None),
    ] {
        assert_ne!(want, got);
    }
    let killed = Command::new("sh").args(["-c", "kill -KILL $$"]).output();
    assert_eq!(Seen::from(killed.unwrap()).code, None);
}
