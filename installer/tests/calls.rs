//! Compiled calls go through slots: results and arguments cross them, a trap surfaces through
//! nested calls with its kind and position, unbounded recursion ends in a `stack` trap, and an
//! interrupt stops a run with status 2 (M4-27).

mod common;

use paka::core_form::Program;
use paka_rt::abi::dev::{self as rt, Context, on_runner};
use std::sync::atomic::{AtomicU64, Ordering};

const PROGRAM: &str = "\
fn add(a: Int, b: Int) -> Int { a + b }
fn divide(a: Int, b: Int) -> Int { a / b }
fn shift(a: Int, n: Int) -> Int { shift_left(a, n) }
fn convert(x: Float) -> Int { to_int(x) }
fn twice(n: Int) -> Int { add(n, n) }
fn overflows() -> Int { twice(9223372036854775807) }
fn divides() -> Int { 1 + divide(1, 0) }
fn shifts() -> Int { shift(1, 64) }
fn converts() -> Int { convert(1.0 / 0.0) }
fn half(x: Float) -> Float { x / 2.0 }
fn flip(on: Bool) -> Bool { !on }
fn crosses() -> Int { to_int(half(5.0)) + add(1, 2) + if flip(false) { 10 } else { 0 } }
fn down(n: Int) -> Int { down(n + 1) + 1 }
fn deep() -> Int { down(0) }
fn spin() -> Int {
  while true { }
  0
}
fn waits() -> Int { spin() + 1 }
";

fn id(p: &Program, name: &str) -> usize {
    p.fns.iter().position(|f| f.name == name).unwrap()
}

/// A trap record position as (definition, line, column).
fn place(p: &Program, at: u64) -> (String, usize, usize) {
    let (f, site) = p.place(at);
    (f.name.clone(), site.line, site.col)
}

/// The trap's kind, its position and then each frame's.
fn trace(p: &Program, ctx: &Context) -> (u64, Vec<(String, usize, usize)>) {
    let at = std::iter::once(ctx.trap.site).chain(ctx.trap.frames.iter().copied());
    (ctx.trap.kind, at.map(|a| place(p, a)).collect())
}

fn at(name: &str, line: usize, col: usize) -> (String, usize, usize) {
    (name.into(), line, col)
}

#[test]
fn a_trap_surfaces_through_nested_calls_with_its_kind_and_position() {
    let (p, slots) = common::load(PROGRAM);
    let cases = [
        (
            "overflows",
            rt::OVERFLOW,
            vec![at("add", 1, 35), at("twice", 5, 27), at("overflows", 6, 25)],
        ),
        (
            "divides",
            rt::DIVIDE_BY_ZERO,
            vec![at("divide", 2, 38), at("divides", 7, 27)],
        ),
        (
            "shifts",
            rt::OUT_OF_BOUNDS,
            vec![at("shift", 3, 35), at("shifts", 8, 22)],
        ),
        (
            "converts",
            rt::BAD_CONVERSION,
            vec![at("convert", 4, 31), at("converts", 9, 24)],
        ),
    ];
    for (name, kind, want) in cases {
        let mut ctx = Context::new(0);
        let (status, out) = common::run(&slots, id(&p, name), &mut ctx);
        assert_eq!(
            (status, out),
            (1, common::UNWRITTEN),
            "{name}: a trap leaves out unwritten"
        );
        assert_eq!(trace(&p, &ctx), (kind, want), "{name}");
    }
}

#[test]
fn results_and_arguments_cross_calls() {
    let (p, slots) = common::load(PROGRAM);
    let mut ctx = Context::new(0);
    assert_eq!(
        common::run(&slots, id(&p, "crosses"), &mut ctx),
        (0, 2 + 3 + 10)
    );
}

#[test]
fn unbounded_recursion_is_a_stack_trap() {
    let (p, slots) = common::load(PROGRAM);
    let (status, trap) = on_runner(0, |ctx| {
        let (status, _) = common::run(&slots, id(&p, "deep"), ctx);
        (status, std::mem::take(&mut ctx.trap))
    });
    assert_eq!(
        (status, trap.kind, trap.site, trap.more),
        (1, rt::STACK, 0, true)
    );
    let frames: Vec<_> = trap.frames.iter().map(|&a| place(&p, a)).collect();
    assert_eq!(
        frames,
        vec![at("down", 13, 26); 8],
        "the first eight frames, innermost first"
    );
}

#[test]
fn an_interrupt_stops_a_loop_and_its_callers() {
    let (p, slots) = common::load(PROGRAM);
    let mut ctx = Context::new(0);
    let flag = (&raw const ctx.interrupt).expose_provenance();
    let (status, out) = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let flag = std::ptr::with_exposed_provenance::<AtomicU64>(flag);
            // SAFETY: `ctx` outlives the scope, and the run only reads the flag atomically.
            unsafe { &*flag }.store(1, Ordering::Relaxed);
        });
        common::run(&slots, id(&p, "waits"), &mut ctx)
    });
    let stopped = (status, out, ctx.trap.kind, ctx.trap.frames.len());
    assert_eq!(
        stopped,
        (2, common::UNWRITTEN, 0, 0),
        "no trap and no frames"
    );
}
