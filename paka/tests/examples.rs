//! `paka examples`: the four statuses, written out by hand from M2-06 section 1.6, and those of
//! UI examples (M5-06), run with no display.

#[allow(dead_code)]
mod common;

use common::{EXAMPLE_CACHE, PAKA, Scratch};
use std::fs;
use std::process::Command;

/// Runs `paka examples` on `source`; stdout, stderr and the exit status.
fn examples(name: &str, source: &str) -> (String, String, Option<i32>) {
    let dir = Scratch::new(name);
    fs::write(dir.path("main.paka"), source).unwrap();
    let out = Command::new(PAKA)
        .args(["examples", "main.paka"])
        .env("XDG_CACHE_HOME", EXAMPLE_CACHE)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .current_dir(dir.path(""))
        .output()
        .unwrap();
    let text = |b| String::from_utf8(b).unwrap();
    (text(out.stdout), text(out.stderr), out.status.code())
}

const COUNTER: &str = "\
type Counter { count: Int = 0 }
enum Msg { Inc, Dec }

fn dec(n: Int) -> Int { if n > 0 { n - 1 } else { 0 } }

fn update(s: inout Counter, m: Msg) -> List<Effect<Msg>> {
  s.count = match m {
    Inc => s.count + ??step
    Dec => dec(s.count)
  }
  []
}

example dec_stops_at_zero {
  var s = Counter {}
  _ = update(&s, Dec)
  s.count == 0
}

example dec_from_two {
  var s = Counter { count: 2 }
  _ = update(&s, Dec)
  s.count == 2
}

example inc_once {
  var s = Counter {}
  _ = update(&s, Inc)
  s.count == 1
}

example first_of_none {
  let xs: List<Int> = []
  xs[0] == 0
}
";

#[test]
fn each_status_in_name_order() {
    let (out, err, status) = examples("counter", COUNTER);
    let expected = "\
dec_from_two fail at main.paka:23:11
  left: 1
  right: 2
dec_stops_at_zero pass
first_of_none trapped at main.paka:34:5
  trap out_of_bounds
  index 0, length 0
  in example first_of_none
inc_once unfinished at main.paka:8:22
  ??step
  in update, called from example inc_once
";
    assert_eq!(
        (out.as_str(), err.as_str(), status),
        (expected, "", Some(3))
    );
}

#[test]
fn all_passing_exits_0() {
    let fixed = COUNTER
        .replace("??step", "1")
        .replace("s.count == 2", "s.count == 1")
        .replace("xs[0] == 0", "xs == []");
    let (out, err, status) = examples("passing", &fixed);
    let expected = "\
dec_from_two pass
dec_stops_at_zero pass
first_of_none pass
inc_once pass
";
    assert_eq!(
        (out.as_str(), err.as_str(), status),
        (expected, "", Some(0))
    );
}

#[test]
fn no_examples_print_nothing_and_exit_0() {
    let (out, err, status) = examples("none", "fn one() -> Int { 1 }\n");
    assert_eq!((out.as_str(), err.as_str(), status), ("", "", Some(0)));
}

const FORMS: &str = r#"type P { x: Int, y: Int }
enum Shape { Dot, Circle(r: Float) }

example asserts {
  assert(1 > 2)
  true
}

example chain { 1 == 1 && 2 > 3 && true }

example plain { false }

example records { P { x: 3, y: 4 } == P { x: 3, y: 5 } }

example shapes { [Circle(1.0), Dot] == [Circle(2.5), Dot] }

example texts { "a\n" != "a\n" }
"#;

#[test]
fn failed_claims_and_asserts() {
    let (out, err, status) = examples("forms", FORMS);
    let expected = r#"asserts trapped at main.paka:5:3
  trap assert
chain fail at main.paka:9:29
  left: 2
  right: 3
plain fail at main.paka:11:17
records fail at main.paka:13:36
  left: P { x: 3, y: 4 }
  right: P { x: 3, y: 5 }
shapes fail at main.paka:15:37
  left: [Circle(1.0), Dot]
  right: [Circle(2.5), Dot]
texts fail at main.paka:17:23
  left: "a\n"
  right: "a\n"
"#;
    assert_eq!(
        (out.as_str(), err.as_str(), status),
        (expected, "", Some(3))
    );
}

#[test]
fn a_program_that_does_not_check_prints_one_line_and_exits_1() {
    let (out, err, status) = examples("bad", "example bad { 1 + 1 }\n");
    assert_eq!((out.as_str(), status), ("", Some(1)));
    assert!(
        err.starts_with("main.paka:1:15: error[type-mismatch]: "),
        "{err}"
    );
    assert_eq!(err.lines().count(), 1);
}

/// GENESIS.md's counter with a placeholder under its buttons.
const UI_COUNTER: &str = r##"type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text("{s.count}")
    Row(gap: 8) {
      Button("-", on: Dec)
      Button("+", on: Inc)
    }
    ??reset
  }
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}

example inc_once {
  var ui = ui_start(counter)
  ui_click(&ui, "#1/#1")
  ui_text(ui, "#0") == "1"
}

example dec_below_zero {
  var ui = ui_start(counter)
  ui_click(&ui, "#1/#0")
  ui_text(ui, "#0") == "-1"
}

example reset_shown {
  let ui = ui_start(counter)
  ui_has(ui, "#2")
}
"##;

#[test]
fn ui_counter_examples_pass_fail_and_touch_a_placeholder() {
    let (out, err, status) = examples("ui_counter", UI_COUNTER);
    let expected = r#"dec_below_zero fail at main.paka:39:21
  left: "0"
  right: "-1"
inc_once pass
reset_shown unfinished at main.paka:44:3
  ??reset
  in example reset_shown
"#;
    assert_eq!(
        (out.as_str(), err.as_str(), status),
        (expected, "", Some(3))
    );
}

const UI_ENDS: &str = r##"type S { text: String = "ab", n: Int = 0 }

enum Msg { Edit(text: String), Dup, Say, Quit, Hang }

fn update(s: inout S, m: Msg) -> List<Effect<Msg>> {
  match m {
    Edit(text) => {
      s.text = text
      []
    }
    Dup => {
      s.n = 1
      []
    }
    Say => [Print("hi")]
    Quit => [Exit]
    Hang => ??hang
  }
}

view v(s: S) -> View<Msg> {
  Col(gap: 0) {
    Input(s.text, on_change: Edit, key: "in")
    Button("dup", on: Dup)
    Button("say", on: Say)
    Button("quit", on: Quit)
    Button("hang", on: Hang)
    for i in 0..<s.n + 1 {
      Text("x", key: 7)
    }
  }
}

app a {
  state:  S
  update: update
  view:   v
  grants: []
}

example typed_at_end {
  var ui = ui_start(a)
  ui_click(&ui, "'in'")
  ui_type(&ui, "c\n")
  ui_text(ui, "'in'") == "abc"
}

example duplicate_key {
  var ui = ui_start(a)
  ui_click(&ui, "#1")
  true
}

example refused {
  var ui = ui_start(a)
  ui_click(&ui, "#2")
  true
}

example after_exit {
  var ui = ui_start(a)
  ui_click(&ui, "#3")
  let quit = ui_text(ui, "#3")
  ui_click(&ui, "#3")
  quit == "quit"
}

example hole_in_handler {
  var ui = ui_start(a)
  ui_click(&ui, "#4")
  true
}

example no_focus {
  var ui = ui_start(a)
  ui_type(&ui, "c")
  true
}

example click_on_text {
  var ui = ui_start(a)
  ui_click(&ui, "7")
  true
}
"##;

#[test]
fn ui_examples_end_in_error_nodes_refusals_and_traps() {
    let (out, err, status) = examples("ui_ends", UI_ENDS);
    let expected = r#"after_exit trapped at main.paka:64:3
  trap assert
click_on_text trapped at main.paka:82:3
  trap assert
duplicate_key fail at main.paka:50:3
  Error 7 "duplicate key 7: 2 children of Col"
hole_in_handler unfinished at main.paka:17:13
  ??hang
  in update, called from example hole_in_handler
no_focus trapped at main.paka:76:3
  trap assert
refused trapped
  a: effect Print refused: grant Console is missing
typed_at_end pass
"#;
    assert_eq!(
        (out.as_str(), err.as_str(), status),
        (expected, "", Some(3))
    );
}

#[test]
fn both_samples_check_clean() {
    for sample in ["sample_counter", "sample_todo"] {
        let file = format!(
            "{}/../tests/cases/examples/{sample}/main.paka",
            env!("CARGO_MANIFEST_DIR")
        );
        let out = Command::new(PAKA).args(["check", &file]).output().unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(0), "{sample}: {err}");
        assert!(!err.contains("error["), "{sample}: {err}");
    }
}
