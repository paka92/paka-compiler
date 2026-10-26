//! The Look tool on the windowless app (M5-34, docs/decisions/M5-09.md section 2): a run that
//! launches it, its tree, a screenshot, clicks and keys, and each refusal.

#[allow(dead_code)]
mod common;

#[path = "common/look.rs"]
mod look;

#[path = "common/edits.rs"]
mod edits;

use common::Scratch;
use paka::session::Runner;

/// Whether this is the copy of test `name` run with no display; runs that copy when not.
fn displayless(name: &str) -> bool {
    const INNER: &str = "PAKA_LOOK_NO_DISPLAY";
    if std::env::var_os(INNER).is_some() {
        return true;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name])
        .env(INNER, "1")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .status()
        .unwrap();
    assert!(status.success());
    false
}

#[test]
fn look_drives_the_windowless_app() {
    if !displayless("look_drives_the_windowless_app") {
        return;
    }
    let scratch = Scratch::new("look");
    look::drive(
        &mut look::session(&scratch.path(""), Runner::new("".into())),
        false,
    );
}

#[test]
fn edits_swap_reset_and_migrate_the_windowless_app() {
    if !displayless("edits_swap_reset_and_migrate_the_windowless_app") {
        return;
    }
    let scratch = Scratch::new("look-edits");
    edits::drive(
        &mut edits::session(&scratch.path(""), Runner::new("".into())),
        false,
    );
}
