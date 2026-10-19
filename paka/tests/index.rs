//! The who-uses-what index (M4-10): exact user lists, and an edit wakes exactly the users
//! whose check can change.

use paka::index::Index;
use paka::parser::parse_def;
use paka::source::Source;
use paka::{DefCheck, check_each};
use std::collections::BTreeSet;

const DEFS: &[&str] = &[
    "type Point { x: Int = 0, y: Int = 0 }",
    "type Box { at: Point, label: String = \"box\" }",
    "enum Shape { Dot, Circle(r: Int) }",
    "fn origin() -> Box {\n  Box { at: Point {} }\n}",
    "fn depth() -> Int {\n  origin().at.x\n}",
    "fn area(s: Shape) -> Int {\n  match s {\n    Dot => 0\n    Circle(r) => r * r\n  }\n}",
    "fn typo() -> Int {\n  areaa(Dot)\n}",
    "fn dangling() -> Int {\n  missing + 1\n}",
    "fn square() -> Shape {\n  Square(2)\n}",
    "example dot {\n  area(Dot) == 0\n}",
];

fn check(defs: &[String]) -> Vec<DefCheck> {
    let text = defs.join("\n\n") + "\n";
    check_each("main.paka", &Source::file("main.paka", text.into_bytes())).unwrap()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn each_name_lists_its_users() {
    let defs: Vec<_> = DEFS.iter().map(|d| d.to_string()).collect();
    let index = Index::build(&check(&defs));
    let expected: &[(&str, &[&str])] = &[
        ("Point", &["Box", "depth", "origin"]),
        ("Box", &["depth", "origin"]),
        ("Shape", &["area", "dot", "square"]),
        ("origin", &["depth"]),
        ("area", &["dot", "typo"]),
        ("areaa", &["typo"]),
        ("missing", &["dangling"]),
        ("Square", &["square"]),
        ("depth", &[]),
        ("dot", &[]),
    ];
    for (name, users) in expected {
        let found: BTreeSet<_> = index.users(name).map(String::from).collect();
        assert_eq!(found, set(users), "{name}");
    }
}

/// Applies `edit` (the definition named `name` becomes `new`, or goes), checks that it wakes
/// `woken`, and that rechecking only those and the edited one gives the clean index.
fn edit(defs: &mut Vec<String>, name: &str, new: Option<&str>, woken: &[&str]) {
    let mut index = Index::build(&check(defs));
    let at = defs.iter().position(|d| first_name(d) == name);
    let mut waiting = BTreeSet::new();
    if let Some(i) = at {
        waiting.extend(index.waiting(&parse_def("main.paka", defs[i].as_bytes()).unwrap()));
        index.remove(name);
        defs.remove(i);
    }
    if let Some(new) = new {
        waiting.extend(index.waiting(&parse_def("main.paka", new.as_bytes()).unwrap()));
        defs.push(new.into());
    }
    waiting.remove(name);
    assert_eq!(waiting, set(woken), "editing {name}");
    let rebuilt = check(defs);
    let edited = new.map(first_name);
    for def in rebuilt
        .iter()
        .filter(|d| waiting.contains(&d.name) || Some(&*d.name) == edited)
    {
        index.set(&def.name, def.reads.clone());
    }
    assert_eq!(index, Index::build(&rebuilt), "editing {name}");
}

fn first_name(def: &str) -> &str {
    let rest = def.split_once(' ').unwrap().1;
    rest.split(|c: char| !c.is_alphanumeric()).next().unwrap()
}

#[test]
fn an_edit_wakes_exactly_the_users_it_can_change() {
    let mut defs: Vec<_> = DEFS.iter().map(|d| d.to_string()).collect();
    // A name near a missing one becomes its suggestion.
    edit(
        &mut defs,
        "missinh",
        Some("fn missinh() -> Int {\n  1\n}"),
        &["dangling"],
    );
    // Adding the missing name wakes its waiting user.
    edit(
        &mut defs,
        "missing",
        Some("fn missing() -> Int {\n  1\n}"),
        &["dangling"],
    );
    // A variant's name was waited on by name, not by its enum.
    let shape = "enum Shape { Dot, Circle(r: Int), Square(side: Int) }";
    edit(&mut defs, "Shape", Some(shape), &["area", "dot", "square"]);
    edit(&mut defs, "origin", None, &["depth"]);
    // A rename is a delete and an add; `depth` stopped at the deleted `origin`.
    let spot = "type Spot { x: Int = 0, y: Int = 0 }";
    edit(&mut defs, "Point", Some(spot), &["Box"]);
}
