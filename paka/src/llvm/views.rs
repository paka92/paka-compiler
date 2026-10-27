//! Views built for release (M6-18a): their first tree as the interpreter draws it, with no display.
//! Each test links the IR with the runtime itself, so that a test can plant a fault in the IR.

use super::{Checks, app, lint};
use paka::interp::Windowless;
use paka::source::Source;
use paka_rt::effects::Host;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Once};
use std::{env, fs, io};

/// The paper's counter with `??reset` made the button M5-27's test uses.
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

fn counter(count: &str) -> String {
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
}

fn checked(text: &str) -> crate::core_form::Program {
    let source = Source::file("main.paka", text.into());
    crate::check("main.paka", &source)
        .unwrap_or_else(|d| panic!("{d}"))
        .0
}

/// The profile directory of this test binary, which sits in its `deps`.
fn profile() -> PathBuf {
    let exe = env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().into()
}

/// `libpaka_rt.a` for this profile, built once per test binary.
fn runtime() -> PathBuf {
    static BUILT: Once = Once::new();
    let profile = profile();
    BUILT.call_once(|| {
        let mut cargo = Command::new(env!("CARGO"));
        cargo.args([
            "build",
            "-p",
            "paka-rt",
            "--locked",
            "--offline",
            "--target-dir",
        ]);
        cargo.arg(profile.parent().unwrap());
        if profile.ends_with("release") {
            cargo.arg("--release");
        }
        assert!(cargo.status().unwrap().success(), "building paka-rt failed");
    });
    profile.join("libpaka_rt.a")
}

/// Builds `text` with its IR passed through `plant`, in a folder of its own named `name`, and
/// runs it: its status, standard output and standard error.
fn release(name: &str, text: &str, plant: fn(String) -> String) -> (Option<i32>, String, String) {
    let ir = plant(app(&mut checked(text), Checks::All).expect("the back end builds the app"));
    assert_eq!(lint::lint(&ir), Vec::<String>::new());
    let dir = env::temp_dir().join(format!("paka-view-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let (ll, exe) = (dir.join("main.ll"), dir.join("main"));
    fs::write(&ll, ir).unwrap();
    let libs = [
        "-lgcc_s",
        "-lutil",
        "-lrt",
        "-lpthread",
        "-lm",
        "-ldl",
        "-lc",
    ];
    let linked = Command::new("clang")
        .arg("-O3")
        .args([&ll, &runtime()])
        .args(libs)
        .arg("-o")
        .arg(&exe)
        .stdin(Stdio::null())
        .status()
        .unwrap();
    assert!(linked.success());
    let out = Command::new(&exe)
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    let text = |b| String::from_utf8(b).unwrap();
    (out.status.code(), text(out.stdout), text(out.stderr))
}

fn drawn(name: &str, text: &str) -> String {
    let (status, tree, err) = release(name, text, |ir| ir);
    assert_eq!((status, err.as_str()), (Some(0), ""));
    tree
}

/// The tree the interpreter draws first.
fn interpreted(text: &str) -> String {
    Windowless::start(Arc::new(checked(text)), &[], Host(io::sink()))
        .tree
        .to_string()
}

#[test]
fn the_counter_draws_its_tree_at_zero_and_three() {
    for count in ["0", "3"] {
        let text = COUNTER.replace("count: Int = 0", &format!("count: Int = {count}"));
        assert_eq!(interpreted(&text), counter(count));
        assert_eq!(drawn(count, &text), counter(count));
    }
}

#[test]
fn the_todo_list_draws_as_the_interpreter_does() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/examples/sample_todo");
    let todo = fs::read_to_string(path.join("main.paka")).unwrap();
    // A start message adds one done row.
    let row = todo
        .replace("draft: String = \"\"", "draft: String = \"milk\"")
        .replace("done: Bool = false", "done: Bool = true")
        .replace("  grants: []", "  start:  Add\n  grants: []");
    for (name, text) in [("todo", todo), ("row", row)] {
        assert_eq!(drawn(name, &text), interpreted(&text), "{name}");
    }
}

#[test]
fn a_trapping_view_stops_with_trap_status() {
    let text = COUNTER.replace("Row(gap: 8)", "Row(gap: s.count - 1)");
    let (status, tree, err) = release("trap", &text, |ir| ir);
    assert_eq!((status, tree.as_str()), (Some(3), ""));
    assert!(
        err.starts_with("counter: trap assert at main.paka:"),
        "{err}"
    );
}

/// Writing the Col's op as a Row's is caught: the compiler and the runtime share the tree layout.
#[test]
fn a_tree_layout_the_runtime_does_not_share_is_caught() {
    let col = |ir: String| ir.replacen("@paka_rt1_view(i64 3,", "@paka_rt1_view(i64 2,", 1);
    let (status, tree, _) = release("plant", COUNTER, col);
    assert_eq!(status, Some(0));
    assert_ne!(tree, counter("0"));
    assert!(tree.starts_with("Row gap=8\n"), "{tree}");
}
