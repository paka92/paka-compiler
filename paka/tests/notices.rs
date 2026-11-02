//! The third-party notices file (M7-16): made from `TRUSTED` and the licence texts in `vendor/`,
//! the rows a program runs in, in the order of `TRUSTED`.

#[allow(dead_code)]
mod common;

use common::Scratch;
use paka::notices::{notices, rows};
use std::fs;

const TRUSTED: &str = "\
# name          version  licence            runs in    purpose
ahash           1.0.0    MIT OR Apache-2.0  programs   hashing
tool            2.0.0    MIT                toolchain  dev-mode code
bare            0.1.0    MIT OR Apache-2.0  programs   used by ahash
libc            system   -                  programs   C runtime
fonts/Demo.ttf  1.0      OFL-1.1            programs   the font
";

const NOTICES: &str = "\
Third-party notices: each library and file in TRUSTED that a program contains or loads, in that
order, then each distinct licence text once, after the list of those it covers.

ahash 1.0.0  MIT OR Apache-2.0  hashing
bare 0.1.0  MIT OR Apache-2.0  used by ahash
libc system  -  C runtime  (from the system, not shipped)
fonts/Demo.ttf 1.0  OFL-1.1  the font

==== text 1, for:
ahash 1.0.0  LICENSE-APACHE
bare 0.1.0  Apache-2.0, not in its package: the standard text from vendor/ahash/LICENSE-APACHE
----
Apache text

==== text 2, for:
ahash 1.0.0  LICENSE-MIT
----
Copyright (c) 2018 Someone

MIT text

==== text 3, for:
bare 0.1.0  MIT, not in its package: vendor/ahash/LICENSE-MIT with the copyright line from its authors
----
Copyright (c) A, B

MIT text

==== text 4, for:
fonts/Demo.ttf 1.0  OFL.txt
----
OFL text
";

/// A tree with `TRUSTED` and the files its rows name.
fn tree(name: &str) -> Scratch {
    let tree = Scratch::new(name);
    let files = [
        ("TRUSTED", TRUSTED),
        ("vendor/ahash/Cargo.toml", "[package]\n"),
        ("vendor/ahash/LICENSE-MIT", "Copyright (c) 2018 Someone\n\nMIT text\n"),
        ("vendor/ahash/LICENSE-APACHE", "Apache text"),
        ("vendor/ahash/src/lib.rs", ""),
        ("vendor/tool/LICENSE", "tool text\n"),
        ("vendor/bare-0.1.0/Cargo.toml", "[package]\nauthors = [\"A\", \"B\"]\n"),
        ("vendor/bare/LICENSE", "another version's text\n"),
        ("fonts/Demo.ttf", "font"),
        ("fonts/OFL.txt", "OFL text\n"),
    ];
    for (path, text) in files {
        let path = tree.path(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    tree
}

#[test]
fn the_notices_hold_each_program_row_and_each_distinct_licence_text_once() {
    let tree = tree("notices");
    assert_eq!(notices(&tree.path("")), Ok(NOTICES.into()));
}

#[test]
fn a_row_without_five_fields_is_refused() {
    let why = "TRUSTED line 2: not a name, version, licence, runs in and purpose two spaces apart";
    assert_eq!(
        rows("# name\ndemo 1.0.0  MIT  programs  drawing\n").err(),
        Some(why.into())
    );
}

#[test]
fn a_row_that_runs_elsewhere_is_refused() {
    let why = "TRUSTED line 1: runs in is programs or toolchain, not everywhere";
    assert_eq!(
        rows("demo  1.0.0  MIT  everywhere  drawing\n").err(),
        Some(why.into())
    );
}

#[test]
fn a_program_row_with_no_package_is_refused() {
    let tree = tree("notices_ghost");
    fs::write(
        tree.path("TRUSTED"),
        "ghost  1.0.0  MIT  programs  nothing\n",
    )
    .unwrap();
    let why = notices(&tree.path("")).unwrap_err();
    assert!(why.starts_with("vendor/ghost: "), "{why}");
}

#[test]
fn a_licence_with_no_text_is_refused() {
    let tree = tree("notices_no_text");
    let row = "odd  1.0.0  BSD-3-Clause  programs  nothing\n";
    fs::write(tree.path("TRUSTED"), row).unwrap();
    fs::create_dir_all(tree.path("vendor/odd")).unwrap();
    fs::write(tree.path("vendor/odd/Cargo.toml"), "[package]\n").unwrap();
    let why = "odd 1.0.0: its package has no licence text, and none is supplied for BSD-3-Clause";
    assert_eq!(notices(&tree.path("")), Err(why.into()));
}

#[test]
fn an_mit_text_without_authors_is_refused() {
    let tree = tree("notices_no_authors");
    let row = "odd  1.0.0  MIT  programs  nothing\n";
    fs::write(tree.path("TRUSTED"), row).unwrap();
    fs::create_dir_all(tree.path("vendor/odd")).unwrap();
    fs::write(tree.path("vendor/odd/Cargo.toml"), "[package]\n").unwrap();
    let why = "odd 1.0.0: no authors in its Cargo.toml for the MIT copyright line";
    assert_eq!(notices(&tree.path("")), Err(why.into()));
}
