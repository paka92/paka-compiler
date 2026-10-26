//! Visual apps built by `paka build` (M6-18): the hole-free counter of paka/src/llvm/views.rs
//! takes clicks on standard input with no display (`--paka-replay`), keeps its old tree when a
//! handler traps, and opens a window. The window test is skipped with no Wayland display.

#[allow(dead_code)]
mod common;

use common::{Scratch, build_native};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const COUNTER: &str = "\
type Counter { count: Int = 0 }

enum Msg { Reset, Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Reset => s.count = 0
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text(\"{s.count}\")
    Row(gap: 8) {
      Button(\"-\", on: Dec)
      Button(\"+\", on: Inc)
    }
    Button(\"reset\", on: Reset)
  }
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}
";

const PLUS: &str = "click #1/#1\n";

/// The tree text after each action, each followed by an empty line, as `--paka-replay` writes it.
fn counters(counts: &[&str]) -> String {
    let tree = |count| {
        format!(
            "\
Col gap=8
  Text #0 \"{count}\"
  Row #1 gap=8
    Button #0 \"-\" on=Dec
    Button #1 \"+\" on=Inc
  Button #2 \"reset\" on=Reset

"
        )
    };
    counts.iter().map(tree).collect()
}

/// Builds `source` in `scratch` as `main`.
fn built(scratch: &Scratch, source: &str) -> PathBuf {
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    std::fs::write(&path, source).unwrap();
    let built = build_native(path.to_str().unwrap(), &exe);
    assert!(built.status.success(), "{built:?}");
    exe
}

/// Replays `actions` on `exe` with no display: its status, standard output and standard error.
fn replay(exe: &Path, actions: &str) -> (Option<i32>, String, String) {
    let mut child = Command::new(exe)
        .arg("--paka-replay")
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(actions.as_bytes()).unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let text = |b| String::from_utf8(b).unwrap();
    (out.status.code(), text(out.stdout), text(out.stderr))
}

#[test]
fn the_counter_takes_a_click_with_no_display() {
    let scratch = Scratch::new("release-counter");
    let exe = built(&scratch, COUNTER);
    let want = (Some(0), counters(&["0", "1"]), String::new());
    assert_eq!(replay(&exe, PLUS), want);
}

/// M5-25b in release: the trapping reset has set the count and made an `Exit`, and keeps neither.
#[test]
fn a_trapping_handler_keeps_the_old_tree_and_performs_none_of_its_effects() {
    let source = COUNTER
        .replace(
            "fn update(s: inout Counter, m: Msg) {\n  match m {\n    Reset => s.count = 0",
            "fn update(s: inout Counter, m: Msg) -> List<Effect<Msg>> {\n  match m {\n    \
             Reset => {\n      s.count = 0\n      return [Exit, Print(\"{1 / s.count}\")]\n    }",
        )
        .replace("s.count - 1)\n  }\n}", "s.count - 1)\n  }\n  []\n}")
        .replace("grants: []", "grants: [Console]");
    let scratch = Scratch::new("release-counter-trap");
    let exe = built(&scratch, &source);
    let (status, trees, err) = replay(&exe, &[PLUS, PLUS, "click #2\n", PLUS].concat());
    assert_eq!(
        (status, trees),
        (Some(0), counters(&["0", "1", "2", "2", "3"]))
    );
    assert_eq!(err, "counter: trap divide_by_zero at main.paka:9:31\n");
}

/// The app, killed when the test ends, which closes its window.
struct App(Child);

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn the_counter_opens_a_window() {
    if env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipped: WAYLAND_DISPLAY is unset, so no window can open");
        return;
    }
    let scratch = Scratch::new("release-counter-window");
    let exe = built(&scratch, COUNTER);
    let mut app = App(Command::new(exe)
        .env("PAKA_FRAME_LOG", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap());
    let (logged, log) = mpsc::channel();
    let stderr = BufReader::new(app.0.stderr.take().unwrap());
    thread::spawn(move || {
        stderr
            .lines()
            .map_while(Result::ok)
            .for_each(|l| drop(logged.send(l)))
    });
    let line = log.recv_timeout(Duration::from_secs(10));
    let line = line.expect("a frame is logged");
    assert!(line.starts_with("first frame presented, "), "{line}");
    assert_eq!(
        app.0.try_wait().unwrap(),
        None,
        "the app runs on in its window"
    );
}
