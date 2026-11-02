//! Every outside crate in a lock file is listed in `TRUSTED` and vendored (M4-06, M7-16), and so
//! is every system library a release links or its code loads at run time.
//! Crates a lock names only for other systems are listed in .cargo/stubbed-crates.txt and vendored
//! as a manifest with no code, so cargo can resolve offline without them ever being built.
//! The shipped font is the pinned Noto Sans file (M5-10).

use paka::clang::SYSTEM_LIBS;
use paka::notices::{Row, notices, rows};
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

fn trusted() -> String {
    fs::read_to_string(root().join("TRUSTED")).unwrap()
}

/// The vendored crates `TRUSTED` names: neither system libraries nor files.
fn approved() -> Crates {
    let text = trusted();
    let rows = rows(&text).unwrap();
    (rows.iter().filter(|r| !r.is_system() && !r.is_file()))
        .map(|r| (r.name.into(), r.version.into()))
        .collect()
}

/// Libraries vendored code can load at run time that paka's features leave out.
const NOT_LOADED: [&str; 3] = [
    "libwayland-server", // wayland-sys feature `server`, off
    "libwayland-egl",    // wayland-sys feature `egl`, off
    "libxkbcommon-x11",  // xkbcommon-dl feature `x11`, off
];

/// The names in `libs` that `rows` lists as no system library.
fn unlisted_system(libs: &[String], rows: &[Row]) -> Vec<String> {
    let listed = |name: &str| rows.iter().any(|r| r.is_system() && r.name == name);
    libs.iter().filter(|name| !listed(name)).cloned().collect()
}

fn linked() -> Vec<String> {
    let lib = |flag: &str| format!("lib{}", flag.strip_prefix("-l").unwrap());
    SYSTEM_LIBS.iter().map(|f| lib(f)).collect()
}

/// The library names of the `"lib<name>.so…"` strings on the code lines of a Rust text.
fn loaded(code: &str) -> BTreeSet<String> {
    (code.lines())
        .filter(|l| !l.trim_start().starts_with("//"))
        .flat_map(|l| l.split('"').skip(1).step_by(2))
        .filter_map(|s| Some(s.split_once(".so")?.0).filter(|n| n.starts_with("lib")))
        .map(String::from)
        .collect()
}

/// The `.rs` files under `dir`.
fn sources(dir: &Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let paths = entries.map(|e| e.unwrap().path());
    paths
        .flat_map(|p| match p.is_dir() {
            true => sources(&p),
            false => (p.extension() == Some("rs".as_ref())).then_some(p).into_iter().collect(),
        })
        .collect()
}

fn stubbed() -> Crates {
    let list = fs::read_to_string(root().join(".cargo/stubbed-crates.txt")).unwrap();
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
fn every_linked_system_library_is_trusted() {
    let text = trusted();
    let rows = rows(&text).unwrap();
    assert_eq!(unlisted_system(&linked(), &rows), Vec::<String>::new());
}

#[test]
fn an_unlisted_system_library_fails() {
    let text = trusted();
    let rows = rows(&text).unwrap();
    let libs = ["libc".into(), "libfoo".into()];
    assert_eq!(unlisted_system(&libs, &rows), ["libfoo"]);
}

#[test]
fn every_library_program_code_loads_is_trusted() {
    let text = trusted();
    let rows = rows(&text).unwrap();
    let crates = rows.iter().filter(|r| r.runs == "programs" && !r.is_system() && !r.is_file());
    let files = crates.flat_map(|r| sources(&r.dir(root()).join("src")));
    let mut found = BTreeSet::new();
    for file in files {
        found.extend(loaded(&fs::read_to_string(file).unwrap_or_default()));
    }
    for name in NOT_LOADED {
        assert!(found.remove(name), "{name} is no longer in the code");
    }
    let found = Vec::from_iter(found);
    assert_eq!(found, ["libwayland-client", "libwayland-cursor", "libxkbcommon"]);
    assert_eq!(unlisted_system(&found, &rows), Vec::<String>::new());
}

#[test]
fn a_loaded_library_is_found_in_code_not_comments() {
    let code = "/// dlopen(\"libdoc.so.1\")\nlet x = open(\"libfoo.so.2\", \"lib.rs\");\n";
    assert_eq!(Vec::from_iter(loaded(code)), ["libfoo"]);
}

#[test]
fn the_notices_are_made_from_trusted() {
    let made = notices(root()).unwrap();
    assert!(made.contains("\nlibc system  -  C runtime  (from the system, not shipped)\n"));
    assert!(made.contains("\ngui/fonts/NotoSans-Regular.ttf 2.015  OFL-1.1  the font\n"));
    assert!(!made.contains("\ncranelift-codegen "), "a toolchain row");
    let supplied = "\naccesskit 0.25.2  MIT, not in its package: vendor/ahash/LICENSE-MIT with the \
                    copyright line from its authors\n";
    assert!(made.contains(supplied));
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
