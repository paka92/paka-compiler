//! Measurement-only builds (M6-30), run with `--features measure`. The IR test also needs
//! `PAKA_NORMAL_BUILD`: a folder holding a normal compiler's `paka` and its `libpaka_rt.a`.
#![cfg(feature = "measure")]

#[allow(dead_code)]
mod common;

use common::{EXAMPLE_CACHE, PAKA, Scratch, build_native, build_runtime};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/bounds.paka");
const CONFORMANCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/conformance");

/// The folder of a clang that passes the version check and keeps the IR it is handed as
/// `$CAPTURE`.
fn capturing_clang(scratch: &Scratch) -> PathBuf {
    let dir = scratch.path("bin");
    fs::create_dir(&dir).unwrap();
    let script = format!(
        "#!/bin/sh\n[ \"$1\" = --version ] && echo 'clang version {}' && exit 0\n\
         for a; do case $a in *.ll) cp \"$a\" \"$CAPTURE\";; esac; done\n",
        include_str!("../clang-version").trim()
    );
    let clang = dir.join("clang");
    fs::write(&clang, script).unwrap();
    fs::set_permissions(&clang, fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// The IR `paka` hands the clang in `bin` for `source`, built in `dir` with `args` before `-o`;
/// none when the build stops first.
fn captured(paka: &Path, bin: &Path, dir: &Path, source: &str, args: &[&str]) -> Option<String> {
    let capture = bin.with_file_name("captured.ll");
    let _ = fs::remove_file(&capture);
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    Command::new(paka)
        .arg("build")
        .arg(source)
        .args(args)
        .arg("-o")
        .arg(bin.with_file_name("out"))
        .current_dir(dir)
        .env("PATH", path)
        .env("CAPTURE", &capture)
        .env("XDG_CACHE_HOME", EXAMPLE_CACHE)
        .output()
        .unwrap();
    fs::read_to_string(capture).ok()
}

/// `ir` at `clang -O3`, every function kept external so none is folded into `update`.
fn optimized(ir: &str) -> String {
    let mut clang = Command::new("clang")
        .args(["-O3", "-S", "-emit-llvm", "-x", "ir", "-", "-o", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let ir = ir.replace("define internal i32 @p_", "define i32 @p_");
    clang
        .stdin
        .take()
        .unwrap()
        .write_all(ir.as_bytes())
        .unwrap();
    let out = clang.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn omitting_bounds_leaves_no_bounds_check_in_optimized_ir() {
    build_runtime();
    let scratch = Scratch::new("measure-fixture");
    let clang = capturing_clang(&scratch);
    let dir = Path::new(FIXTURE).parent().unwrap();
    let ir =
        |args: &[_]| optimized(&captured(Path::new(PAKA), &clang, dir, FIXTURE, args).unwrap());
    assert!(ir(&[]).contains("out_of_bounds"));
    assert!(!ir(&["--omit", "bounds"]).contains("out_of_bounds"));
}

#[test]
fn a_measurement_build_runs_under_its_banner() {
    build_runtime();
    let scratch = Scratch::new("measure-run");
    let exe = scratch.path("bounds");
    let built = Command::new(PAKA)
        .args(["build", FIXTURE, "--omit", "bounds", "-o"])
        .arg(&exe)
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");
    let out = Command::new(&exe).output().unwrap();
    assert_eq!(
        (out.status.code(), String::from_utf8(out.stdout).unwrap()),
        (Some(0), "2 0 4 1\n".into())
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(
        lines[0], "MEASUREMENT BUILD: not a Paka program",
        "{stderr}"
    );
    assert!(
        lines[1].starts_with("MEASUREMENT BUILD: copy routine "),
        "{stderr}"
    );
    assert_eq!(lines.len(), 2, "{stderr}");
    // A build without the switch links the same runtime.
    let built = build_native(FIXTURE, &exe);
    assert!(built.status.success(), "{built:?}");
    let out = Command::new(&exe).output().unwrap();
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .starts_with("MEASUREMENT BUILD: ")
    );
}

#[test]
fn without_the_switch_conformance_ir_is_the_normal_compilers() {
    build_runtime();
    let normal = std::env::var_os("PAKA_NORMAL_BUILD").expect("PAKA_NORMAL_BUILD is not set");
    let normal = Path::new(&normal).join("paka");
    let scratch = Scratch::new("measure-conformance");
    let clang = capturing_clang(&scratch);
    let mut built = 0;
    for area in fs::read_dir(CONFORMANCE).unwrap() {
        let area = area.unwrap().path();
        for case in fs::read_dir(&area).into_iter().flatten() {
            let case = case.unwrap().path();
            if !case.join("main.paka").is_file() {
                continue;
            }
            let ir = |paka: &Path| captured(paka, &clang, &case, "main.paka", &[]);
            let measured = ir(Path::new(PAKA));
            assert!(measured == ir(&normal), "{}", case.display());
            built += usize::from(measured.is_some());
        }
    }
    assert!(built > 400, "only {built} cases built");
}
