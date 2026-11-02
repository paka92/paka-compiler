//! The third-party notices file (M7-16): made from `TRUSTED` and the licence texts in `vendor/`,
//! the rows a program runs in, in the order of `TRUSTED`.

#[allow(dead_code)]
mod common;

use common::Scratch;
use paka::notices::{notices, rows};
use std::fs;

const TRUSTED: &str = "\
# name          version  licence            runs in    purpose
demo            1.0.0    MIT OR Apache-2.0  programs   drawing
tool            2.0.0    MIT                toolchain  dev-mode code
bare            0.1.0    MIT                programs   used by demo
libc            system   -                  programs   C runtime
fonts/Demo.ttf  1.0      OFL-1.1            programs   the font
";

const NOTICES: &str = "\
Third-party notices: each library and file in TRUSTED that a program contains or loads, in that
order, with the licence texts its package ships.

==== demo 1.0.0
licence: MIT OR Apache-2.0
purpose: drawing
---- LICENSE-APACHE
Apache text
---- LICENSE-MIT
MIT text

==== bare 0.1.0
licence: MIT
purpose: used by demo
(no licence text in its package)

==== libc system
licence: -
purpose: C runtime
(from the system, not shipped)

==== fonts/Demo.ttf 1.0
licence: OFL-1.1
purpose: the font
---- OFL.txt
OFL text
";

/// A tree with `TRUSTED` and the files its rows name.
fn tree(name: &str) -> Scratch {
    let tree = Scratch::new(name);
    let files = [
        ("TRUSTED", TRUSTED),
        ("vendor/demo/Cargo.toml", "[package]\n"),
        ("vendor/demo/LICENSE-MIT", "MIT text\n"),
        ("vendor/demo/LICENSE-APACHE", "Apache text"),
        ("vendor/demo/src/lib.rs", ""),
        ("vendor/tool/LICENSE", "tool text\n"),
        ("vendor/bare-0.1.0/Cargo.toml", "[package]\n"),
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
fn the_notices_hold_each_program_row_with_its_licence_texts() {
    let tree = tree("notices");
    assert_eq!(notices(&tree.path("")), Ok(NOTICES.into()));
}

#[test]
fn a_row_without_five_fields_is_refused() {
    let why = "TRUSTED line 2: not a name, version, licence, runs in and purpose two spaces apart";
    assert_eq!(rows("# name\ndemo 1.0.0  MIT  programs  drawing\n").err(), Some(why.into()));
}

#[test]
fn a_row_that_runs_elsewhere_is_refused() {
    let why = "TRUSTED line 1: runs in is programs or toolchain, not everywhere";
    assert_eq!(rows("demo  1.0.0  MIT  everywhere  drawing\n").err(), Some(why.into()));
}

#[test]
fn a_program_row_with_no_package_is_refused() {
    let tree = tree("notices_ghost");
    fs::write(tree.path("TRUSTED"), "ghost  1.0.0  MIT  programs  nothing\n").unwrap();
    let why = notices(&tree.path("")).unwrap_err();
    assert!(why.starts_with("vendor/ghost: "), "{why}");
}
