//! Each definition checked alone (M4-09): its result needs only the definitions it read, and the
//! read log is exact.

use paka::parser::split;
use paka::source::Source;
use paka::{DefCheck, check_each};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/conformance");

fn programs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            programs(&path, out);
        } else if path.file_name().unwrap() == "main.paka" {
            out.push(path);
        }
    }
}

fn each(text: &[u8]) -> Option<Vec<DefCheck>> {
    check_each("main.paka", &Source::file("main.paka", text.to_vec())).ok()
}

/// The definitions `name` reads, and those they read, in turn.
fn closure(defs: &[DefCheck], name: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::from([name.to_string()]);
    let mut todo = vec![name.to_string()];
    while let Some(next) = todo.pop() {
        let Some(def) = defs.iter().find(|d| d.name == next) else {
            continue;
        };
        todo.extend(
            def.reads
                .iter()
                .filter(|r| seen.insert(r.to_string()))
                .cloned(),
        );
    }
    seen
}

/// Every definition of every conformance program, checked in a program of only it and what it
/// read, gives the same error and the same reads as in the whole program. The other definitions
/// are blanked, line feeds kept, so positions stay put.
#[test]
fn a_definition_needs_only_what_it_read() {
    // The main thread's stack, which `paka` runs on; deep conformance cases need it.
    let run = std::thread::Builder::new().stack_size(8 << 20);
    run.spawn(alone_equals_whole).unwrap().join().unwrap();
}

fn alone_equals_whole() {
    let mut paths = vec![];
    programs(Path::new(ROOT), &mut paths);
    let mut checked = 0;
    for path in paths {
        let text = fs::read(&path).unwrap();
        let Some(whole) = each(&text) else { continue };
        let parts = split("main.paka", &text).unwrap();
        for def in &whole {
            let kept = closure(&whole, &def.name);
            let mut cut = text.clone();
            for &(_, part) in parts.iter().filter(|p| !kept.contains(p.0)) {
                let at = part.as_ptr() as usize - text.as_ptr() as usize;
                let blank = |b: &mut u8| *b = if *b == b'\n' { b'\n' } else { b' ' };
                cut[at..at + part.len()].iter_mut().for_each(blank);
            }
            let alone = each(&cut).unwrap_or_else(|| panic!("{path:?} {}", def.name));
            let found = alone.iter().find(|d| d.name == def.name).unwrap();
            let show = |d: &DefCheck| format!("{:?} {:?}", d.error, d.reads);
            assert_eq!(show(found), show(def), "{path:?} {}", def.name);
            checked += 1;
        }
    }
    assert!(checked > 2000, "{checked} definitions");
}

const LOG: &str = "\
type Point { x: Int = 0, y: Int = 0 }

type Box { at: Point, label: String = \"box\" }

enum Shape { Dot, Circle(r: Int) }

fn origin() -> Box {
  Box { at: Point {} }
}

fn depth() -> Int {
  origin().at.x
}

fn area(s: Shape) -> Int {
  match s {
    Dot => 0
    Circle(r) => r * r
  }
}

fn typo() -> Int {
  areaa(Dot)
}

fn dangling() -> Int {
  missing + 1
}

example dot {
  area(Dot) == 0
}
";

#[test]
fn the_read_log_lists_exactly_the_names_looked_up() {
    let defs = each(LOG.as_bytes()).unwrap();
    let expected: &[(&str, &[&str])] = &[
        ("Point", &[]),
        ("Box", &["Point"]),
        ("Shape", &[]),
        ("origin", &["Box", "Point"]),
        ("depth", &["Box", "Point", "origin"]),
        ("area", &["Shape"]),
        ("typo", &["area", "areaa"]),
        ("dangling", &["missing"]),
        ("dot", &["Shape", "area"]),
    ];
    let found: Vec<_> = defs.iter().map(|d| (d.name.as_str(), &d.reads)).collect();
    for ((name, reads), (want, want_reads)) in found.into_iter().zip(expected) {
        assert_eq!(name, *want);
        let want_reads: BTreeSet<String> = want_reads.iter().map(|r| r.to_string()).collect();
        assert_eq!(*reads, want_reads, "{name}");
    }
    assert_eq!(defs.len(), expected.len());
}
