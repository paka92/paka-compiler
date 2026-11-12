//! Holes and examples as machine code (M4-30): an unreached hole costs nothing, a reached one
//! traps naming it, and every example's status equals the interpreter's.

mod common;

use paka::core_form::Program;
use paka_installer::Slots;
use paka_rt::abi::dev::{self as rt, Context, on_runner};
use std::fs;
use std::path::Path;

const HOLES: &str = "\
fn pick(n: Int) -> Int { if n > 0 { n } else { ??negative } }
fn unreached() -> Int { pick(3) }
fn reached() -> Int { pick(-1) }
";

#[test]
fn an_unreached_hole_runs_and_a_reached_one_traps_naming_it() {
    let (p, slots) = common::load(HOLES);
    let run = |id| {
        let mut ctx = Context::new(0);
        (common::run(&slots, id, &mut ctx), ctx.trap)
    };
    let ((status, out), _) = run(1);
    assert_eq!((status, out), (0, 3));
    let ((status, _), trap) = run(2);
    assert_eq!((status, trap.kind), (1, rt::HOLE));
    let hole = p.place(trap.site).1;
    assert_eq!(hole.hole.as_deref(), Some("negative"));
    let caller = |&at: &u64| p.place(at).0.name.as_str();
    assert_eq!(caller(&trap.site), "pick");
    assert_eq!(
        trap.frames.iter().map(caller).collect::<Vec<_>>(),
        ["reached"]
    );
}

/// A heap no fixture fills.
const HEAP: u64 = 1 << 30;

/// Each example's status word, as `paka examples` prints it (M2-06 section 1.6).
fn statuses(p: &Program, slots: &Slots) -> Vec<(String, &'static str)> {
    let example = |(i, f): (usize, &paka::core_form::Fn)| {
        let id = p.fns.len() + i;
        let (run, kind, ledger) = on_runner(HEAP, |ctx| {
            let run = common::run(slots, id, ctx);
            let ledger = (ctx.heap.leaks().len(), ctx.heap.faults().len());
            (run, ctx.trap.kind, ledger)
        });
        // A run that returns frees every block it made; a trap or a stop leaves them to its end.
        if run.0 == 0 {
            assert_eq!(ledger, (0, 0), "{} leaks or faults", f.name);
        }
        let status = match (run, kind) {
            ((0, 1), _) => "pass",
            ((0, _), _) => "fail",
            ((1, _), rt::HOLE) => "unfinished",
            _ => "trapped",
        };
        (f.name.clone(), status)
    };
    let mut all: Vec<_> = p.examples.iter().enumerate().map(example).collect();
    all.sort();
    all
}

/// The status lines of a hand-written `paka examples` stdout: name, label, status.
fn expected(stdout: &str) -> Vec<(String, &str)> {
    let lines = stdout.lines().filter(|l| !l.starts_with(' '));
    let mut all: Vec<_> = lines
        .map(|l| {
            let mut words = l.split(' ');
            (words.next().unwrap().to_string(), words.nth(1).unwrap())
        })
        .collect();
    all.sort();
    all
}

#[test]
fn example_statuses_equal_the_interpreters() {
    let apps = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/conformance/apps");
    let mut compiled = vec![];
    let mut refused = vec![];
    for dir in fs::read_dir(apps).unwrap().map(|d| d.unwrap().path()) {
        let command = fs::read_to_string(dir.join("command")).unwrap_or_default();
        if command.trim() != "examples main.paka" || dir.join("reject").exists() {
            continue;
        }
        let source = fs::read_to_string(dir.join("main.paka")).unwrap();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let (p, slots) = match common::try_load(&source) {
            Ok(loaded) => loaded,
            Err(why) => {
                assert!(!why.contains("error["), "{name} checks: {why}");
                refused.push(format!("{name}: {why}"));
                continue;
            }
        };
        if !p.ui_examples.is_empty() {
            refused.push(format!("{name}: UI examples run in the interpreter"));
            continue;
        }
        let stdout = fs::read_to_string(dir.join("stdout")).unwrap();
        assert_eq!(statuses(&p, &slots), expected(&stdout), "{name}");
        compiled.push(name);
    }
    println!("compiled: {compiled:?}\nrefused:\n{}", refused.join("\n"));
    // One program for each status besides pass, and those of records and enums (M4-31), of
    // `?`, `unwrap_or` and `ok_or` (M4-31b), of Strings and Lists (M4-32), and of function values
    // (M4-37b); the rest wait on the later emitter tasks.
    let each = [
        "examples_records",
        "examples_errors",
        "examples_text",
        "examples_text_traps",
        "examples_lists",
        "examples_list_traps",
        "examples_owned",
        "examples_inout",
        "examples_fn_values",
        "examples_trapped",
        "examples_fail_comparisons",
        "examples_unfinished",
        "examples_assert_is_trapped",
    ];
    for name in each {
        assert!(compiled.iter().any(|c| c == name), "{name} compiles");
    }
}
