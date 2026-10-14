//! The metrics command on a hand-made set of logs (tests/metrics/runs) and two rulings files.
//!
//! Run folders are `<task>-<language>-<rep>`; edits count 8 for a run that never passed. The
//! numbers in each log (first try, edits, output tokens), by task A to D:
//!
//! ```text
//!        Paka                       Rust                       TypeScript
//! A  1: yes 0 100  2: no 2 300   1: yes 0 80   2: no 1 120   1: yes 0 90   2: yes 0 110
//! B  1: no 1 200   2: yes 0 150  1: yes 0 100  2: no 4 999   1: no 3 250   2: yes 0 100
//! C  1: never 8 400  2: no 5 300 1: yes 0 120  2: no 2 180   1: yes 0 100  2: no 1 140
//! D  1: yes 0 100  2: yes 0 100  1: yes 0 100  2: yes 0 100  1: yes 0 100  2: unfinished
//! ```
//!
//! - A-Rust-1 has a void attempt, then an ended one: only the ended attempt counts.
//! - C-Paka-1 never passed: 8 edits, its real 400 tokens.
//! - D-TypeScript-2 has only a void attempt: not counted, listed.
//! - B-Rust-2 is ruled void: out of every table.
//! - A-Paka-2 is tagged: counted, but left out of the tagged table.

use std::path::Path;
use std::process::{Command, Output};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/metrics");

/// Runs the command on the fixture's logs; `rulings` is a file in the fixture folder or a full path.
fn metrics(rulings: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_metrics"))
        .arg(Path::new(DIR).join("runs"))
        .arg(Path::new(DIR).join(rulings))
        .output()
        .unwrap()
}

fn stdout(rulings: &str) -> String {
    let out = metrics(rulings);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Every run but B-Rust-2, tasks as they fell.
///
/// ```text
/// Paka  A 1+2: first 1, edits 0+2=2, out 100+300=400
///       B 1+2: first 1, edits 1+0=1, out 200+150=350
///       C 1+2: first 0, edits 8+5=13, out 400+300=700
///       D 1+2: first 2, edits 0, out 100+100=200
/// Rust  A: first 1 of 2, edits 0+1=1, out 80+120=200
///       B: first 1 of 1, edits 0, out 100 (B-Rust-2 is void)
///       C: first 1 of 2, edits 0+2=2, out 120+180=300
///       D: first 2 of 2, edits 0, out 200
/// TS    A: first 2 of 2, edits 0, out 90+110=200
///       B: first 1 of 2, edits 3+0=3, out 250+100=350
///       C: first 1 of 2, edits 0+1=1, out 100+140=240
///       D: first 1 of 1, edits 0, out 100 (D-TypeScript-2 is unfinished)
/// ```
/// Edits are the sum over mean: A Paka 2/2 = 1.00, B Rust 0/1 = 0.00, B TS 3/2 = 1.50,
/// C Paka 13/2 = 6.50, and so on.
const BY_TASK: &str = "\
by task
task language     runs  first-try   edits   output
A Paka               2      50.0%    1.00      400
A Rust               2      50.0%    0.50      200
A TypeScript         2     100.0%    0.00      200
B Paka               2      50.0%    0.50      350
B Rust               1     100.0%    0.00      100
B TypeScript         2      50.0%    1.50      350
C Paka               2       0.0%    6.50      700
C Rust               2      50.0%    1.00      300
C TypeScript         2      50.0%    0.50      240
D Paka               2     100.0%    0.00      200
D Rust               2     100.0%    0.00      200
D TypeScript         1     100.0%    0.00      100
";

/// Nothing dropped, so the sums of BY_TASK over A to D.
///
/// ```text
/// Paka  runs 8, first 1+1+0+2=4 (50.0%), edits 2+1+13+0=16 (2.00), out 400+350+700+200=1650
/// Rust  runs 7, first 1+1+1+2=5 (71.4%), edits 1+0+2+0=3 (0.43),   out 200+100+300+200=800
/// TS    runs 7, first 2+1+1+1=5 (71.4%), edits 0+3+1+0=4 (0.57),   out 200+350+240+100=890
/// ```
const NOTHING_DROPPED: &str = "\
not the verdict: nothing dropped
language          runs  first-try   edits   output
Paka                 8      50.0%    2.00     1650
Rust                 7      71.4%    0.43      800
TypeScript           7      71.4%    0.57      890
";

const UNFINISHED: &str = "unfinished, not counted: D-TypeScript-2\n";

/// Tasks C and D dropped, so A and B decide, and 2 dropped is within the limit.
///
/// ```text
/// Paka  runs 4, first 1+1=2 (50.0%), edits 2+1=3 (0.75), out 400+350=750
/// Rust  runs 3, first 1+1=2 (66.7%), edits 1+0=1 (0.33), out 200+100=300
/// TS    runs 4, first 2+1=3 (75.0%), edits 0+3=3 (0.75), out 200+350=550
///
/// first-try gap = 66.7 - 50.0 = 16.7 points: above 10, up to 20: close
/// edits gap     = 0.75 - 0.33 = 0.42:        up to 0.5: held
/// token ratio   = 750 / 300   = 2.50:        above 2.0: lost
/// one metric lost: stop
///
/// tagged table: A-Paka-2 (first no, edits 2, out 300) leaves Paka with
///   runs 3, first 2 (66.7%), edits 3-2=1 (0.33), out 750-300=450
/// ```
const TWO_DROPPED: [&str; 3] = [
    "\
by language, dropped tasks removed
language          runs  first-try   edits   output
Paka                 4      50.0%    0.75      750
Rust                 3      66.7%    0.33      300
TypeScript           4      75.0%    0.75      550
",
    "\
verdict, Paka against Rust
dropped tasks: 2 (C, D)
first-try gap    16.7 points  close
edits gap               0.42  held
token ratio             2.50  lost
stop
",
    "\
not the verdict: tagged runs removed
language          runs  first-try   edits   output
Paka                 3      66.7%    0.33      450
Rust                 3      66.7%    0.33      300
TypeScript           4      75.0%    0.75      550
",
];

/// Tasks A, C and D dropped, so only B is left, and a third dropped task makes the verdict
/// inconclusive although the metrics alone would give stop.
///
/// ```text
/// Paka  runs 2, first 1 (50.0%),  edits 1 (0.50), out 350
/// Rust  runs 1, first 1 (100.0%), edits 0 (0.00), out 100
/// TS    runs 2, first 1 (50.0%),  edits 3 (1.50), out 350
///
/// first-try gap = 100.0 - 50.0 = 50.0 points: above 20: lost
/// edits gap     = 0.50 - 0.00  = 0.50:        up to 0.5: held
/// token ratio   = 350 / 100    = 3.50:        above 2.0: lost
/// metrics alone: stop; 3 dropped tasks: inconclusive
///
/// A-Paka-2 is dropped with its task, so the tagged table equals the first one.
/// ```
const THREE_DROPPED: [&str; 3] = [
    "\
by language, dropped tasks removed
language          runs  first-try   edits   output
Paka                 2      50.0%    0.50      350
Rust                 1     100.0%    0.00      100
TypeScript           2      50.0%    1.50      350
",
    "\
verdict, Paka against Rust
dropped tasks: 3 (A, C, D)
first-try gap    50.0 points  lost
edits gap               0.50  held
token ratio             3.50  lost
inconclusive
",
    "\
not the verdict: tagged runs removed
language          runs  first-try   edits   output
Paka                 2      50.0%    0.50      350
Rust                 1     100.0%    0.00      100
TypeScript           2      50.0%    1.50      350
",
];

/// The report: by language, by task, verdict, nothing dropped, tagged removed, unfinished runs.
fn report([by_language, verdict, tagged]: [&str; 3]) -> String {
    [
        by_language,
        BY_TASK,
        verdict,
        NOTHING_DROPPED,
        tagged,
        UNFINISHED,
    ]
    .join("\n")
}

#[test]
fn two_dropped_tasks_give_the_verdict_of_the_rest() {
    assert_eq!(stdout("rulings-2.txt"), report(TWO_DROPPED));
}

#[test]
fn a_third_dropped_task_makes_the_verdict_inconclusive() {
    assert_eq!(stdout("rulings-3.txt"), report(THREE_DROPPED));
}

#[test]
fn a_ruling_that_names_nothing_is_refused() {
    let dir = std::env::temp_dir().join(format!("harness-metrics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in [
        ("typo", "run B-Rust-9 void\n"),
        ("form", "task C removed\n"),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        let out = metrics(path.to_str().unwrap());
        assert!(!out.status.success() && out.stdout.is_empty(), "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}
