//! Every outside crate in a lock file is listed in .cargo/approved-crates.txt and vendored (M4-06).
//! Crates a lock names only for other systems are listed in .cargo/stubbed-crates.txt and vendored
//! as a manifest with no code, so cargo can resolve offline without them ever being built.
//! The shipped font is the pinned Noto Sans file (M5-10).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type Crates = BTreeSet<(String, String)>;

const LOCKS: [&str; 3] = [
    "Cargo.lock",
    "bench/generators/Cargo.lock",
    "trial/gui/Cargo.lock",
];

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
    listed(".cargo/approved-crates.txt")
}

fn stubbed() -> Crates {
    listed(".cargo/stubbed-crates.txt")
}

fn listed(file: &str) -> Crates {
    let list = fs::read_to_string(root().join(file)).unwrap();
    list.lines()
        .map(|l| {
            let (n, v) = l.split_once(' ').unwrap();
            (n.into(), v.into())
        })
        .collect()
}

/// Whether a vendored directory holds no code: every file but the manifest and checksum is empty.
fn is_stub(dir: &Path) -> bool {
    fs::read_dir(dir).unwrap().all(|e| {
        let path = e.unwrap().path();
        let name = path.file_name().unwrap();
        if path.is_dir() {
            is_stub(&path)
        } else {
            name == "Cargo.toml"
                || name == ".cargo-checksum.json"
                || path.metadata().unwrap().len() == 0
        }
    })
}

#[test]
fn every_locked_crate_is_approved() {
    let approved: Crates = approved().union(&stubbed()).cloned().collect();
    for lock in LOCKS {
        let locked = outside(&fs::read_to_string(root().join(lock)).unwrap());
        assert_eq!(unlisted(&locked, &approved), Vec::<String>::new(), "{lock}");
    }
}

#[test]
fn the_vendor_directory_holds_exactly_the_approved_crates() {
    let (mut real, mut stubs) = (Crates::new(), Crates::new());
    for entry in fs::read_dir(root().join("vendor")).unwrap() {
        let dir = entry.unwrap().path();
        let crate_ = package(&fs::read_to_string(dir.join("Cargo.toml")).unwrap());
        if is_stub(&dir) { &mut stubs } else { &mut real }.insert(crate_);
    }
    assert_eq!(real, approved());
    assert_eq!(stubs, stubbed());
}

#[test]
fn an_unlisted_crate_fails() {
    let lock = "[[package]]\nname = \"paka\"\nversion = \"0.0.0\"\n\n\
                [[package]]\nname = \"left-pad\"\nversion = \"1.0.0\"\n\
                source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    assert_eq!(unlisted(&outside(lock), &approved()), ["left-pad 1.0.0"]);
}

#[test]
fn the_font_is_the_pinned_noto_sans() {
    // NotoSans/unhinted/ttf/NotoSans-Regular.ttf of the notofonts NotoSans-v2.015 release.
    const PINNED: &str = "f3961a9cde016d41a4879aecda1474d3a36d6bf54fa0e4643de029cc2248b0e8";
    let font = fs::read(root().join("gui/fonts/NotoSans-Regular.ttf")).unwrap();
    let hex: String = paka::sha256::sha256(&font)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex, PINNED);
}
