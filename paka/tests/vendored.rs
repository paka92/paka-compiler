//! Every outside crate in a lock file is listed in .cargo/approved-crates.txt and vendored (M4-06).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type Crates = BTreeSet<(String, String)>;

const LOCKS: [&str; 2] = ["Cargo.lock", "bench/generators/Cargo.lock"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?
        .strip_prefix(" = \"")?
        .strip_suffix('"')
}

/// The (name, version) of each package taken from a registry, in a Cargo.lock text.
fn outside(lock: &str) -> Crates {
    let mut crates = Crates::new();
    for package in lock.split("[[package]]").skip(1) {
        let field = |key| package.lines().find_map(|l| value(l, key));
        if field("source").is_some() {
            crates.insert((
                field("name").unwrap().into(),
                field("version").unwrap().into(),
            ));
        }
    }
    crates
}

/// The (name, version) of a Cargo.toml's [package] section.
fn package(toml: &str) -> (String, String) {
    let section = toml.split("[package]").nth(1).unwrap();
    let section = section.split("\n[").next().unwrap();
    let field = |key| {
        section
            .lines()
            .find_map(|l| value(l, key))
            .unwrap()
            .to_string()
    };
    (field("name"), field("version"))
}

fn unlisted(locked: &Crates, approved: &Crates) -> Vec<String> {
    locked
        .difference(approved)
        .map(|(n, v)| format!("{n} {v}"))
        .collect()
}

fn approved() -> Crates {
    let list = fs::read_to_string(root().join(".cargo/approved-crates.txt")).unwrap();
    list.lines()
        .map(|l| {
            let (n, v) = l.split_once(' ').unwrap();
            (n.into(), v.into())
        })
        .collect()
}

#[test]
fn every_locked_crate_is_approved() {
    let approved = approved();
    for lock in LOCKS {
        let locked = outside(&fs::read_to_string(root().join(lock)).unwrap());
        assert_eq!(unlisted(&locked, &approved), Vec::<String>::new(), "{lock}");
    }
}

#[test]
fn the_vendor_directory_holds_exactly_the_approved_crates() {
    let vendored: Crates = fs::read_dir(root().join("vendor"))
        .unwrap()
        .map(|e| package(&fs::read_to_string(e.unwrap().path().join("Cargo.toml")).unwrap()))
        .collect();
    assert_eq!(vendored, approved());
}

#[test]
fn an_unlisted_crate_fails() {
    let lock = "[[package]]\nname = \"paka\"\nversion = \"0.0.0\"\n\n\
                [[package]]\nname = \"left-pad\"\nversion = \"1.0.0\"\n\
                source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    assert_eq!(unlisted(&outside(lock), &approved()), ["left-pad 1.0.0"]);
}
