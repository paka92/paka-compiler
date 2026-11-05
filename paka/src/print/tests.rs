use super::program;
use crate::hash::hashes;
use crate::parser::parse;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn print(src: &str) -> String {
    program("main.paka", src.as_bytes()).unwrap()
}

#[test]
fn the_layout_samples_print_unchanged() {
    let samples = std::fs::read_to_string(root().join("tests/print/samples.paka")).unwrap();
    assert_eq!(print(&samples), samples);
}

#[test]
fn golden_files_print_as_expected() {
    let dir = root().join("tests/print");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "printed") {
            let input = std::fs::read_to_string(path.with_extension("paka")).unwrap();
            let expected = std::fs::read_to_string(&path).unwrap();
            assert_eq!(print(&input), expected, "{}", path.display());
            seen += 1;
        }
    }
    assert!(seen > 0);
}

#[test]
fn printing_is_idempotent_and_keeps_the_tree() {
    // The 8 MiB of a program's main thread; a debug test thread has only 2 MiB.
    std::thread::scope(|s| {
        let run = std::thread::Builder::new().stack_size(8 << 20);
        run.spawn_scoped(s, every_source).unwrap().join().unwrap();
    });
}

fn every_source() {
    let mut files = vec![];
    for dir in ["tests", "examples"] {
        walk(&root().join(dir), &mut files);
    }
    let mut printed = 0;
    for path in &files {
        let src = std::fs::read(path).unwrap();
        let Ok(before) = parse("main.paka", &src) else {
            continue;
        };
        let once = program("main.paka", &src).unwrap();
        let after = parse("main.paka", once.as_bytes()).unwrap();
        let reads = vec![Default::default(); before.defs.len()];
        let at = path.display();
        assert_eq!(hashes(&before, &reads), hashes(&after, &reads), "{at}");
        assert_eq!(program("main.paka", once.as_bytes()).unwrap(), once, "{at}");
        assert_eq!(comments(&src), comments(once.as_bytes()), "{at}");
        printed += 1;
    }
    assert!(printed > 1000, "only {printed} sources parse");
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else if path.extension().is_some_and(|e| e == "paka") {
            files.push(path);
        }
    }
}

/// How many comments a source has, so none is lost.
fn comments(src: &[u8]) -> usize {
    let p = super::Printer::new("main.paka", src).unwrap();
    p.comments.len()
}
