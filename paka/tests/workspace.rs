//! Recheck after an edit with early cutoff (M4-12): scripted edits recheck exactly the
//! hand-written sets, and the kept results equal a clean rebuild's.

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
    let mut ws = Workspace::new(PROGRAM.as_bytes()).unwrap();
    for &(name, new, rechecked, broken) in EDITS {
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
