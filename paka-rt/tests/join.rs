//! `rt.text`'s JOIN makes its result in one block of the joined size, and checks the cap before it
//! makes anything (M2-06 section 1.4), so a String built by joins costs no more than in the
//! interpreter.

use paka_rt::abi::dev::words::{JOIN, TextWords, rt_text};
use paka_rt::abi::dev::{Context, MEMORY};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::{ptr, slice};

/// The system allocator, counting the bytes each thread asks of it.
struct Counted;

thread_local! {
    static ASKED: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: each call passes straight to `System`.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ASKED.with(|n| n.set(n.get() + layout.size()));
        // SAFETY: the caller's layout, as given.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTED: Counted = Counted;

/// Constant words for `s`, which owns no block.
fn constant(s: &str) -> TextWords {
    TextWords {
        ptr: s.as_ptr().cast_mut(),
        bytes: s.len() as u64,
        chars: s.chars().count() as u64,
        cap: 0,
    }
}

/// JOIN of `a` and `b`: its status, its words and the bytes asked of the allocator meanwhile.
fn join(ctx: &mut Context, a: &str, b: &str) -> (i32, TextWords, usize) {
    let (a, b) = (constant(a), constant(b));
    let mut out = [0u64; 4];
    let before = ASKED.with(Cell::get);
    let at = |t: &TextWords| ptr::from_ref(t).expose_provenance() as u64;
    // SAFETY: both operands are Strings' words, and `out` has room for a String's words.
    let status = unsafe { rt_text(ctx, JOIN, 0, at(&a), at(&b), 0, out.as_mut_ptr()) };
    let asked = ASKED.with(Cell::get) - before;
    // SAFETY: `out` holds a String's words when the status is 0, and is unread otherwise.
    let words = unsafe { out.as_ptr().cast::<TextWords>().read() };
    (status, words, asked)
}

#[test]
fn a_join_makes_one_block_of_the_joined_size() {
    let mut ctx = Context::new(1 << 30);
    let (a, b) = ("é".repeat(1 << 19), "z".repeat(1 << 20));
    let (status, words, asked) = join(&mut ctx, &a, &b);
    assert_eq!(status, 0);
    assert_eq!((words.bytes, words.chars), (2 << 20, 3 << 19));
    // SAFETY: the joined words point at `bytes` bytes.
    let made = unsafe { slice::from_raw_parts(words.ptr, words.bytes as usize) };
    assert!(made == [a.as_bytes(), b.as_bytes()].concat());
    // The block, and at most the heap's bookkeeping.
    assert!(asked < (2 << 20) + 4096, "asked {asked} bytes");
}

#[test]
fn a_join_past_the_cap_traps_before_it_makes_anything() {
    let mut ctx = Context::new(1 << 30);
    let half = "z".repeat(9 << 20);
    let (status, _, asked) = join(&mut ctx, &half, &half);
    assert_eq!((status, ctx.trap.kind), (1, MEMORY));
    assert_eq!(asked, 0);
}
