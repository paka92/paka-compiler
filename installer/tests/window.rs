//! The counter in a window (M5-35): the dev runner opens it in the Wayland session and logs its
//! first presented frame; a second run closes it and opens another on the kept event loop, and
//! the runner exits cleanly when its input ends. Skipped with no Wayland display.

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

#[test]
fn the_counter_opens_a_window_and_presents_a_frame() {
    if env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipped: WAYLAND_DISPLAY is unset, so no window can open");
        return;
    }
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../");
    let text = fs::read_to_string(format!("{root}{COUNTER}")).unwrap();
    let mut runner = Runner(
        Command::new(env!("CARGO_BIN_EXE_paka-runner"))
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
    for _ in 0..2 {
        let run = ask(order.clone());
        assert_eq!(run.get("window"), Some(&Value::Bool(true)), "{run}");
        let tree = run.get("tree").and_then(Value::as_str).unwrap();
        assert!(tree.contains("Button #1 \"+\" on=Inc"), "{tree}");
        let line = log
            .recv_timeout(Duration::from_secs(10))
            .expect("a frame is logged");
        assert!(line.starts_with("first frame presented, "), "{line}");
    }
    drop(runner.0.stdin.take());
    assert!(runner.0.wait().unwrap().success());
}
