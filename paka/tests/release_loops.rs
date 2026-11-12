//! The loop list of tests/loops.paka, built by `paka build` at full optimization: it runs as
//! written, and the off-by-one twin of each loop still traps at its item (M6-19).

#[allow(dead_code)]
mod common;

use common::{Scratch, build_native};
use std::process::{Command, Output};

const LOOPS: &str = include_str!("loops.paka");

/// Each loop's kernel, its bound as written, the bound one past the end, and the trap's site.
const TWINS: [(&str, &str, &str, &str); 6] = [
    ("total", "0..<len(xs)", "0..<len(xs) + 1", "9:14"),
    ("fill", "0..<n", "0..<n + 1", "17:7"),
    ("walk", "i < len(xs)", "i <= len(xs)", "27:14"),
    ("copy", "0..<len(xs)", "0..<len(xs) + 1", "37:16"),
    ("count", "2..<n", "2..<n + 1", "46:9"),
    ("clear", "0..<n", "0..<n + 1", "57:9"),
];

fn run(scratch: &Scratch, source: &str) -> Output {
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    std::fs::write(&path, source).unwrap();
    let built = build_native(path.to_str().unwrap(), &exe);
    assert!(built.status.success(), "{built:?}");
    Command::new(&exe).output().unwrap()
}

/// `LOOPS` with the first `from` in kernel `name` made `to`.
fn twin(name: &str, from: &str, to: &str) -> String {
    let start = LOOPS.find(&format!("fn {name}(")).unwrap();
    let at = start + LOOPS[start..].find(from).unwrap();
    format!("{}{to}{}", &LOOPS[..at], &LOOPS[at + from.len()..])
}

#[test]
fn the_loop_list_runs_and_each_off_by_one_twin_traps() {
    let scratch = Scratch::new("release-loops");
    let out = run(&scratch, LOOPS);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "11 4 7 9\n");
    for (name, from, to, site) in TWINS {
        let out = run(&scratch, &twin(name, from, to));
        let stderr = String::from_utf8(out.stderr).unwrap();
        let want = format!("loops: trap out_of_bounds at main.paka:{site}\n");
        assert_eq!((out.status.code(), stderr), (Some(3), want), "{name}");
    }
}
