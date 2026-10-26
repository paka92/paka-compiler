//! The differential harness (M4-40): planted differences in output, status, trap kind and trap
//! position each differ, as does a run ended by a signal; and the command lists every run case of
//! the conformance suite once. Every UI example agrees on both back ends, and a planted view fault
//! differs in each (M5-26). A planted release fault is found, shrunk and kept as a case (M6-20).

use paka_installer::diff::{Outcome, cases, difference};
use std::path::Path;
use std::process::Command;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/conformance");

fn trapped(stderr: &str) -> Outcome {
    Outcome {
        stdout: b"before\n".to_vec(),
        stderr: stderr.as_bytes().to_vec(),
        status: Some(3),
    }
}

const TRAP: &str = "a: trap divide_by_zero at main.paka:4:9\n  7 / 0\n  in f, called from update\n";

#[test]
fn planted_differences_differ() {
    let base = trapped(TRAP);
    let output = Outcome {
        stdout: b"after\n".to_vec(),
        ..base.clone()
    };
    let status = Outcome {
        status: Some(1),
        ..base.clone()
    };
    let kind = trapped(&TRAP.replace("divide_by_zero", "overflow"));
    let position = trapped(&TRAP.replace("4:9", "4:10"));
    for planted in [output, status, kind, position] {
        assert!(difference(&base, &planted).is_some());
    }
}

#[test]
fn help_lines_and_limit_positions_are_not_meaning() {
    let first = "a: trap divide_by_zero at main.paka:4:9\n";
    assert_eq!(difference(&trapped(TRAP), &trapped(first)), None);
    let stack = |at| trapped(&format!("a: trap stack at main.paka:{at}\n"));
    assert_eq!(difference(&stack("2:3"), &stack("9:1")), None);
    let hole = |name| {
        trapped(&format!(
            "a: trap hole at main.paka:2:3\n  ??{name}\n  in update\n"
        ))
    };
    assert_eq!(difference(&hole("scan"), &hole("scan")), None);
    assert!(difference(&hole("scan"), &hole("walk")).is_some());
}

#[test]
fn death_by_signal_differs() {
    let killed = Command::new("sh")
        .args(["-c", "kill -SEGV $$"])
        .output()
        .unwrap();
    let killed = Outcome::from(killed);
    assert_eq!(killed.status, None);
    let clean = Outcome {
        status: Some(0),
        ..Outcome::default()
    };
    assert!(difference(&clean, &killed).is_some());
}

#[test]
fn every_run_case_is_listed_once() {
    // The foreign cases build their hosts in this cache.
    let cache = std::env::temp_dir().join(format!("paka-diff-cache-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
        .arg(ROOT)
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&cache);
    let text = String::from_utf8(out.stdout).unwrap();
    let (total, listed) = text
        .trim_end()
        .rsplit_once('\n')
        .map(|(l, t)| (t, l))
        .unwrap();
    let named: Vec<_> = (listed.lines())
        .map(|l| {
            let (verdict, rest) = l.split_once(' ').unwrap();
            assert!(["agree", "differ", "unsupported"].contains(&verdict), "{l}");
            rest.split([';', ':']).next().unwrap()
        })
        .collect();
    let want: Vec<_> = (cases(Path::new(ROOT)).unwrap().iter())
        .map(|c| {
            c.strip_prefix(ROOT)
                .unwrap()
                .to_string_lossy()
                .trim_start_matches('/')
                .to_owned()
        })
        .collect();
    assert_eq!(named, want);
    println!("{total}");
    assert!(total.contains(" 0 differ,"), "{text}");
    assert_eq!(out.status.code(), Some(0));
}

/// Each planted fault of the dev back end, with a seed whose program it changes.
const PLANTED: [(u8, u64); 5] = [(1, 224), (2, 59), (3, 60), (4, 181), (5, 119)];

#[test]
fn the_fuzz_loop_finds_and_shrinks_each_planted_fault() {
    let keep = std::env::temp_dir().join(format!("paka-fuzz-{}", std::process::id()));
    std::thread::scope(|s| {
        for (plant, seed) in PLANTED {
            let keep = keep.join(plant.to_string());
            s.spawn(move || {
                let fuzz = |plant: u8| {
                    let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
                        .args(["--fuzz", &seed.to_string(), "1"])
                        .arg(&keep)
                        .env("PAKA_PLANT", plant.to_string())
                        .output()
                        .unwrap();
                    String::from_utf8(out.stdout).unwrap()
                };
                assert!(fuzz(0).starts_with(&format!("agree seed {seed};")));
                let found = fuzz(plant);
                assert!(
                    found.starts_with(&format!("differ seed {seed}:")),
                    "{found}"
                );
                let kept = std::fs::read_to_string(keep.join(format!("{seed}.paka"))).unwrap();
                assert!(kept.lines().count() < 30, "plant {plant}:\n{kept}");
            });
        }
    });
    let _ = std::fs::remove_dir_all(&keep);
}

/// The release fault planted in the IR clang is given, `>` as `>=`, is found from seed 83, which
/// agrees without it, and kept as a run case of under 20 lines that release mode of the harness
/// passes, and fails with the fault (M6-20).
#[test]
fn the_release_fuzz_loop_keeps_a_planted_release_fault_as_a_case() {
    build_paka_beside(Path::new(env!("CARGO_BIN_EXE_paka-diff")));
    let root = std::env::temp_dir().join(format!("paka-fuzz-release-{}", std::process::id()));
    let diff = |plant: &str, args: &[&std::ffi::OsStr]| {
        let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
            .args(args)
            .env("PAKA_PLANT", plant)
            .output()
            .unwrap();
        (String::from_utf8(out.stdout).unwrap(), out.status.code())
    };
    let fuzz = |plant| {
        diff(
            plant,
            &[
                "--fuzz-release".as_ref(),
                "83".as_ref(),
                "1".as_ref(),
                root.join("fuzz").as_ref(),
            ],
        )
        .0
    };
    assert!(fuzz("0").starts_with("agree seed 83;"));
    let found = fuzz("9");
    assert!(
        found.starts_with("differ seed 83: release stdout"),
        "{found}"
    );
    let kept = std::fs::read_to_string(root.join("fuzz/83/main.paka")).unwrap();
    assert!(kept.lines().count() < 20, "{kept}");
    for list in ["release-skip", "release-exempt"] {
        std::fs::write(root.join(list), "").unwrap();
    }
    let release = |plant| diff(plant, &["--release".as_ref(), root.as_ref()]);
    let (text, status) = release("0");
    assert!(
        text.starts_with("agree fuzz/83;") && status == Some(0),
        "{text}"
    );
    let (text, status) = release("9");
    assert!(
        text.starts_with("differ fuzz/83: release stdout") && status == Some(1),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Builds `paka` and its runtime archive beside the binary `exe`, in its profile.
fn build_paka_beside(exe: &Path) {
    let profile = exe.parent().unwrap();
    let mut cargo = Command::new(env!("CARGO"));
    cargo.args([
        "build",
        "-p",
        "paka",
        "-p",
        "paka-rt",
        "--locked",
        "--offline",
        "--target-dir",
    ]);
    cargo.arg(profile.parent().unwrap());
    if profile.ends_with("release") {
        cargo.arg("--release");
    }
    assert!(cargo.status().unwrap().success(), "building paka failed");
}

/// Each planted staleness of the compile cache is found by seed 0's edit sequence, which agrees
/// without it.
#[test]
fn edit_sequences_find_each_planted_staleness() {
    let keep = std::env::temp_dir().join(format!("paka-edits-{}", std::process::id()));
    std::thread::scope(|s| {
        for plant in [0, 6, 7] {
            let keep = keep.join(plant.to_string());
            s.spawn(move || {
                let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
                    .args(["--edits", "0", "1"])
                    .arg(&keep)
                    .env("PAKA_PLANT", plant.to_string())
                    .output()
                    .unwrap();
                let text = String::from_utf8(out.stdout).unwrap();
                let found = text.contains("\ndiffer seed 0 ");
                assert_eq!(found, plant != 0, "plant {plant}:\n{text}");
                assert_eq!(found, keep.join("0/edits/0.paka").exists());
            });
        }
    });
    let _ = std::fs::remove_dir_all(&keep);
}

#[test]
fn ui_examples_agree_and_a_planted_view_fault_differs_in_each() {
    let ui = |plant: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
            .args(["--ui", concat!(env!("CARGO_MANIFEST_DIR"), "/../tests")])
            .env("PAKA_PLANT", plant)
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        let total = text.lines().last().unwrap_or_default().to_owned();
        (text, total, out.status.code())
    };
    let (text, total, status) = ui("0");
    for case in ["sample_counter", "sample_todo", "ui_ends"] {
        assert!(
            text.contains(&format!("agree cases/examples/{case} ")),
            "{text}"
        );
    }
    let agree: usize = total.split(' ').next().unwrap().parse().unwrap();
    assert_eq!(
        (total.as_str(), status),
        (
            format!("{agree} agree, 0 differ, 0 unsupported").as_str(),
            Some(0)
        ),
        "{text}"
    );
    let (text, total, status) = ui("8");
    assert_eq!(
        (total.as_str(), status),
        (
            format!("0 agree, {agree} differ, 0 unsupported").as_str(),
            Some(1)
        ),
        "{text}"
    );
}
