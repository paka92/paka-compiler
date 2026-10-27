//! Headless apps built by `paka build` (M6-17): the paper's wc sample, a trapping handler, and a
//! binary free of the installer.

#[allow(dead_code)]
mod common;

use common::{Scratch, build_native};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Builds `source` in `scratch` as `main`.
fn built(scratch: &Scratch, source: &str) -> PathBuf {
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    std::fs::write(&path, source).unwrap();
    let built = build_native(path.to_str().unwrap(), &exe);
    assert!(built.status.success(), "{built:?}");
    exe
}

/// Runs `exe` with `args`: its status, standard output and standard error.
fn run(exe: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(exe).args(args).output().unwrap();
    let text = |b| String::from_utf8(b).unwrap();
    (out.status.code(), text(out.stdout), text(out.stderr))
}

#[test]
fn the_wc_sample_runs_and_holds_no_installer_symbol() {
    let scratch = Scratch::new("release-wc");
    let case =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/run-interp/sample_wc_count");
    let exe = built(
        &scratch,
        &std::fs::read_to_string(case.join("main.paka")).unwrap(),
    );
    let words = case.join("words.txt");
    let ok = (Some(0), "11 words\n".into(), "".into());
    assert_eq!(run(&exe, &[words.to_str().unwrap()]), ok);
    let usage = "wc: expected 1 argument (path), got 0\n";
    assert_eq!(run(&exe, &[]), (Some(2), "".into(), usage.into()));
    let missing = scratch.path("missing.txt");
    let failed = (Some(20), "cannot read file: not found\n".into(), "".into());
    assert_eq!(run(&exe, &[missing.to_str().unwrap()]), failed);
    let nm = Command::new("nm").arg(&exe).output().unwrap();
    assert!(nm.status.success() && !nm.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&nm.stdout).contains("installer"));
}

#[test]
fn a_trapping_handler_performs_none_of_its_effects() {
    let source = "enum Msg { Start, Saved, Failed(why: String) }\n\n\
        fn update(m: Msg) -> List<Effect<Msg>> {\n  match m {\n    \
        Start => [Print(\"first\"), WriteFile(\"out.txt\", \"x\", then: Saved, or: Failed)]\n    \
        Saved => [Print(\"lost\"), Print(\"{[1][5]}\")]\n    \
        Failed(why) => [Print(why)]\n  }\n}\n\n\
        app trapped {\n  start: Start\n  update: update\n  grants: [Console, WriteFiles]\n}\n";
    let scratch = Scratch::new("release-trap");
    let exe = built(&scratch, source);
    let out = Command::new(&exe)
        .current_dir(scratch.path(""))
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(3), "{err}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "first\n");
    assert!(
        err.starts_with("trapped: trap out_of_bounds at main.paka:"),
        "{err}"
    );
    assert!(scratch.path("out.txt").is_file());
}
