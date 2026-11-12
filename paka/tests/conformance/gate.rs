//! The M2 exit gate (tests/conformance/README.md, "Exit gate"): every reference sample checks and
//! runs, each reference page fits its size budget, and every syntax node kind and built-in occurs in a
//! passing conformance case. The self-tests plant one fault each and expect the gate to fail.

use super::*;
use common::tokens;
use paka::coverage::{self, Uses};
use paka::parser::parse;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The reference pages: name, text and token budget.
type Page<'a> = (&'a str, &'a str, usize);

const PAGES: [Page<'static>; 2] = [
    ("spec/paka.md", include_str!("../../../spec/paka.md"), 8000),
    (
        "spec/screens.md",
        include_str!("../../../spec/screens.md"),
        2500,
    ),
];

/// The blocks fenced as `paka`, each ended by a line feed.
fn samples(reference: &str) -> Vec<String> {
    let reference = format!("\n{reference}");
    let blocks = reference.split("\n```paka\n").skip(1);
    blocks
        .map(|b| b.split("\n```").next().unwrap().to_string() + "\n")
        .collect()
}

/// `paka check` and `paka examples` exit 0; a sample with an app is also run on a file.
fn sample_runs(source: &str) -> Result<(), String> {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let scratch = Scratch::new(&format!("sample-{}", RUNS.fetch_add(1, Ordering::Relaxed)));
    let dir = scratch.path("");
    fs::write(dir.join("main.paka"), source).unwrap();
    fs::write(dir.join("words.txt"), "a b c").unwrap();
    let app = source.lines().any(|l| l.starts_with("app "));
    // `paka run` does not run a visual app yet (`no-app`).
    let visual = source.lines().any(|l| l.trim_start().starts_with("view:"));
    let runs: [(bool, &[&str]); 3] = [
        (true, &["check", "main.paka"]),
        (true, &["examples", "main.paka"]),
        (app && !visual, &["run", "main.paka", "words.txt"]),
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
fn problems(pages: &[Page<'_>], kinds: &[&str], builtins: &[&str], used: &Uses) -> Vec<String> {
    let mut problems = vec![];
    for (name, text, budget) in pages {
        let size = tokens(text);
        if size > *budget {
            problems.push(format!("{name} is {size} tokens, limit {budget}"));
        }
        for (i, source) in samples(text).iter().enumerate() {
            if let Err(e) = sample_runs(source) {
                problems.push(format!("{name} sample {} fails: {e}", i + 1));
            }
        }
    }
    let unused: Vec<_> = kinds.iter().filter(|k| !used.kinds.contains(**k)).collect();
    if !unused.is_empty() {
        problems.push(format!(
            "{} node kinds in no passing case: {unused:?}",
            unused.len()
        ));
    }
    let unused: Vec<_> = builtins
        .iter()
        .filter(|b| !used.names.contains(**b))
        .collect();
    if !unused.is_empty() {
        problems.push(format!(
            "{} built-ins in no passing case: {unused:?}",
            unused.len()
        ));
    }
    problems
}

/// The gate: fails with every problem listed.
#[test]
fn exit_gate() {
    let passed = results().iter().filter(|(_, result)| result.is_ok());
    let passing: Vec<_> = passed.map(|(case, _)| case.as_path()).collect();
    let problems = problems(
        &PAGES,
        coverage::NODE_KINDS,
        &coverage::builtin_names(),
        &used_by(&passing),
    );
    assert!(problems.is_empty(), "exit gate:\n{}", problems.join("\n"));
}

mod self_test {
    use super::*;

    const GOOD: &[Page<'static>] = &[("ref", GOOD_TEXT, 8000)];
    const GOOD_TEXT: &str = "# Reference\n\n```paka\nfn f(x: String) -> Int {\n  len(x)\n}\n\nexample e {\n  f(\"ab\") == 2\n}\n```\n";

    fn page(text: &str) -> [Page<'_>; 1] {
        [("ref", text, 8000)]
    }

    fn used() -> Uses {
        let source = "fn f(x: String) -> Int {\n  len(x)\n}\n";
        coverage::uses(&parse("main.paka", source.as_bytes()).unwrap())
    }

    #[test]
    fn a_complete_small_gate_passes() {
        let found = problems(
            GOOD,
            &["Def::Fn", "ExprKind::Call"],
            &["len", "String"],
            &used(),
        );
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
        let found = problems(&page("```paka\nfn f( {\n```\n"), &[], &[], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("ref sample 1 fails"), "{found:?}");
    }

    #[test]
    fn a_sample_that_does_not_run_fails() {
        let trapping = "```paka\nexample e {\n  1 / 0 == 0\n}\n```\n";
        let found = problems(&page(trapping), &[], &[], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("paka examples"), "{found:?}");
    }

    #[test]
    fn an_app_sample_whose_run_fails_fails() {
        let exiting = "```paka\ntype T { n: Int = 0 }\n\nenum Msg { Start(path: String) }\n\n\
            fn update(s: inout T, m: Msg) -> List<Effect<Msg>> {\n  [ExitWith(7)]\n}\n\n\
            app a {\n  state: T\n  start: Start\n  update: update\n  grants: [Console]\n}\n```\n";
        let found = problems(&page(exiting), &[], &[], &used());
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("paka run"), "{found:?}");
    }

    #[test]
    fn a_local_name_does_not_cover_a_built_in_function() {
        let source = "fn f(text: String) -> Int {\n  let words = 1\n  words\n}\n";
        let program = parse("main.paka", source.as_bytes()).unwrap();
        assert!(!coverage::uses(&program).names.contains("words"));
        let source = "fn f(text: String) -> Int {\n  len(words(text))\n}\n";
        let program = parse("main.paka", source.as_bytes()).unwrap();
        let used = coverage::uses(&program);
        assert!(used.names.contains("words") && used.names.contains("len"));
    }

    #[test]
    fn operators_are_node_kinds() {
        let source = "fn f(a: Int) -> Int {\n  var b = -a\n  b += a % 2\n  b\n}\n";
        let program = parse("main.paka", source.as_bytes()).unwrap();
        let kinds = coverage::uses(&program).kinds;
        for kind in ["UnOp::Neg", "AssignOp::Add", "BinOp::Rem"] {
            assert!(kinds.contains(kind), "{kind}");
        }
    }

    #[test]
    fn an_oversize_reference_fails() {
        let text = "a".repeat(3 * 8000 + 1);
        let found = problems(&[("ref", &text, 8000)], &[], &[], &used());
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
        assert!(
            ["len", "Console", "Option", "ExitWith"]
                .iter()
                .all(|n| names.contains(n))
        );
        assert_eq!(names.iter().filter(|n| **n == "len").count(), 1);
    }
}
