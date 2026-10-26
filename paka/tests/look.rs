//! The Look tool on the windowless app (M5-34, docs/decisions/M5-09.md section 2): a run that
//! launches it, its tree, a screenshot, clicks and keys, and each refusal.

#[allow(dead_code)]
mod common;

#[path = "common/look.rs"]
mod look;

use common::Scratch;
use paka::session::Runner;

#[test]
fn look_drives_the_windowless_app() {
    const INNER: &str = "PAKA_LOOK_NO_DISPLAY";
    if std::env::var_os(INNER).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "look_drives_the_windowless_app"])
            .env(INNER, "1")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let scratch = Scratch::new("look");
    look::drive(
        &mut look::session(&scratch.path(""), Runner::new("".into())),
        false,
    );
}
