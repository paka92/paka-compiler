//! The example cache (M7-09): a rerun with no edit runs no example, an edit reruns exactly the
//! examples that reach the edited definition, and every output equals an uncached run's.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use paka::results::Cache;
use paka::source::Source;
use paka::{check_defs, interp};
use std::fs;
use std::path::Path;
use std::process::Command;

const PROGRAM: &str = r##"type Point { x: Int = 0, y: Int = 0 }

fn base() -> Int { 3 }

fn twice() -> Int { base() * 2 }

fn other() -> Int { 1 }

fn origin_x(p: Point) -> Int { p.x }

enum Msg { Add }

fn update(s: inout Point, m: Msg) {
  match m {
    Add => s.x += 1
  }
}

view show_x(s: Point) -> View<Msg> {
  Col(gap: 0) {
    Text("{s.x}")
    Button("+", on: Add)
  }
}

app clicks {
  state:  Point
  update: update
  view:   show_x
  grants: []
}

example two { twice() == 6 }

example one { other() == 1 }

example origin { origin_x(Point {}) == 0 }

example clicked {
  var ui = ui_start(clicks)
  ui_click(&ui, "#1")
  ui_text(ui, "#0") == "1"
}

example wrong { base() == 4 }
"##;

const STAMP: &str = "paka test 1";

/// Runs the examples of `text` against the cache in `dir`: the output and how many ran. The
/// output must equal an uncached run's.
fn run(dir: &Path, stamp: &str, text: &str) -> (String, usize) {
    let source = Source::file("main.paka", text.into());
    let ((core, ..), defs) = check_defs("main.paka", &source)
        .map_err(|d| d.to_string())
        .unwrap();
    let examples = |mut cache: Cache| {
        let mut out = vec![];
        interp::examples(&core, &mut cache, &mut out, &mut vec![]);
        (String::from_utf8(out).unwrap(), cache.runs)
    };
    let (cached, runs) = examples(Cache::open(dir.into(), stamp, &defs, &source));
    assert_eq!(cached, examples(Cache::default()).0, "a stale result");
    (cached, runs)
}

/// `PROGRAM` with `from` replaced by `to`, which must be in it.
fn edit(from: &str, to: &str) -> String {
    assert!(PROGRAM.contains(from));
    PROGRAM.replace(from, to)
}

#[test]
fn a_rerun_with_no_edit_runs_no_example() {
    let dir = Scratch::new("cache_rerun");
    let dir = dir.path("");
    let (first, runs) = run(&dir, STAMP, PROGRAM);
    assert_eq!(runs, 5);
    assert!(first.starts_with("clicked pass\none pass\norigin pass\ntwo pass\nwrong fail at"));
    assert_eq!(run(&dir, STAMP, PROGRAM), (first, 0));
}

#[test]
fn an_edit_reruns_exactly_the_examples_that_reach_it() {
    let dir = Scratch::new("cache_edits");
    let dir = dir.path("");
    run(&dir, STAMP, PROGRAM);
    let edits = [
        // A transitive dependency: wrong reads base, two reads it through twice.
        (edit("{ 3 }", "{ 2 + 1 }"), 2),
        (edit("{ 1 }", "{ 0 + 1 }"), 1),
        // A field default, reached by origin through a literal and by clicked through the app.
        (edit("x: Int = 0", "x: Int = 5"), 2),
        (edit("Text(\"{s.x}\")", "Text(\"{s.x + 1}\")"), 1),
        (edit("s.x += 1", "s.x += 2"), 1),
        // Unrelated: a new definition at the end, and a layout change, which no hash sees.
        (format!("{PROGRAM}\nfn unused() -> Int {{ 9 }}\n"), 0),
        (edit("{ base() * 2 }", "{  base()  *  2  }"), 0),
        // Moving a failing example's lines, or a definition it reaches, reruns it alone, as its
        // entry names positions.
        (edit("fn other", "fn unused() -> Int { 9 }\n\nfn other"), 1),
        (edit("fn base", "\n\nfn base"), 1),
    ];
    for (text, reruns) in edits {
        assert_eq!(
            run(&dir, STAMP, &text).1,
            reruns,
            "after editing to\n{text}"
        );
        assert_eq!(run(&dir, STAMP, PROGRAM).1, 0, "the first results are kept");
    }
}

#[test]
fn another_compiler_reruns_every_example_and_drops_the_old_results() {
    let dir = Scratch::new("cache_stamp");
    let dir = dir.path("");
    run(&dir, STAMP, PROGRAM);
    assert_eq!(run(&dir, "paka test 2", PROGRAM).1, 5);
    assert_eq!(
        fs::read_dir(&dir).unwrap().count(),
        1,
        "the first version is removed"
    );
    assert_eq!(run(&dir, STAMP, PROGRAM).1, 5);
}

/// `paka examples` keeps results between runs: an entry planted in its cache is what the next
/// run prints.
#[test]
fn paka_examples_reads_the_user_cache() {
    let dir = Scratch::new("cache_cli");
    fs::write(dir.path("main.paka"), PROGRAM).unwrap();
    let examples = || {
        let out = Command::new(PAKA)
            .args(["examples", "main.paka"])
            .env("XDG_CACHE_HOME", dir.path("cache"))
            .current_dir(dir.path(""))
            .output()
            .unwrap();
        (String::from_utf8(out.stdout).unwrap(), out.status.code())
    };
    let (first, status) = examples();
    assert_eq!(status, Some(3));
    assert!(first.starts_with("clicked pass\n"));
    let mut versions = fs::read_dir(dir.path("cache/paka/examples")).unwrap();
    let held = versions.next().unwrap().unwrap().path();
    assert!(versions.next().is_none(), "one compiler version");
    let mut planted = 0;
    for file in fs::read_dir(&held).unwrap() {
        let file = file.unwrap().path();
        if !file.file_name().unwrap().to_string_lossy().contains('-') {
            fs::write(file, "held\n").unwrap();
            planted += 1;
        }
    }
    assert_eq!(planted, 4);
    let (second, status) = examples();
    assert_eq!(status, Some(3));
    assert!(
        second.starts_with("held\nheld\nheld\nheld\nwrong fail at"),
        "{second}"
    );
    assert!(first.ends_with(second.trim_start_matches("held\n")));
}
