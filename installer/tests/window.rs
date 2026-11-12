//! The counter in a window (M5-35): the dev runner opens it in the Wayland session and logs its
//! first presented frame, and a frame after a Look click (M5-38); a second run closes it and opens
//! another on the kept event loop, and the runner exits cleanly when its input ends. The M5-34
//! Look script passes on an app in a window, and so do the M5-07 edits of paka/tests/look.rs
//! (M5-42). Skipped with no Wayland display.

#[path = "../../paka/tests/common/look.rs"]
mod look;

#[path = "../../paka/tests/common/edits.rs"]
mod edits;

use paka_json::{Value, parse};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const COUNTER: &str = "tests/cases/examples/sample_counter/main.paka";

/// The runner, killed if the test fails before it exits.
struct Runner(Child);

impl Drop for Runner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Whether a window can open; says so when not.
fn displayed() -> bool {
    let shown = env::var_os("WAYLAND_DISPLAY").is_some();
    if !shown {
        eprintln!("skipped: WAYLAND_DISPLAY is unset, so no window can open");
    }
    shown
}

#[test]
fn look_drives_the_app_in_a_window() {
    if !displayed() {
        return;
    }
    let dir = env::temp_dir().join(format!("paka-look-window-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let runner = paka::session::Runner::new(env!("CARGO_BIN_EXE_paka-runner").into());
    look::drive(&mut look::session(&dir, runner), true);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn edits_swap_reset_and_migrate_the_app_in_a_window() {
    if !displayed() {
        return;
    }
    let dir = env::temp_dir().join(format!("paka-edits-window-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let runner = paka::session::Runner::new(env!("CARGO_BIN_EXE_paka-runner").into());
    edits::drive(&mut edits::session(&dir, runner), true);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_counter_opens_a_window_and_presents_a_frame() {
    if !displayed() {
        return;
    }
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../");
    let text = fs::read_to_string(format!("{root}{COUNTER}")).unwrap();
    let mut runner = Runner(
        Command::new(env!("CARGO_BIN_EXE_paka-runner"))
            .env("PAKA_FRAME_LOG", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut replies = BufReader::new(runner.0.stdout.take().unwrap()).lines();
    let (logged, log) = mpsc::channel();
    let stderr = BufReader::new(runner.0.stderr.take().unwrap());
    thread::spawn(move || {
        stderr
            .lines()
            .map_while(Result::ok)
            .for_each(|l| drop(logged.send(l)))
    });
    let mut ask = |request: Value| {
        let stdin = runner.0.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        parse(&replies.next().unwrap().unwrap()).unwrap()
    };
    let file = Value::object([("path", "main.paka".into()), ("text", text.as_str().into())]);
    ask(Value::object([("files", Value::Array(vec![file]))]));
    let order = Value::object([
        ("app", "counter".into()),
        ("args", Value::Array(vec![])),
        ("limit_ms", 10_000usize.into()),
        ("max_bytes", 65_536usize.into()),
    ]);
    let click = Value::object([
        ("v", 1usize.into()),
        ("id", 1usize.into()),
        ("call", "look_click".into()),
        ("path", "#1/#1".into()),
    ]);
    let frame = || {
        let line = log.recv_timeout(Duration::from_secs(10));
        line.expect("a frame is logged")
    };
    for _ in 0..2 {
        let run = ask(order.clone());
        assert_eq!(run.get("window"), Some(&Value::Bool(true)), "{run}");
        let tree = run.get("tree").and_then(Value::as_str).unwrap();
        assert!(tree.contains("Button #1 \"+\" on=Inc"), "{tree}");
        let line = frame();
        assert!(line.starts_with("first frame presented, "), "{line}");
        let clicked = ask(click.clone());
        let tree = clicked.get("tree").and_then(Value::as_str).unwrap();
        assert!(tree.contains("Text #0 \"1\""), "{clicked}");
        let line = frame();
        assert!(line.starts_with("frame presented, "), "{line}");
    }
    drop(runner.0.stdin.take());
    assert!(runner.0.wait().unwrap().success());
}
