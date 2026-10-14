//! The M2 exit gate (tests/conformance/README.md, "Exit gate"): every reference sample checks and
//! runs, the reference fits its size budget, and every syntax node kind and built-in occurs in a
//! passing conformance case. The self-tests plant one fault each and expect the gate to fail.

use super::*;
use common::tokens;
use paka::coverage::{self, Uses};
use paka::parser::parse;

const BUDGET: usize = 8000;
const REFERENCE: &str = include_str!("../../../spec/paka.md");

/// The blocks fenced as `paka`, each ended by a line feed.
fn samples(reference: &str) -> Vec<String> {
    let blocks = reference.split("\n```paka\n").skip(1);
    blocks.map(|b| b.split("\n```").next().unwrap().to_string() + "\n").collect()
}

/// `paka check` and `paka examples` exit 0; a sample with an app is also run on a file.
fn sample_runs(number: usize, source: &str) -> Result<(), String> {
    let scratch = Scratch::new(&format!("sample-{number}"));
    let dir = scratch.path("");
    fs::write(dir.join("main.paka"), source).unwrap();
    fs::write(dir.join("words.txt"), "a b c").unwrap();
    let app = source.lines().any(|l| l.starts_with("app "));
    let runs: [(bool, &[&str]); 3] = [
        (true, &["check", "main.paka"]),
        (true, &["examples", "main.paka"]),
        (app, &["run", "main.paka", "words.txt"]),
    ];
    for (_, args) in runs.iter().filter(|(wanted, _)| *wanted) {
        status_is(&paka(&dir, args), 0).map_err(|e| format!("paka {}: {e}", args.join(" ")))?;
    }
    Ok(())
}

/// A case shows a construct at work when it passes and is not a rejection.
fn counts(case: &Path) -> bool {
    read(case, "reject").is_none() && read(case, "expect.json").is_none()
}

/// What the passing cases use: the node kinds and names of each `main.paka` that parses.
fn used_by(cases: &[&Path]) -> Uses {
    let mut used = Uses::default();
    for case in cases.iter().filter(|c| counts(c)) {
        let source = read(case, "main.paka").unwrap();
        if let Ok(program) = parse("main.paka", &source) {
            used.merge(coverage::uses(&program));
        }
    }
    used
}

/// Every way the gate fails, one message each; empty when it passes.
fn problems(reference: &str, kinds: &[&str], builtins: &[&str], used: &Uses) -> Vec<String> {
    let mut problems = vec![];
    let size = tokens(reference);
    if size > BUDGET {
        problems.push(format!("the reference is {size} tokens, limit {BUDGET}"));
    }
    for (i, source) in samples(reference).iter().enumerate() {
        if let Err(e) = sample_runs(i + 1, source) {
            problems.push(format!("reference sample {} fails: {e}", i + 1));
        }
    }
    let unused: Vec<_> = kinds.iter().filter(|k| !used.kinds.contains(**k)).collect();
    if !unused.is_empty() {
        problems.push(format!("{} node kinds in no passing case: {unused:?}", unused.len()));
    }
    let unused: Vec<_> = builtins.iter().filter(|b| !used.names.contains(**b)).collect();
    if !unused.is_empty() {
        problems.push(format!("{} built-ins in no passing case: {unused:?}", unused.len()));
    }
    problems
}

/// The gate. Ignored until M2-45 closes the gaps it reports; then the attribute goes.
#[test]
#[ignore = "reports gaps until M2-45 closes them"]
fn exit_gate() {
    let passing: Vec<_> = results().iter().filter(|(_, r)| r.is_ok()).collect();
    let passing: Vec<_> = passing.iter().map(|(case, _)| case.as_path()).collect();
    let problems = problems(
        REFERENCE,
        coverage::NODE_KINDS,
        &coverage::builtin_names(),
        &used_by(&passing),
    );
    assert!(problems.is_empty(), "exit gate:\n{}", problems.join("\n"));
}

mod self_test {
    use super::*;

    const GOOD: &str = "# Reference\n\n```paka\nfn f(x: String) -> Int {\n  len(x)\n}\n\nexample e {\n  f(\"ab\") == 2\n}\n```\n";
    const BAD: &str = "```paka\nfn f( {\n```\n";

    fn used() -> Uses {
        let source = "fn f(x: String) -> Int {\n  len(x)\n}\n";
        coverage::uses(&parse("main.paka", source.as_bytes()).unwrap())
    }

    #[test]
    fn a_complete_small_gate_passes() {
        let found = problems(GOOD, &["Def::Fn", "ExprKind::Call"], &["len", "String"], &used());
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn an_unused_node_kind_fails() {
        let found = problems(GOOD, &["Def::Fn", "ExprKind::Planted"], &["len"], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("ExprKind::Planted"), "{found:?}");
    }

    #[test]
    fn an_unused_built_in_fails() {
        let found = problems(GOOD, &["Def::Fn"], &["len", "planted"], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("planted"), "{found:?}");
    }

    #[test]
    fn a_failing_sample_fails() {
        let found = problems(BAD, &[], &[], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("reference sample 1 fails"), "{found:?}");
    }

    #[test]
    fn a_sample_that_does_not_run_fails() {
        let trapping = "```paka\nexample e {\n  1 / 0 == 0\n}\n```\n";
        assert_eq!(problems(trapping, &[], &[], &used()).len(), 1);
    }

    #[test]
    fn an_oversize_reference_fails() {
        let found = problems(&"a".repeat(3 * BUDGET + 1), &[], &[], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("limit 8000"), "{found:?}");
    }

    #[test]
    fn only_paka_blocks_are_samples() {
        let text = "```\nnot a sample\n```\n\n```paka\nfn f() {}\n```\n";
        assert_eq!(samples(text), ["fn f() {}\n"]);
    }

    #[test]
    fn rejections_count_for_nothing() {
        let case = |name: &str| Path::new(ROOT).join(name);
        assert!(counts(&case("core/bool_operators")));
        assert!(!counts(&case("core/arm_after_wildcard")));
        assert!(!counts(&case("errors/assign_to_let")));
    }

    #[test]
    fn the_real_lists_are_not_empty() {
        assert!(coverage::NODE_KINDS.contains(&"ExprKind::Call"));
        assert!(coverage::NODE_KINDS.contains(&"Def::App"));
        let names = coverage::builtin_names();
        assert!(["len", "Console", "Option", "ExitWith"].iter().all(|n| names.contains(n)));
        assert_eq!(names.iter().filter(|n| **n == "len").count(), 1);
    }
}
