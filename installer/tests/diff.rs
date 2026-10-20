//! The differential harness (M4-40): planted differences in output, status, trap kind and trap
//! position each differ, as does a run ended by a signal; and the command lists every run case of
//! the conformance suite once.

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
    let out = Command::new(env!("CARGO_BIN_EXE_paka-diff"))
        .arg(ROOT)
        .output()
        .unwrap();
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
