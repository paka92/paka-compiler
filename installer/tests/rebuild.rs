//! A rebuild compiles only the definitions whose code changes and installs them together (M4-37).

mod common;

use common::Dev;
use paka::hash::Hash;
use paka_rt::abi::dev::Context;
use std::collections::BTreeSet;

const BASE: &str = "\
fn half(x: Int) -> Int { x / 2 }
fn same(a: Int) -> Bool { half(a) == half(a) }
fn one() -> Int { 1 }
fn top() -> Int { if same(4) { one() } else { 0 } }
";

fn dev(source: &str) -> Dev {
    let mut dev = Dev::default();
    dev.edit(source).unwrap();
    dev
}

fn run(dev: &Dev, name: &str) -> u64 {
    let id = dev.program.fns.iter().position(|f| f.name == name);
    let (status, out) = common::run(&dev.slots, id.unwrap(), &mut Context::new(1 << 20));
    assert_eq!(status, 0, "{name}");
    out
}

/// The definitions whose key changed since `before`.
fn changed<'a>(dev: &'a Dev, before: &[Hash]) -> BTreeSet<&'a str> {
    let old: BTreeSet<_> = before.iter().collect();
    let new = dev.keys.iter().zip(&dev.program.fns);
    new.filter(|(k, _)| !old.contains(k))
        .map(|(_, f)| f.name.as_str())
        .collect()
}

#[test]
fn a_body_edit_compiles_once_and_its_revert_compiles_nothing() {
    let mut dev = dev(BASE);
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (4, 1));
    dev.edit(&BASE.replace("{ 1 }", "{ 2 }")).unwrap();
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (5, 2));
    dev.edit(BASE).unwrap();
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (5, 1));
    assert_eq!(
        dev.loader.installed(),
        5,
        "the revert reuses installed code"
    );
}

#[test]
fn a_failed_compile_installs_nothing() {
    let mut dev = dev(BASE);
    let broken = BASE.replace("{ 1 }", "{ 3 }")
        + "fn map() -> Int {\n  let m: Map<String, Int> = [\"a\": 1]\n  1\n}\n";
    assert!(dev.edit(&broken).unwrap_err().contains("not compiled yet"));
    assert_eq!((dev.cache.compiles, dev.loader.installed()), (4, 4));
    assert_eq!(run(&dev, "top"), 1);
}

#[test]
fn a_signature_edit_recompiles_the_definition_and_its_direct_users() {
    let mut dev = dev(BASE);
    let before = dev.keys.clone();
    let float = "fn half(x: Int) -> Float { to_float(x) / 2.0 }";
    dev.edit(&BASE.replace("fn half(x: Int) -> Int { x / 2 }", float))
        .unwrap();
    assert_eq!(changed(&dev, &before), BTreeSet::from(["half", "same"]));
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (6, 1));
}

#[test]
fn an_edit_elsewhere_leaves_trap_sites_and_code_alone() {
    let mut dev = dev(BASE);
    let before = dev.keys.clone();
    dev.edit(&BASE.replace("x / 2", "(x + 0) / 2")).unwrap();
    assert_eq!(changed(&dev, &before), BTreeSet::from(["half"]));
}

/// The List rows are runtime code told each item's record and size (M2-09 section 3), so editing
/// a comparator compiles that one definition and no collection code.
#[test]
fn a_comparator_edit_compiles_only_the_comparator() {
    const LISTS: &str = "\
type P { id: Int, score: Int }
fn before(a: P, b: P) -> Bool { a.score > b.score }
fn top() -> Int {
  var ps = [P { id: 1, score: 5 }, P { id: 2, score: 9 }]
  append(&ps, P { id: 3, score: 7 })
  sort_by(&ps, before)
  if contains(ps, P { id: 3, score: 7 }) { ps[0].id } else { 0 }
}
";
    let mut dev = dev(LISTS);
    let before = dev.keys.clone();
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (2, 2));
    dev.edit(&LISTS.replace("a.score > b.score", "a.score < b.score"))
        .unwrap();
    assert_eq!(changed(&dev, &before), BTreeSet::from(["before"]));
    assert_eq!((dev.cache.compiles, run(&dev, "top")), (3, 1));
}
