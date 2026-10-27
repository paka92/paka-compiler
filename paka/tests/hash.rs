//! Body and interface hashes (M4-11): each kind of edit changes exactly the expected hashes.

use paka::hash::Hash;
use paka::source::Source;
use paka::{DefCheck, check_each};
use std::collections::BTreeMap;

const DEFS: &[&str] = &[
    "type Point { x: Int = 0, y: Int = 0 }",
    "type Line { a: Point, b: Point }",
    "type Shapes { all: List<Line> }",
    "enum Tree { Leaf, Node(left: Tree, right: Tree) }",
    "type A { bs: List<B> }",
    "type B { a: Option<A> }",
    "type C { b: B }",
    "fn span(l: Line) -> Int {\n  l.b.x - l.a.x\n}",
    "fn scale(p: Point; by: Int) -> Point {\n  Point { x: p.x * by, y: p.y * by }\n}",
    "fn total(s: Shapes) -> Int {\n  0\n}",
    "fn size(t: Tree) -> Int {\n  match t {\n    Leaf => 1\n    Node(l, r) => size(l) + size(r)\n  }\n}",
    "fn count() -> Int {\n  3\n}",
    "fn waits() -> Int {\n  nope()\n}",
    "example three {\n  count() == 3\n}",
];

type Hashes = BTreeMap<String, (Hash, Hash)>;

fn hashes(defs: &[String]) -> Hashes {
    let text = defs.join("\n\n") + "\n";
    let checked = check_each("main.paka", &Source::file("main.paka", text.into_bytes())).unwrap();
    let pair = |d: &DefCheck| (d.name.clone(), (d.body, d.interface));
    checked.iter().map(pair).collect()
}

/// The names whose body and whose interface hash differ, a new definition counted as changed.
fn changed(before: &Hashes, after: &Hashes) -> (Vec<String>, Vec<String>) {
    let differ = |pick: fn(&(Hash, Hash)) -> Hash| {
        let old = |n: &String| before.get(n).map(pick);
        let names = after.iter().filter(|(n, h)| old(n) != Some(pick(h)));
        names.map(|(n, _)| n.clone()).collect()
    };
    (differ(|h| h.0), differ(|h| h.1))
}

fn first_name(def: &str) -> &str {
    let rest = def.split_once(' ').unwrap().1;
    rest.split(|c: char| !c.is_alphanumeric()).next().unwrap()
}

#[test]
fn each_edit_changes_exactly_the_expected_hashes() {
    // (what, the definition replaced or added, its new text, bodies changed, interfaces changed)
    let edits: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "a body edit",
            "fn count() -> Int {\n  4\n}",
            &["count"],
            &[],
        ),
        (
            "layout only",
            "fn span(l: Line)->Int{\n    l.b.x  -  l.a.x\n}",
            &[],
            &[],
        ),
        (
            "a comment",
            "fn total(s: Shapes) -> Int {\n  // none yet\n  0\n}",
            &[],
            &[],
        ),
        (
            "a default value, reaching past the type's own users through List",
            "type Point { x: Int = 1, y: Int = 0 }",
            &["Point"],
            &["Line", "Point", "Shapes", "scale", "span", "total"],
        ),
        (
            "a field added in a recursive group, and the type holding the group",
            "type B { a: Option<A>, n: Int }",
            &["B"],
            &["A", "B", "C"],
        ),
        (
            "a positional name",
            "fn span(line: Line) -> Int {\n  line.b.x - line.a.x\n}",
            &["span"],
            &[],
        ),
        (
            "a label",
            "fn scale(p: Point; times: Int) -> Point {\n  Point { x: p.x * times, y: p.y * times }\n}",
            &["scale"],
            &["scale"],
        ),
        (
            "a variant added to a recursive enum",
            "enum Tree { Leaf, Node(left: Tree, right: Tree), Pair(a: Int, b: Int) }",
            &["Tree"],
            &["Tree", "size"],
        ),
        (
            "a missing name defined",
            "fn nope() -> Int {\n  1\n}",
            &["nope", "waits"],
            &["nope"],
        ),
        (
            "a result type",
            "fn count() -> Float {\n  4.0\n}",
            &["count"],
            &["count"],
        ),
    ];
    let mut defs: Vec<String> = DEFS.iter().map(|d| d.to_string()).collect();
    for &(what, new, bodies, interfaces) in edits {
        let before = hashes(&defs);
        match defs.iter().position(|d| first_name(d) == first_name(new)) {
            Some(i) => defs[i] = new.into(),
            None => defs.push(new.into()),
        }
        let (b, i) = changed(&before, &hashes(&defs));
        assert_eq!((b, i), (names(bodies), names(interfaces)), "{what}");
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|n| n.to_string()).collect()
}

#[test]
fn order_changes_no_hash() {
    let defs: Vec<String> = DEFS.iter().map(|d| d.to_string()).collect();
    let reversed: Vec<String> = defs.iter().rev().cloned().collect();
    assert_eq!(hashes(&reversed), hashes(&defs));
}
