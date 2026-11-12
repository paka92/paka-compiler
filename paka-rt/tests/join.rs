//! `rt.text`'s JOIN and JOIN_ALL make their result in one block of the joined size, and check the
//! cap before they make anything (M2-06 section 1.4), so a String built by joins costs no more than
//! in the interpreter.

use paka_rt::abi::dev::words::{JOIN, JOIN_ALL, TextWords, rt_text};
use paka_rt::abi::dev::{Context, MEMORY};
use paka_rt::ops::CAP;
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

/// The address of a String's words, as compiled code passes it.
fn at(t: &TextWords) -> u64 {
    ptr::from_ref(t).expose_provenance() as u64
}

/// String row `row` on `a` and `b`: its status, its words and the bytes asked of the allocator
/// meanwhile.
fn run(ctx: &mut Context, row: u64, a: u64, b: u64) -> (i32, TextWords, usize) {
    let mut out = [0u64; 4];
    let before = ASKED.with(Cell::get);
    // SAFETY: the callers pass the operands `row` takes, and `out` has room for a String's words.
    let status = unsafe { rt_text(ctx, row, 0, a, b, 0, out.as_mut_ptr()) };
    let asked = ASKED.with(Cell::get) - before;
    // SAFETY: `out` is four set words, valid as a String's words whatever they hold; they are a
    // String only when the status is 0.
    let words = unsafe { out.as_ptr().cast::<TextWords>().read() };
    (status, words, asked)
}

/// JOIN of `a` and `b`.
fn join(ctx: &mut Context, a: &str, b: &str) -> (i32, TextWords, usize) {
    let (a, b) = (constant(a), constant(b));
    run(ctx, JOIN, at(&a), at(&b))
}

/// JOIN_ALL of `parts`, passed as the addresses of their words.
fn join_all(ctx: &mut Context, parts: &[&str]) -> (i32, TextWords, usize) {
    let words: Vec<TextWords> = parts.iter().map(|p| constant(p)).collect();
    let list: Vec<u64> = words.iter().map(at).collect();
    let list_at = list.as_ptr().expose_provenance() as u64;
    run(ctx, JOIN_ALL, list_at, list.len() as u64)
}

/// The bytes of a made String.
fn bytes(words: &TextWords) -> &[u8] {
    match words.bytes {
        0 => &[],
        // SAFETY: a made String's words point at `bytes` bytes.
        n => unsafe { slice::from_raw_parts(words.ptr, n as usize) },
    }
}

#[test]
fn a_join_makes_one_block_of_the_joined_size() {
    let mut ctx = Context::new(1 << 30);
    let (a, b) = ("é".repeat(1 << 19), "z".repeat(1 << 20));
    let (status, words, asked) = join(&mut ctx, &a, &b);
    assert_eq!(status, 0);
    assert_eq!((words.bytes, words.chars), (2 << 20, 3 << 19));
    assert!(bytes(&words) == [a.as_bytes(), b.as_bytes()].concat());
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

#[test]
fn a_join_to_the_cap_is_made_and_one_byte_more_traps() {
    let mut ctx = Context::new(1 << 30);
    let half = "z".repeat(CAP / 2);
    assert_eq!(join(&mut ctx, &half, &half).0, 0);
    let (status, ..) = join(&mut ctx, &half, &format!("{half}z"));
    assert_eq!((status, ctx.trap.kind), (1, MEMORY));
}

#[test]
fn a_join_of_all_parts_makes_one_block_of_their_size() {
    let mut ctx = Context::new(1 << 30);
    let (a, b) = ("é".repeat(1 << 19), "z".repeat(1 << 20));
    let parts: [&str; 7] = ["", &a, "", "ü-42", &b, "true", ""];
    let (status, words, asked) = join_all(&mut ctx, &parts);
    assert_eq!(status, 0);
    assert_eq!((words.bytes, words.chars), ((2 << 20) + 9, (3 << 19) + 8));
    assert!(bytes(&words) == parts.concat().as_bytes());
    // The block, and at most the heap's bookkeeping.
    assert!(asked < (2 << 20) + 4096, "asked {asked} bytes");
}

#[test]
fn a_join_of_empty_parts_is_the_empty_string() {
    let mut ctx = Context::new(1 << 30);
    let (status, words, _) = join_all(&mut ctx, &["", "", ""]);
    assert_eq!(status, 0);
    assert_eq!((words.bytes, words.chars, words.cap), (0, 0, 0));
}

#[test]
fn a_join_of_many_parts_keeps_their_order() {
    let mut ctx = Context::new(1 << 30);
    let numbers: Vec<String> = (0..1000).map(|n| n.to_string()).collect();
    let parts: Vec<&str> = numbers.iter().map(String::as_str).collect();
    let (status, words, _) = join_all(&mut ctx, &parts);
    assert_eq!(status, 0);
    assert!(bytes(&words) == parts.concat().as_bytes());
    assert_eq!(words.chars, words.bytes);
}

#[test]
fn a_join_of_all_parts_to_the_cap_is_made_and_one_byte_more_traps_first() {
    let mut ctx = Context::new(1 << 30);
    let third = "é".repeat(CAP / 6);
    let rest = "z".repeat(CAP - 3 * third.len() - 1);
    let (t, r) = (third.as_str(), rest.as_str());
    assert_eq!(join_all(&mut ctx, &[t, t, "", t, r, "z"]).0, 0);
    // The last part, two bytes, crosses the cap by one.
    let (status, _, asked) = join_all(&mut ctx, &[t, t, t, r, "é"]);
    assert_eq!((status, ctx.trap.kind), (1, MEMORY));
    assert_eq!(asked, 0);
}
