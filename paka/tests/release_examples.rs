//! Function examples built by `paka build --examples` and run by the binary's `--paka-examples`
//! switch, one per process, and release's runner over them (M7-11b, docs/decisions/M7-03.md).

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch, build_runtime};
use paka::release::{Ran, run};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXAMPLES: &str = "
example halves { ratio(4, 2) == 2 }

example wrong { ratio(4, 2) == 3 }

example divides { ratio(1, 0) == 0 }
";

/// An app the M1 view holds; `ratio` is on line 3.
const LIGHT: &str = "enum Msg { Start }

fn ratio(n: Int, d: Int) -> Int { n / d }

fn update(m: Msg) -> List<Effect<Msg>> {
  [Print(\"light\"), Exit]
}

app light {
  start: Start
  update: update
  grants: [Console]
}
";

/// A headless app with state, beyond the M1 view; `ratio` is on line 5.
const TALLY: &str = "type Tally { n: Int = 0 }

enum Msg { Start }

fn ratio(n: Int, d: Int) -> Int { n / d }

fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {
  s.n += ratio(4, 2)
  [Print(\"n is {s.n}\"), Exit]
}

app tally {
  state: Tally
  start: Start
  update: update
  grants: [Console]
}
";

/// `paka build` of `source` in `scratch` as `main`, with `extra` arguments after `-o main`.
fn build(scratch: &Scratch, source: &str, extra: &[&str]) -> (Output, PathBuf) {
    build_runtime();
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    fs::write(&path, source).unwrap();
    let out = Command::new(PAKA)
        .args(["build".as_ref(), path.as_os_str(), "-o".as_ref(), exe.as_os_str()])
        .args(extra)
        .output()
        .unwrap();
    (out, exe)
}

fn built(scratch: &Scratch, source: &str, extra: &[&str]) -> PathBuf {
    let (out, exe) = build(scratch, source, extra);
    assert!(out.status.success(), "{out:?}");
    exe
}

/// Runs `exe` with `args`: its status, standard output and standard error.
fn ran(exe: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(exe).args(args).output().unwrap();
    let text = |b| String::from_utf8(b).unwrap();
    (out.status.code(), text(out.stdout), text(out.stderr))
}

fn switch(exe: &Path, name: &str) -> (Option<i32>, String, String) {
    ran(exe, &["--paka-examples", name])
}

fn said(status: i32, stdout: &str, stderr: &str) -> (Option<i32>, String, String) {
    (Some(status), stdout.into(), stderr.into())
}

/// Each named example runs alone through the switch, on both of the back end's entries; without
/// the switch the app runs as built without examples.
#[test]
fn the_switch_runs_one_named_example() {
    let apps = [
        ("light", LIGHT, 3, "light\n"),
        ("tally", TALLY, 5, "n is 2\n"),
    ];
    for (app, source, line, plain) in apps {
        let scratch = Scratch::new(&format!("examples-{app}"));
        let names = ["--examples", "halves", "wrong", "divides"];
        let exe = built(&scratch, &(source.to_owned() + EXAMPLES), &names);
        assert_eq!(ran(&exe, &[]), said(0, plain, ""));
        assert_eq!(switch(&exe, "halves"), said(0, "halves pass\n", ""));
        assert_eq!(switch(&exe, "wrong"), said(0, "wrong fail\n", ""));
        let trap = format!("{app}: trap divide_by_zero at main.paka:{line}:37\n");
        assert_eq!(switch(&exe, "divides"), said(3, "", &trap));
        let unknown = format!("{app}: --paka-examples: no function example named nope\n");
        assert_eq!(switch(&exe, "nope"), said(2, "", &unknown));
        let shape = format!("{app}: --paka-examples takes one example name\n");
        assert_eq!(ran(&exe, &["--paka-examples"]), said(2, "", &shape));
        let two = ["--paka-examples", "halves", "wrong"];
        assert_eq!(ran(&exe, &two), said(2, "", &shape));
    }
}

/// Only the named examples are compiled in, each once however often it is named.
#[test]
fn only_the_named_examples_are_built() {
    let scratch = Scratch::new("examples-named");
    let names = ["--examples", "halves", "halves"];
    let exe = built(&scratch, &(TALLY.to_owned() + EXAMPLES), &names);
    assert_eq!(switch(&exe, "halves"), said(0, "halves pass\n", ""));
    let unknown = "tally: --paka-examples: no function example named wrong\n";
    assert_eq!(switch(&exe, "wrong"), said(2, "", unknown));
}

/// A name that is not a function example of the program refuses the build, naming it, and
/// writes nothing.
#[test]
fn a_name_that_is_not_a_function_example_refuses_the_build() {
    let scratch = Scratch::new("examples-unknown");
    for name in ["nope", "ratio"] {
        let names = ["--examples", "halves", name];
        let (out, exe) = build(&scratch, &(TALLY.to_owned() + EXAMPLES), &names);
        let path = scratch.path("main.paka");
        let want = format!(
            "{}: error[unknown-name]: no function example named {name}\n",
            path.display()
        );
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(String::from_utf8_lossy(&out.stderr), want);
        assert!(out.stdout.is_empty());
        assert!(!exe.exists());
    }
}

/// A plain build holds no example and does not serve the switch.
#[test]
fn a_plain_build_holds_no_example() {
    let scratch = Scratch::new("examples-plain");
    let exe = built(&scratch, &(TALLY.to_owned() + EXAMPLES), &[]);
    let refused = "tally: --paka-examples cannot be used here\n";
    assert_eq!(switch(&exe, "halves"), said(2, "", refused));
    let nm = Command::new("nm").arg(&exe).output().unwrap();
    assert!(nm.status.success() && !nm.stdout.is_empty());
    let symbols = String::from_utf8_lossy(&nm.stdout);
    for name in ["example.", "examples.call", "paka_rt1_examples"] {
        assert!(!symbols.contains(name), "{name}");
    }
    let named = built(&scratch, &(TALLY.to_owned() + EXAMPLES), &["--examples", "halves"]);
    let nm = Command::new("nm").arg(&named).output().unwrap();
    assert!(String::from_utf8_lossy(&nm.stdout).contains("paka_rt1_examples"));
}

/// The runner reports what a built binary's run was.
#[test]
fn the_runner_reports_each_outcome_of_a_built_binary() {
    let scratch = Scratch::new("examples-runner");
    let names = ["--examples", "halves", "wrong", "divides"];
    let exe = built(&scratch, &(TALLY.to_owned() + EXAMPLES), &names);
    assert_eq!(run(&exe, "halves").unwrap(), Ran::Pass);
    assert_eq!(run(&exe, "wrong").unwrap(), Ran::Fail);
    let trap = "tally: trap divide_by_zero at main.paka:5:37";
    assert_eq!(run(&exe, "divides").unwrap(), Ran::Trapped(trap.into()));
    let seen = "status 2, standard output \"\", standard error \
        \"tally: --paka-examples: no function example named nope\\n\"";
    assert_eq!(run(&exe, "nope").unwrap(), Ran::Other(seen.into()));
    assert_eq!(Ran::Pass.to_string(), "pass");
    assert_eq!(Ran::Fail.to_string(), "fail");
    assert_eq!(Ran::TimedOut.to_string(), "timed out");
    assert_eq!(Ran::Trapped(trap.into()).to_string(), format!("trapped: {trap}"));
    assert_eq!(Ran::Other(seen.into()).to_string(), seen);
}

/// A fake binary: a shell script with `body`.
fn fake(scratch: &Scratch, name: &str, body: &str) -> PathBuf {
    let path = scratch.path(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Planted wrong passes: `e pass` printed with any other status, more output or any standard
/// error is not a pass, and neither is another example's pass.
#[test]
fn the_runner_trusts_only_the_exact_pass() {
    let scratch = Scratch::new("examples-planted");
    let pass = fake(&scratch, "pass", "test \"$1 $2\" = \"--paka-examples e\" && echo 'e pass'");
    assert_eq!(run(&pass, "e").unwrap(), Ran::Pass);
    let fail = fake(&scratch, "fail", "echo 'e fail'");
    assert_eq!(run(&fail, "e").unwrap(), Ran::Fail);
    let planted = [
        ("status", "echo 'e pass'; exit 3", "status 3, standard output \"e pass\\n\", standard error \"\""),
        ("stderr", "echo 'e pass'; echo warning >&2", "status 0, standard output \"e pass\\n\", standard error \"warning\\n\""),
        ("more", "echo 'e pass'; echo 'e pass'", "status 0, standard output \"e pass\\ne pass\\n\", standard error \"\""),
        ("other", "echo 'f pass'", "status 0, standard output \"f pass\\n\", standard error \"\""),
        ("unended", "printf 'e pass'", "status 0, standard output \"e pass\", standard error \"\""),
        ("signal", "echo 'e pass'; kill -9 $$", "killed by a signal, standard output \"e pass\\n\", standard error \"\""),
    ];
    for (name, body, seen) in planted {
        let exe = fake(&scratch, name, body);
        assert_eq!(run(&exe, "e").unwrap(), Ran::Other(seen.into()), "{name}");
    }
    let trapped = fake(&scratch, "trap", "echo 'e pass'; echo 'a: trap stack at main.paka:1:1' >&2; exit 3");
    let seen = "status 3, standard output \"e pass\\n\", standard error \"a: trap stack at main.paka:1:1\\n\"";
    assert_eq!(run(&trapped, "e").unwrap(), Ran::Other(seen.into()));
}

/// A run past the fixed limit is stopped and reported as timed out.
#[test]
fn the_runner_stops_a_run_past_the_limit() {
    let scratch = Scratch::new("examples-slow");
    let slow = fake(&scratch, "slow", "exec sleep 600");
    assert_eq!(run(&slow, "e").unwrap(), Ran::TimedOut);
}
