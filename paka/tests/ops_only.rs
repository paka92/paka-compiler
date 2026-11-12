//! Paka's Int and Float are the types of `paka_rt::ops`; no other source file may name an `f64`
//! or `f32`, so Float maths and rounding exist in one place (M2-03, M2-08).

use std::fs;
use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn mentions_float_type(text: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    // A literal's suffix counts: `1.5_f64` and `1f64` are the type, `buf64` and `my_f64` are not.
    let in_number = |c: char| c.is_ascii_digit() || c == '_' || c == '.';
    ["f64", "f32"].iter().any(|ty| {
        text.match_indices(ty).any(|(at, _)| {
            let before = text[..at].trim_end_matches(in_number).chars().next_back();
            let after = text[at + ty.len()..].chars().next();
            !before.is_some_and(char::is_alphabetic) && !after.is_some_and(is_word)
        })
    })
}

#[test]
fn only_the_ops_module_names_a_float_type() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = vec![];
    sources(&root.join("paka/src"), &mut files);
    sources(&root.join("paka-rt/src"), &mut files);
    let ops = root.join("paka-rt/src/ops");
    let bad: Vec<_> = files
        .iter()
        .filter(|f| !f.starts_with(&ops))
        .filter(|f| mentions_float_type(&fs::read_to_string(f).unwrap()))
        .collect();
    assert!(
        bad.is_empty(),
        "f64 or f32 outside paka-rt/src/ops: {bad:?}"
    );
}

#[test]
fn the_scan_sees_float_types_and_only_whole_words() {
    assert!(mentions_float_type("let x: f64 = 1.0;"));
    assert!(mentions_float_type("f32::MAX"));
    assert!(mentions_float_type("x(1.5_f64)") && mentions_float_type("x(1f32)"));
    assert!(!mentions_float_type(
        "let buf64 = 1; f640(); my_f64; 0x1f64"
    ));
}
