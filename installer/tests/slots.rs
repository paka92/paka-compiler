//! A swap redirects the callers of a slot, a group swaps whole or not at all, and no swap happens
//! while installed code runs.

use paka_installer::{Code, Refused, Slots, install};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

/// `mov rax, [rdi]; jmp [rax + 8]`: calls slot 1 with the same arguments.
const CALL_SLOT_1: &[u8] = &[0x48, 0x8b, 0x07, 0xff, 0x60, 0x08];
/// `mov rax, [rdi]; jmp [rax + 16]`: calls slot 2 with the same arguments.
const CALL_SLOT_2: &[u8] = &[0x48, 0x8b, 0x07, 0xff, 0x60, 0x10];
/// `mov qword [rsi], 7; xor eax, eax; ret`
const SEVEN: &[u8] = &[0x48, 0xc7, 0x06, 0x07, 0, 0, 0, 0x31, 0xc0, 0xc3];
/// `mov qword [rsi], 9; xor eax, eax; ret`
const NINE: &[u8] = &[0x48, 0xc7, 0x06, 0x09, 0, 0, 0, 0x31, 0xc0, 0xc3];
/// `wait: mov rax, [rdi + 24]; test rax, rax; jz wait; mov eax, 2; ret`: spins until interrupted.
const SPIN: &[u8] = &[
    0x48, 0x8b, 0x47, 0x18, 0x48, 0x85, 0xc0, 0x74, 0xf7, 0xb8, 2, 0, 0, 0, 0xc3,
];

/// A context's four fixed words: slots, rt, stack_limit, interrupt.
type Context = [AtomicU64; 4];

fn context() -> Context {
    Default::default()
}

/// Runs slot 0; its status and the word it wrote.
fn run(slots: &Slots, ctx: &Context) -> Result<(i32, u64), Refused> {
    let mut out = 0u64;
    let ctx = ctx.as_ptr().cast_mut().cast::<c_void>();
    // SAFETY: every slot a test runs holds one of the functions above, all of type
    // `fn(ctx, out) -> i32`, which touch only the context's words and `out`.
    let status = unsafe { slots.run(0, ctx, (&raw mut out).cast())? };
    Ok((status, out))
}

fn installed(functions: &[&[u8]]) -> Vec<usize> {
    let code = install(functions).unwrap();
    code.iter().map(|c: &Code| c.address).collect()
}

#[test]
fn a_swapped_slot_changes_the_callee() {
    let [caller, seven, nine] = installed(&[CALL_SLOT_1, SEVEN, NINE])[..] else {
        unreachable!()
    };
    let slots = Slots::new(2);
    slots.swap(&[(0, caller), (1, seven)]).unwrap();
    let ctx = context();
    assert_eq!(run(&slots, &ctx), Ok((0, 7)));
    slots.swap(&[(1, nine)]).unwrap();
    assert_eq!(run(&slots, &ctx), Ok((0, 9)));
}

#[test]
fn a_group_swaps_together_or_not_at_all() {
    let [one, two, seven, nine] = installed(&[CALL_SLOT_1, CALL_SLOT_2, SEVEN, NINE])[..] else {
        unreachable!()
    };
    let slots = Slots::new(3);
    slots.swap(&[(0, one), (1, seven), (2, nine)]).unwrap();
    let ctx = context();
    assert_eq!(run(&slots, &ctx), Ok((0, 7)));
    assert_eq!(slots.swap(&[(0, two), (3, seven)]), Err(Refused::NoSlot(3)));
    assert_eq!(run(&slots, &ctx), Ok((0, 7)));
    slots.swap(&[(0, two), (2, seven), (1, nine)]).unwrap();
    assert_eq!(run(&slots, &ctx), Ok((0, 7)));
}

#[test]
fn a_swap_during_a_run_is_refused() {
    let [spin, seven] = installed(&[SPIN, SEVEN])[..] else {
        unreachable!()
    };
    let slots = Slots::new(1);
    slots.swap(&[(0, spin)]).unwrap();
    let ctx = context();
    thread::scope(|s| {
        // A run that starts while the loop below swaps is refused; it tries again.
        let running = s.spawn(|| {
            let mut ran = run(&slots, &ctx);
            while ran == Err(Refused::Busy) {
                ran = run(&slots, &ctx);
            }
            ran
        });
        while slots.swap(&[(0, spin)]).is_ok() {
            thread::yield_now();
        }
        assert_eq!(slots.swap(&[(0, seven)]), Err(Refused::Busy));
        assert_eq!(run(&slots, &ctx), Err(Refused::Busy));
        ctx[3].store(1, Ordering::Relaxed);
        assert_eq!(running.join().unwrap(), Ok((2, 0)));
    });
    slots.swap(&[(0, seven)]).unwrap();
    assert_eq!(run(&slots, &ctx), Ok((0, 7)));
}

#[test]
fn a_run_of_a_missing_slot_is_refused() {
    assert_eq!(run(&Slots::new(0), &context()), Err(Refused::NoSlot(0)));
}
