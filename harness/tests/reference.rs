//! Every reference solution passes every case of its task, except the cases its task lists as
//! disputed. A dispute that no longer fails is reported too, so the list stays current.

#[allow(dead_code)] // this test uses only the runner
mod common;

use harness::runner::{CaseResult, Lang, Verdict};
use harness::task::Task;
use std::fs;
use std::path::Path;
use std::time::Duration;

const LANGS: [(Lang, &str); 3] = [
    (Lang::Paka, "main.paka"),
    (Lang::Rust, "main.rs"),
    (Lang::TypeScript, "main.ts"),
];

#[test]
fn reference_solutions_pass_all_undisputed_cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../experiment");
    let Ok(dirs) = fs::read_dir(root.join("reference")) else {
        return;
    };
    let mut runner = common::runner();
    runner.limits.case = Duration::from_secs(10);
    let mut problems = vec![];
    for dir in dirs {
        let dir = dir.unwrap().path();
        let id = dir.file_name().unwrap().to_string_lossy().into_owned();
        let task = Task::load(&root.join("tasks").join(&id)).unwrap();
        let disputed = fs::read_to_string(dir.join("disputes")).unwrap_or_default();
        for (lang, file) in LANGS {
            let Ok(source) = fs::read_to_string(dir.join(file)) else {
                continue;
            };
            let listed = |case: &str| {
                disputed
                    .lines()
                    .any(|l| l.split(' ').take(2).eq([file, case]))
            };
            let report = runner.judge(lang, &source, &task).unwrap();
            if let Err(text) = &report.build {
                problems.push(format!("{id} {file}: build failed\n{text}"));
                continue;
            }
            let cases = named(&report.visible, "cases").chain(named(&report.hidden, "hidden"));
            for (case, result) in cases {
                let failed = result.verdict != Verdict::Pass;
                if failed != listed(&case) {
                    let what = if failed {
                        "fails"
                    } else {
                        "passes but is disputed"
                    };
                    problems.push(format!("{id} {file} {case}: {what} ({:?})", result.verdict));
                }
            }
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

fn named<'a>(
    results: &'a [CaseResult],
    folder: &'a str,
) -> impl Iterator<Item = (String, &'a CaseResult)> {
    results
        .iter()
        .map(move |r| (format!("{folder}/{}", r.name), r))
}
