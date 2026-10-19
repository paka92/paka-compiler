//! The last checked version stays runnable (M4-13): after a breaking signature edit the
//! interpreter runs the old version; fixing the users releases both edits together.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use paka::workspace::Workspace;
use std::process::Command;

const PROGRAM: &str = "\
enum Msg { Start }

fn count() -> Int {
  7
}

fn update(m: Msg) -> List<Effect<Msg>> {
  [ExitWith(count())]
}

app counter {
  start:  Start
  update: update
  grants: []
}
";

#[test]
fn a_broken_edit_runs_the_old_version_until_its_users_are_fixed() {
    let mut ws = Workspace::new(PROGRAM.as_bytes()).unwrap();
    let scratch = Scratch::new("release");

    let broke = ws.edit("count", Some("fn count(n: Int) -> Int {\n  n + 3\n}"));
    assert_eq!(broke.unwrap().broken.len(), 1);
    assert_eq!(ws.released(), Some(PROGRAM));
    assert_eq!(run(&scratch, ws.released().unwrap()), 7);

    let fix = "fn update(m: Msg) -> List<Effect<Msg>> {\n  [ExitWith(count(6))]\n}";
    assert!(ws.edit("update", Some(fix)).unwrap().broken.is_empty());
    assert_eq!(ws.released(), Some(ws.text().as_str()));
    assert_eq!(run(&scratch, ws.released().unwrap()), 9);
}

fn run(scratch: &Scratch, text: &str) -> i32 {
    let path = scratch.path("main.paka");
    std::fs::write(&path, text).unwrap();
    let out = Command::new(PAKA).arg("run").arg(&path).output().unwrap();
    out.status.code().unwrap()
}
