//! Recheck after an edit with early cutoff (M4-12): scripted edits recheck exactly the
//! hand-written sets, views included (M5-14), and the kept results equal a clean rebuild's.

use paka::DefCheck;
use paka::workspace::Workspace;

const PROGRAM: &str = "\
type Point { x: Int = 0, y: Int = 0 }

type Line { a: Point, b: Point }

type Path { lines: List<Line> }

fn span(l: Line) -> Int {
  l.b.x - l.a.x
}

fn first(p: Path) -> Line {
  p.lines[0]
}

fn total(p: Path) -> Int {
  span(first(p))
}

fn count() -> Int {
  3
}

fn twice() -> Int {
  count() * 2
}

fn waits() -> Int {
  nope()
}

example two {
  twice() == 6
}
";

fn show(checks: &[DefCheck]) -> Vec<String> {
    let one = |d: &DefCheck| {
        let error = d.error.as_ref().map(|e| (e.position, e.code, &e.message));
        format!(
            "{} {error:?} {:?} {:?} {:?}",
            d.name, d.reads, d.body, d.interface
        )
    };
    checks.iter().map(one).collect()
}

/// An edit: the definition named, its new text or none, then what it must recheck and the
/// messages of what it broke.
type Edit = (
    &'static str,
    Option<&'static str>,
    &'static [&'static str],
    &'static [&'static str],
);

const EDITS: &[Edit] = &[
    (
        "count",
        Some("fn count() -> Int {\n  4\n}"),
        &["count"],
        &[],
    ),
    (
        "twice",
        Some("fn twice() -> Int {\n    count()  *  2\n}"),
        &[],
        &[],
    ),
    (
        "count",
        Some("fn count(n: Int) -> Int {\n  n\n}"),
        &["count", "twice"],
        &["count takes 1 argument (n), got 0"],
    ),
    (
        "twice",
        Some("fn twice() -> Int {\n  count(3) * 2\n}"),
        &["twice"],
        &[],
    ),
    (
        "nope",
        Some("fn nope() -> Int {\n  1\n}"),
        &["nope", "waits"],
        &[],
    ),
    ("count", None, &["twice"], &["count is not defined here"]),
    // Past Point's own users: Path holds Line, and first and total reach Path.
    (
        "Point",
        Some("type Point { x: Int = 0, y: Int = 0, z: Int = 0 }"),
        &["Line", "Path", "Point", "first", "span", "total"],
        &[],
    ),
    // A longer body above the broken twice moves its kept error down.
    (
        "span",
        Some("fn span(l: Line) -> Int {\n  let d = l.b.x - l.a.x\n  d\n}"),
        &["span"],
        &[],
    ),
];

#[test]
fn each_edit_rechecks_exactly_the_expected_set() {
    edit_all(PROGRAM, EDITS);
}

const VIEWS: &str = "\
type Counter { count: Int = 0, items: List<Item> = [], last: Option<Tag> = None }

type Item { n: Int = 0 }

enum Tag { A, B }

type Unused { x: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count -= 1
  }
}

view line(n: Int) -> View<Msg> {
  Row(gap: 4) {
    Text(\"{n}\")
    Button(\"+\", on: Inc)
  }
}

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text(\"{s.count}\")
    line(s.count)
  }
}

view other(s: Counter) -> View<Msg> {
  Text(\"other\")
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}
";

const VIEW_EDITS: &[Edit] = &[
    (
        "counter_view",
        Some(
            "view counter_view(s: Counter) -> View<Msg> {\n  Col(gap: 4) {\n    Text(\"{s.count}\")\n    line(s.count)\n  }\n}",
        ),
        &["counter_view"],
        &[],
    ),
    (
        "Msg",
        Some("enum Msg { Inc, Dec, Reset }"),
        &["Msg", "counter", "counter_view", "line", "other", "update"],
        &["no arm for variant Reset; add Reset => ..."],
    ),
    (
        "Msg",
        Some("enum Msg { Inc, Dec }"),
        &["Msg", "counter", "counter_view", "line", "other", "update"],
        &[],
    ),
    (
        "line",
        Some("view line(n: Int, m: Int) -> View<Msg> {\n  Text(\"{n}\")\n}"),
        &["counter_view", "line"],
        &["line takes 2 arguments (n, m), got 1"],
    ),
    (
        "line",
        Some("view line(n: Int) -> View<Msg> {\n  Text(\"{n}\")\n}"),
        &["counter_view", "line"],
        &[],
    ),
    (
        "Counter",
        Some(
            "type Counter { count: Int = 0, items: List<Item> = [], last: Option<Tag> = None, step: Int = 1 }",
        ),
        &["Counter", "counter", "counter_view", "other", "update"],
        &[],
    ),
    // Through Counter's list: the app, update and both views that take a Counter.
    (
        "Item",
        Some("type Item { n: Int = 1 }"),
        &[
            "Counter",
            "Item",
            "counter",
            "counter_view",
            "other",
            "update",
        ],
        &[],
    ),
    (
        "counter",
        Some("app counter {\n  state:  Counter\n  update: update\n  view:   line\n  grants: []\n}"),
        &["counter"],
        &["view must be view(s: Counter) -> View<Msg>"],
    ),
    (
        "counter",
        Some(
            "app counter {\n  state:  Counter\n  update: update\n  view:   other\n  grants: []\n}",
        ),
        &["counter"],
        &[],
    ),
];

#[test]
fn view_edits_recheck_exactly_the_expected_set() {
    edit_all(VIEWS, VIEW_EDITS);
}

fn edit_all(program: &str, edits: &[Edit]) {
    let mut ws = Workspace::new(program.as_bytes()).unwrap();
    for &(name, new, rechecked, broken) in edits {
        let done = ws.edit(name, new).unwrap();
        let found: Vec<_> = done.rechecked.iter().map(String::as_str).collect();
        let broke: Vec<_> = done.broken.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(
            (found, broke),
            (rechecked.to_vec(), broken.to_vec()),
            "editing {name}"
        );
        let clean = Workspace::new(ws.text().as_bytes()).unwrap();
        assert_eq!(show(&ws.checks()), show(&clean.checks()), "editing {name}");
    }
}

#[test]
fn the_stored_types_are_those_the_state_reaches() {
    let ws = Workspace::new(VIEWS.as_bytes()).unwrap();
    let stored: Vec<_> = ws.stored("Counter").into_iter().collect();
    assert_eq!(stored, ["Counter", "Item", "Tag"]);
    assert!(ws.stored("update").is_empty() && ws.stored("Unused").len() == 1);
}
