//! A borrow lends the caller's value without copying it (M4-34): an argument, a List slice and an
//! `inout` place make no block, and a write through `inout` lands in the caller's place.

mod common;

use paka_rt::abi::dev::Context;

const BORROWS: &str = "\
fn total(xs: List<Int>) -> Int {
  var sum = 0
  for x in xs { sum += x }
  sum
}
fn bump(n: inout Int) { n += 10 }
fn whole() -> Int {
  let xs = [1, 2, 3, 4]
  total(xs)
}
fn sliced() -> Int {
  let xs = [1, 2, 3, 4]
  var n = total(xs[1..<3]) + len(xs[0..<2])
  for x in xs[2..<4] { n += x }
  n
}
fn bumped() -> Int {
  var xs = [1, 2, 3, 4]
  bump(&xs[1])
  xs[1]
}
fn copied() -> Int {
  let xs = [1, 2, 3, 4]
  let ys = xs[1..<3]
  total(ys)
}
";

#[test]
fn a_borrow_makes_no_block() {
    let (p, slots) = common::load(BORROWS);
    let run = |name: &str| {
        let id = p.fns.iter().position(|f| f.name == name).unwrap();
        let mut ctx = Context::new(1 << 20);
        let (status, out) = common::run(&slots, id, &mut ctx);
        assert_eq!((ctx.heap.leaks(), ctx.heap.faults()), (vec![], &[][..]));
        (status, out, ctx.heap.made())
    };
    // The List literal is the one block; a copied slice is a second.
    assert_eq!(run("whole"), (0, 10, 1));
    assert_eq!(run("sliced"), (0, 14, 1));
    assert_eq!(run("bumped"), (0, 12, 1));
    assert_eq!(run("copied"), (0, 5, 2));
}
