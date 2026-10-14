//! The metrics command: on a hand-made set of logs (tests/metrics/runs) with three rulings files,
//! and on small sets built in a temp folder for the edge cases.
//!
//! Run folders are `<task>-<language>-<rep>`; edits count 8 for a run that never passed. The
//! numbers in each log (first try, edits, output tokens), by task A to D:
//!
//! ```text
//!        Paka                       Rust                       TypeScript
//! A  1: yes 0 100  2: no 2 300   1: yes 0 80   2: no 1 120   1: yes 0 90   2: yes 0 110
//! B  1: no 1 200   2: yes 0 150  1: yes 0 100  2: void      1: never 8 250 2: yes 0 100
//!                                3: no 1 60
//! C  1: never 8 400  2: no 5 300 1: yes 0 120  2: no 2 180   1: yes 0 100  2: no 1 140
//! D  1: yes 0 100  2: yes 0 100  1: yes 0 100  2: yes 0 100  1: yes 0 100  2: unfinished
//! ```
//!
//! - Every ended run was sent 5000 input tokens; a run that passed made edits + 1 calls, one that
//!   never passed made 8.
//! - A-Rust-1 has a void attempt, then an ended one: only the ended attempt counts.
//! - A-Paka-1 has a void attempt whose reason is a refusal, then an ended one: one refused reply.
//! - B-Rust-2 is ruled void and rerun as B-Rust-3: out of every table.
//! - B-TypeScript-1 never passed (8 edits, its real 250 tokens) and had one cut-off reply.
//! - C-Paka-1 never passed: 8 edits, its real 400 tokens.
//! - D-TypeScript-2 has only a void attempt: not counted, listed.
//! - A-Paka-2 is tagged: counted, but left out of the tagged table.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/metrics");

fn exec(runs: &Path, rulings: &Path, reference: Option<&str>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_metrics"))
        .arg(runs)
        .arg(rulings)
        .args(reference)
        .env("PAKA_PROTOCOL", Path::new(DIR).join("protocol.md"))
        .output()
        .unwrap()
}

/// The fixture's logs under one of its rulings files.
fn fixture(rulings: &str, reference: Option<&str>) -> Output {
    exec(
        &Path::new(DIR).join("runs"),
        &Path::new(DIR).join(rulings),
        reference,
    )
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The block of the report that starts with `title`, up to the blank line.
fn section(out: &Output, title: &str) -> String {
    let all = text(&out.stdout);
    let block = all.split("\n\n").find(|b| b.starts_with(title));
    format!(
        "{}\n",
        block.unwrap_or_else(|| panic!("no section {title}:\n{all}"))
    )
}

// ---- the fixture ----

/// Rulings 2: tasks C and D dropped, so A and B decide, and 2 dropped is within the limit.
///
/// ```text
/// A and B, B-Rust-2 void:
/// Paka  runs 4, first 1+0+0+1=2 (50.0%), edits 0+2+1+0=3 (0.75), out 100+300+200+150=750
/// Rust  runs 4, first 1+0+1+0=2 (50.0%), edits 0+1+0+1=2 (0.50), out 80+120+100+60=360
/// TS    runs 4, first 1+1+0+1=3 (75.0%), edits 0+0+8+0=8 (2.00), out 90+110+250+100=550
///
/// first-try gap = 50.0 - 50.0 = 0.00 points: up to 10: held
/// edits gap     = 0.75 - 0.50 = 0.25:        up to 0.5: held
/// token ratio   = 750 / 360   = 2.083:       above 2.0: lost
/// one metric lost: stop
///
/// nothing dropped (all runs but B-Rust-2):
/// Paka  runs 8, first 4 (50.0%), edits 2+1+13+0=16 (2.00), out 400+350+700+200=1650
/// Rust  runs 8, first 5 (62.5%), edits 1+1+2+0=4 (0.50),   out 200+160+300+200=860
/// TS    runs 7, first 5 (71.4%), edits 0+8+1+0=9 (1.29),   out 200+350+240+100=890
///
/// tagged (A-Paka-2 out, its first no, edits 2, out 300): Paka runs 3, first 2 (66.7%),
///   edits 3-2=1 (0.33), out 750-300=450
///
/// solved, A and B: Paka 4 of 4, Rust 4 of 4, TS 3 of 4 (75.0%, B-TypeScript-1 never passed)
/// cut-off: TS 1. refused: Paka 1 (A-Paka-1)
///
/// input, A and B: 4 runs of 5000 = 20000 for Paka and Rust and TS. Paka's calls
///   1+3+2+1=7, so 20000 - 7*1000 = 13000 without the reference
///
/// against TS: first-try gap = 75.0 - 50.0 = 25.00 points (lost); edits gap = 0.75 - 2.00 = -1.25
///   (held); token ratio = 750 / 550 = 1.364 (held)
/// ```
const TWO_DROPPED: &str = "\
by language, dropped tasks removed
language              runs  first-try     edits    output
Paka                     4      50.0%      0.75       750
Rust                     4      50.0%      0.50       360
TypeScript               4      75.0%      2.00       550

not the verdict: by task
task language         runs  first-try     edits    output    ruling
A Paka                   2      50.0%      1.00       400
A Rust                   2      50.0%      0.50       200
A TypeScript             2     100.0%      0.00       200
B Paka                   2      50.0%      0.50       350
B Rust                   2      50.0%      0.50       160
B TypeScript             2      50.0%      4.00       350
C Paka                   2       0.0%      6.50       700   dropped
C Rust                   2      50.0%      1.00       300   dropped
C TypeScript             2      50.0%      0.50       240   dropped
D Paka                   2     100.0%      0.00       200   dropped
D Rust                   2     100.0%      0.00       200   dropped
D TypeScript             1     100.0%      0.00       100   dropped

verdict, Paka against Rust
dropped tasks: 2 (C, D)
first-try gap        0.00 points  held
edits gap                  0.250  held
token ratio                2.083  lost
stop

not the verdict: nothing dropped
language              runs  first-try     edits    output
Paka                     8      50.0%      2.00      1650
Rust                     8      62.5%      0.50       860
TypeScript               7      71.4%      1.29       890

not the verdict: tagged runs removed
language              runs  first-try     edits    output
Paka                     3      66.7%      0.33       450
Rust                     4      50.0%      0.50       360
TypeScript               4      75.0%      2.00       550

not the verdict: solve rate, cut-off and refused replies
language            solved   cut-off   refused
Paka                100.0%         0         1
Rust                100.0%         0         0
TypeScript           75.0%         1         0

not the verdict: input tokens
language             input  without reference
Paka                 20000              13000
Rust                 20000              20000
TypeScript           20000              20000

not the verdict: Paka against TypeScript
first-try gap       25.00 points  lost
edits gap                 -1.250  held
token ratio                1.364  held

unfinished, not counted: D-TypeScript-2
";

#[test]
fn two_dropped_tasks_give_the_verdict_of_the_rest() {
    let out = fixture("rulings-2.txt", Some("1000"));
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), TWO_DROPPED);
}

/// Without the reference's token count only Paka's input without it is missing.
#[test]
fn input_without_the_reference_needs_its_token_count() {
    let out = fixture("rulings-2.txt", None);
    assert!(out.status.success());
    let input = section(&out, "not the verdict: input tokens");
    assert!(
        input.contains("Paka                 20000  needs the reference token count\n"),
        "{input}"
    );
}

/// Rulings 3: tasks A, C and D dropped, so only B is left, and a third dropped task makes the
/// verdict inconclusive although the metrics alone would give stop.
///
/// ```text
/// Paka  runs 2, first 1 (50.0%), edits 1 (0.50), out 350
/// Rust  runs 2, first 1 (50.0%), edits 1 (0.50), out 100+60=160
/// first-try gap = 0.00 points: held;  edits gap = 0.50 - 0.50 = 0.000: held
/// token ratio   = 350 / 160 = 2.1875: above 2.0: lost (shown 2.188)
/// metrics alone: stop; 3 dropped tasks: inconclusive
/// ```
#[test]
fn a_third_dropped_task_makes_the_verdict_inconclusive() {
    let out = fixture("rulings-3.txt", Some("1000"));
    assert!(out.status.success(), "{}", text(&out.stderr));
    let verdict = "\
verdict, Paka against Rust
dropped tasks: 3 (A, C, D)
first-try gap        0.00 points  held
edits gap                  0.000  held
token ratio                2.188  lost
inconclusive
";
    assert_eq!(section(&out, "verdict"), verdict);
}

/// Rulings 4: tasks A and B dropped, so C and D decide, but D has a TypeScript run that did not
/// end: no verdict, status 20, the tables still printed.
#[test]
fn an_incomplete_run_set_gives_no_verdict() {
    let out = fixture("rulings-4.txt", Some("1000"));
    assert_eq!(out.status.code(), Some(20), "{}", text(&out.stderr));
    let report = text(&out.stdout);
    assert!(report.starts_with("by language, dropped tasks removed\n"));
    assert!(!report.contains("verdict, Paka against Rust"));
    let none = "\
no verdict: incomplete run set
D: Paka 2, Rust 2, TypeScript 1
unfinished: D-TypeScript-2
";
    assert_eq!(section(&out, "no verdict"), none);
    assert!(report.ends_with("unfinished, not counted: D-TypeScript-2\n"));
}

// ---- small sets built on the fly ----

/// A runs folder and a rulings file in a temp folder, removed when dropped.
struct Case(PathBuf);

impl Case {
    fn new(name: &str) -> Case {
        let dir =
            std::env::temp_dir().join(format!("harness-metrics-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("runs")).unwrap();
        Case(dir)
    }

    fn log(&self, id: &str, lines: &[String]) {
        let attempt = self.0.join("runs").join(id).join("1");
        std::fs::create_dir_all(&attempt).unwrap();
        std::fs::write(attempt.join("log"), lines.join("\n") + "\n").unwrap();
    }

    /// A run that ended after `edits` edits (8: never passed) with `output` tokens.
    fn ended(&self, id: &str, edits: u32, output: u64) {
        self.log(
            id,
            &[r#"{"event":"start","at":1}"#.into(), end(edits, output)],
        );
    }

    /// Task T in the three languages, TypeScript as Rust; each pair is (edits, output).
    fn trio(&self, paka: &[(u32, u64)], rust: &[(u32, u64)]) {
        for (rep, &(edits, output)) in paka.iter().enumerate() {
            self.ended(&format!("T-Paka-{}", rep + 1), edits, output);
        }
        for (rep, &(edits, output)) in rust.iter().enumerate() {
            self.ended(&format!("T-Rust-{}", rep + 1), edits, output);
            self.ended(&format!("T-TypeScript-{}", rep + 1), edits, output);
        }
    }

    fn exec(&self, rulings: &str) -> Output {
        std::fs::write(self.0.join("rulings"), rulings).unwrap();
        exec(&self.0.join("runs"), &self.0.join("rulings"), None)
    }

    /// The lines of the verdict, whitespace squeezed, dropped-tasks line left out.
    fn verdict(&self) -> Vec<String> {
        let out = self.exec("");
        assert!(out.status.success(), "{}", text(&out.stderr));
        let squeeze = |l: &str| l.split_whitespace().collect::<Vec<_>>().join(" ");
        section(&out, "verdict, Paka")
            .lines()
            .skip(2)
            .map(squeeze)
            .collect()
    }

    /// The error of a run that must be refused; nothing is printed to stdout.
    fn error(&self, rulings: &str) -> String {
        let out = self.exec(rulings);
        assert!(
            !out.status.success() && out.stdout.is_empty(),
            "{}",
            text(&out.stdout)
        );
        text(&out.stderr)
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An end line the way trial.rs writes it, with 1000 input tokens.
fn end(edits: u32, output: u64) -> String {
    let passed = edits < 8;
    let submissions = if passed { edits + 1 } else { 8 };
    format!(
        r#"{{"event":"end","passed":{passed},"first_try":{},"submissions":{submissions},"edits":{edits},"input":1000,"output":{output},"cut_off":0,"at":3}}"#,
        edits == 0
    )
}

/// `n` runs of 100 tokens, the first `k` on the first try and the others after one edit.
fn first_tries(k: usize, n: usize) -> Vec<(u32, u64)> {
    (0..n).map(|i| (u32::from(i >= k), 100)).collect()
}

/// `n` runs of 100 tokens with `total` edits between them, at most 8 in a run.
fn edits_in(total: u32, n: u32) -> Vec<(u32, u64)> {
    (0..n)
        .map(|i| (total.saturating_sub(8 * i).min(8), 100))
        .collect()
}

/// One run each of `output` tokens against 1000.
fn tokens(output: u64) -> Case {
    let case = Case::new(&format!("tokens-{output}"));
    case.trio(&[(0, output)], &[(0, 1000)]);
    case
}

/// Paka as good as Rust on every line: go.
///
/// Both languages: 2 runs, first 1 (50.0%), edits 0+1=1, out 100+50=150: gaps 0, ratio 1.
#[test]
fn equal_languages_give_go() {
    let case = Case::new("go");
    case.trio(&[(0, 100), (1, 50)], &[(0, 100), (1, 50)]);
    let lines = [
        "first-try gap 0.00 points held",
        "edits gap 0.000 held",
        "token ratio 1.000 held",
        "go",
    ];
    assert_eq!(case.verdict(), lines);
}

/// No metric lost but one close: inconclusive, not go.
///
/// One run each: tokens 1800 / 1000 = 1.8, above 1.7 and up to 2.0: close; the rest held.
#[test]
fn a_close_metric_without_a_lost_one_is_inconclusive() {
    let lines = [
        "first-try gap 0.00 points held",
        "edits gap 0.000 held",
        "token ratio 1.800 close",
        "inconclusive",
    ];
    assert_eq!(tokens(1800).verdict(), lines);
}

/// First-try gap = Rust share - Paka share, over 100 runs each: Rust 100 of 100, Paka k of 100,
/// so the gap is 100 - k points. A gap of 10 or 20 points is still inside its band.
#[test]
fn first_try_gap_edges() {
    for (k, line) in [
        (90, "first-try gap 10.00 points held"),
        (89, "first-try gap 11.00 points close"),
        (80, "first-try gap 20.00 points close"),
        (79, "first-try gap 21.00 points lost"),
    ] {
        let case = Case::new(&format!("first-{k}"));
        case.trio(&first_tries(k, 100), &first_tries(100, 100));
        assert_eq!(case.verdict()[0], line);
    }
}

/// Edits gap = Paka mean - Rust mean, over 10 runs each: Rust 0 edits, Paka t in all, so the gap
/// is t / 10. A gap of 0.5 or 1.0 is still inside its band.
#[test]
fn edits_gap_edges() {
    for (total, line) in [
        (5, "edits gap 0.500 held"),
        (6, "edits gap 0.600 close"),
        (10, "edits gap 1.000 close"),
        (11, "edits gap 1.100 lost"),
    ] {
        let case = Case::new(&format!("edits-{total}"));
        case.trio(&edits_in(total, 10), &edits_in(0, 10));
        assert_eq!(case.verdict()[1], line);
    }
}

/// Token ratio = Paka tokens / Rust tokens, one run each, Rust 1000: a ratio of 1.7 or 2.0 is
/// still inside its band.
#[test]
fn token_ratio_edges() {
    for (output, line) in [
        (1700, "token ratio 1.700 held"),
        (1701, "token ratio 1.701 close"),
        (2000, "token ratio 2.000 close"),
        (2001, "token ratio 2.001 lost"),
    ] {
        assert_eq!(tokens(output).verdict()[2], line);
    }
}

#[test]
fn bad_input_is_refused_naming_the_culprit() {
    // A practice run, a folder that is no run, a stray file, and end lines that disagree.
    let case = Case::new("bad-folder");
    case.ended("X1-Paka-1", 0, 100);
    assert!(case.error("").contains("X1-Paka-1"));

    // a task the protocol lists neither as counted nor as reserve
    let case = Case::new("bad-task");
    case.ended("Z-Paka-1", 0, 100);
    assert!(case.error("").contains("Z-Paka-1"));

    let case = Case::new("bad-name");
    case.ended("notes", 0, 100);
    assert!(case.error("").contains("notes"));

    let case = Case::new("bad-file");
    std::fs::write(case.0.join("runs/readme"), "x").unwrap();
    assert!(case.error("").contains("readme"));

    // 2 edits but 1 submission: edits and submissions disagree
    let case = Case::new("bad-end");
    let skewed = end(2, 100).replace(r#""submissions":3"#, r#""submissions":1"#);
    case.log("T-Paka-1", &[r#"{"event":"start","at":1}"#.into(), skewed]);
    assert!(case.error("").contains("T-Paka-1"));

    // first_try true although the run passed on its second submission
    let case = Case::new("bad-first-try");
    let skewed = end(1, 100).replace(r#""first_try":false"#, r#""first_try":true"#);
    case.log("T-Rust-1", &[r#"{"event":"start","at":1}"#.into(), skewed]);
    assert!(case.error("").contains("T-Rust-1"));
}

#[test]
fn bad_rulings_are_refused() {
    let case = Case::new("rulings");
    case.trio(&[(0, 100)], &[(0, 100)]);
    for rulings in [
        "run T-Rust-9 void sandbox failure\n", // no such run
        "task Q dropped no such task\n",
        "task T removed\n",    // not a ruling
        "run T-Paka-1 void\n", // no reason
        "task T dropped\n",
        "run T-Paka-1 tagged\n",
    ] {
        case.error(rulings);
    }
}
