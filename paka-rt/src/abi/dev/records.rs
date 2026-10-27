//! Type records and the walkers that copy and drop through them (M4-33, docs/decisions/M4-05.md
//! sections 9 and 10). A record is words: size, equality, order, then the owned parts. An untagged
//! type has word 3 = n and n entries; a tagged one has word 3 = `TAGGED | variants`, then per
//! variant its n and n entries. An entry is four words: offset, kind, then by kind:
//!
//! - `STRING`: nothing.
//! - `LIST`: the item's record, 0 when items own nothing, and the item's size.
//! - `VALUE`: a tagged value held inline, and its record.
//! - `MAP`: the Map's word, its value's record (0 when values own nothing) and its shape
//!   (`map::STRING_KEY` and the value's size).
//! - `BOX`: a pointer to a block holding one value, the value's record (0 when it owns nothing)
//!   and size. A zero word is a box not yet built.
//!
//! Both walkers keep a worklist, so no value is deep enough to overflow the stack.

use super::map::Shape;
use super::words::{ListWords, TextWords};
use super::{Context, MEMORY, Trap};
use crate::abi::measure::Timed;
use std::ptr;

pub const STRING: u64 = 1;
pub const LIST: u64 = 2;
pub const MAP: u64 = 3;
pub const BOX: u64 = 4;
pub const VALUE: u64 = 5;
pub const TAGGED: u64 = 1 << 63;

/// An owned part of a value, the types it names by their program number.
pub enum Part {
    String,
    List { item: usize, size: u64 },
    Map { value: usize, shape: u64 },
    Value(usize),
    Box { item: usize, size: u64 },
}

/// What a type owns: one list of (offset, part), or one per variant when tagged.
pub struct Spec {
    pub size: u64,
    pub tagged: bool,
    pub alternatives: Vec<Vec<(u64, Part)>>,
}

/// Every record of a program in one block, which never moves.
pub struct Records {
    words: Box<[u64]>,
    /// Each type's first word, if it owns anything.
    starts: Vec<Option<usize>>,
}

impl Records {
    /// The records of `specs`, by program type number; `None` owns nothing.
    pub fn new(specs: &[Option<Spec>]) -> Records {
        let mut starts = vec![];
        let mut len = 0;
        for spec in specs {
            starts.push(spec.as_ref().map(|s| {
                let start = len;
                len += 3 + usize::from(s.tagged);
                len += s
                    .alternatives
                    .iter()
                    .map(|a| 1 + 4 * a.len())
                    .sum::<usize>();
                start
            }));
        }
        let mut records = Records {
            words: vec![0; len].into_boxed_slice(),
            starts,
        };
        let mut words = vec![];
        for spec in specs.iter().flatten() {
            words.extend([spec.size, 0, 0]);
            if spec.tagged {
                words.push(TAGGED | spec.alternatives.len() as u64);
            }
            for alt in &spec.alternatives {
                words.push(alt.len() as u64);
                for (offset, part) in alt {
                    words.extend(match *part {
                        Part::String => [*offset, STRING, 0, 0],
                        Part::List { item, size } => [*offset, LIST, records.address(item), size],
                        Part::Map { value, shape } => [*offset, MAP, records.address(value), shape],
                        Part::Value(t) => [*offset, VALUE, records.address(t), 0],
                        Part::Box { item, size } => [*offset, BOX, records.address(item), size],
                    });
                }
            }
        }
        records.words.copy_from_slice(&words);
        records
    }

    /// The address of type `t`'s record, 0 when it owns nothing.
    pub fn address(&self, t: usize) -> u64 {
        let base = self.words.as_ptr().expose_provenance();
        self.starts[t].map_or(0, |s| (base + s * 8) as u64)
    }
}

/// `rt.copy`: the value at `from` copied into the words at `to`, every block it owns new. Past
/// the cap a `memory` trap at `site`; what was copied by then is freed when the run ends.
///
/// # Safety
///
/// `record` is the record of the value at `from`, and `to` has room for it apart from `from`.
pub unsafe extern "sysv64" fn rt_copy(
    ctx: &mut Context,
    site: u64,
    record: u64,
    from: u64,
    to: u64,
) -> i32 {
    let _timed = Timed::copy();
    let (from, to) = (at(from), at(to));
    // SAFETY: the caller vouches for the record and both places; a record starts with the size.
    unsafe { ptr::copy_nonoverlapping(from, to, *at::<u64>(record) as usize) };
    let mut work = vec![(record, from, to)];
    while let Some((record, from, to)) = work.pop() {
        // SAFETY: the record is the type's, and `to` already holds a word copy of `from`.
        for &[offset, kind, a, b] in unsafe { entries(record, from) } {
            let (from, to) = (
                from.wrapping_add(offset as usize),
                to.wrapping_add(offset as usize),
            );
            // SAFETY: the entry's kind says what the words at `offset` hold.
            let copied = unsafe {
                match kind {
                    STRING => text(ctx, from.cast(), to.cast()),
                    LIST => list(ctx, (a, b), from.cast(), to.cast(), &mut work),
                    MAP => map(ctx, Shape::new(a, b), from.cast(), to.cast(), &mut work),
                    BOX => boxed(ctx, (a, b), from.cast(), to.cast(), &mut work),
                    _ => {
                        work.push((a, from, to));
                        Some(())
                    }
                }
            };
            if copied.is_none() {
                ctx.trap = Trap {
                    kind: MEMORY,
                    site,
                    ..Trap::default()
                };
                return 1;
            }
        }
    }
    0
}

/// `rt.drop`: frees every block the value at `at` owns, items before their List; never traps.
///
/// # Safety
///
/// `record` is the record of the value at `at`, which no one uses after.
pub unsafe extern "sysv64" fn rt_drop(ctx: &mut Context, record: u64, value: u64) {
    enum Task {
        Walk(u64, *mut u8),
        Free(*mut u8, u64),
    }
    let mut work = vec![Task::Walk(record, at(value))];
    while let Some(task) = work.pop() {
        let (record, value) = match task {
            Task::Walk(r, v) => (r, v),
            Task::Free(block, bytes) => {
                ctx.heap.release(block, bytes);
                continue;
            }
        };
        // SAFETY: the caller vouches for the record and the value.
        for &[offset, kind, a, b] in unsafe { entries(record, value) } {
            let part = value.wrapping_add(offset as usize);
            match kind {
                STRING => {
                    // SAFETY: a STRING entry holds a String's words.
                    let words = unsafe { &*part.cast::<TextWords>() };
                    if words.cap != 0 {
                        work.push(Task::Free(words.ptr, words.cap));
                    }
                }
                LIST => {
                    // SAFETY: a LIST entry holds a List's words.
                    let words = unsafe { &*part.cast::<ListWords>() };
                    if words.cap == 0 {
                        continue;
                    }
                    work.push(Task::Free(words.ptr, words.cap * b));
                    if a != 0 {
                        let item = |i| Task::Walk(a, words.ptr.wrapping_add((i * b) as usize));
                        work.extend((0..words.len).map(item));
                    }
                }
                MAP => {
                    // SAFETY: a MAP entry holds a Map's word.
                    let block = unsafe { *part.cast::<*mut u8>() };
                    // SAFETY: the word is null or a Map block of the entry's shape.
                    let (len, cap) = unsafe { Shape::head(block) };
                    if block.is_null() {
                        continue;
                    }
                    let shape = Shape::new(a, b);
                    work.push(Task::Free(block, shape.bytes(cap)));
                    for i in 0..len {
                        // SAFETY: entry `i` is there, its key first.
                        let key = unsafe { &*shape.key(block, i).cast::<TextWords>() };
                        if shape.string() && key.cap != 0 {
                            work.push(Task::Free(key.ptr, key.cap));
                        }
                        if a != 0 {
                            work.push(Task::Walk(a, shape.value(block, i)));
                        }
                    }
                }
                BOX => {
                    // SAFETY: a BOX entry holds a pointer to its block, or 0.
                    let block = unsafe { *part.cast::<*mut u8>() };
                    if !block.is_null() {
                        work.push(Task::Free(block, b));
                        if a != 0 {
                            work.push(Task::Walk(a, block));
                        }
                    }
                }
                _ => work.push(Task::Walk(a, part)),
            }
        }
    }
}

/// A new block for the String at `to`, a copy of the one at `from`.
pub(super) unsafe fn text(
    ctx: &mut Context,
    from: *const TextWords,
    to: *mut TextWords,
) -> Option<()> {
    // SAFETY: the caller vouches both are a String's words.
    let (from, to) = unsafe { (&*from, &mut *to) };
    to.cap = from.bytes;
    // SAFETY: a String's `ptr` holds `bytes` bytes.
    to.ptr = unsafe { block(ctx, from.ptr, from.bytes)? };
    Some(())
}

/// A new block for the List at `to`, a copy of the one at `from`; its items that own blocks go
/// on `work`.
unsafe fn list(
    ctx: &mut Context,
    (item, size): (u64, u64),
    from: *const ListWords,
    to: *mut ListWords,
    work: &mut Vec<(u64, *mut u8, *mut u8)>,
) -> Option<()> {
    // SAFETY: the caller vouches both are a List's words.
    let (from, to) = unsafe { (&*from, &mut *to) };
    to.cap = from.len;
    if from.len == 0 {
        to.ptr = ptr::null_mut();
        return Some(());
    }
    // A List of 0-byte items still takes a block, so `ptr` is null only when `cap` is 0.
    let bytes = from.len * size;
    to.ptr = ctx.heap.alloc(bytes)?;
    // SAFETY: both blocks hold `len` items of `size`, and `to.ptr` is new.
    unsafe { ptr::copy_nonoverlapping(from.ptr, to.ptr, bytes as usize) };
    if item != 0 {
        let at = |p: *mut u8, i: u64| p.wrapping_add((i * size) as usize);
        work.extend((0..from.len).map(|i| (item, at(from.ptr, i), at(to.ptr, i))));
    }
    Some(())
}

/// A new block for the Map at `to`, a copy of the one at `from`: its String keys copied, its
/// values that own blocks on `work`.
unsafe fn map(
    ctx: &mut Context,
    shape: Shape,
    from: *const *mut u8,
    to: *mut *mut u8,
    work: &mut Vec<(u64, *mut u8, *mut u8)>,
) -> Option<()> {
    // SAFETY: the caller vouches both hold a Map's word, `from`'s null or a block of `shape`.
    let (from, to, (len, _)) = unsafe { (*from, &mut *to, Shape::head(*from)) };
    if len == 0 {
        *to = ptr::null_mut();
        return Some(());
    }
    *to = ctx.heap.alloc(shape.bytes(len))?;
    // SAFETY: both blocks hold `len` entries, and `*to` is new; its capacity is its length.
    unsafe {
        ptr::copy_nonoverlapping(from, *to, shape.bytes(len) as usize);
        to.cast::<u64>().add(1).write(len);
    }
    for i in 0..len {
        if shape.string() {
            // SAFETY: entry `i` of each holds a String key, word-copied.
            unsafe { text(ctx, shape.key(from, i).cast(), shape.key(*to, i).cast())? };
        }
        if shape.record() != 0 {
            work.push((shape.record(), shape.value(from, i), shape.value(*to, i)));
        }
    }
    Some(())
}

/// A new block for the box at `to`, a copy of the one at `from`; its value goes on `work` when it
/// owns blocks.
unsafe fn boxed(
    ctx: &mut Context,
    (item, size): (u64, u64),
    from: *const *mut u8,
    to: *mut *mut u8,
    work: &mut Vec<(u64, *mut u8, *mut u8)>,
) -> Option<()> {
    // SAFETY: the caller vouches both hold a box: a pointer to a block of `size`.
    let (from, to) = unsafe { (*from, &mut *to) };
    *to = ctx.heap.alloc(size)?;
    // SAFETY: both blocks hold `size` bytes, and `*to` is new.
    unsafe { ptr::copy_nonoverlapping(from, *to, size as usize) };
    if item != 0 {
        work.push((item, from, *to));
    }
    Some(())
}

/// A new block holding `bytes` from `from`; null when there are none.
unsafe fn block(ctx: &mut Context, from: *const u8, bytes: u64) -> Option<*mut u8> {
    if bytes == 0 {
        return Some(ptr::null_mut());
    }
    let to = ctx.heap.alloc(bytes)?;
    // SAFETY: `to` is a new block of `bytes`, and the caller vouches for `from`.
    unsafe { ptr::copy_nonoverlapping(from, to, bytes as usize) };
    Some(to)
}

/// The owned entries of the value at `value` of the type whose record is at `record`.
///
/// # Safety
///
/// `record` is a record from `Records`, and `value` holds a value of its type.
unsafe fn entries<'a>(record: u64, value: *const u8) -> &'a [[u64; 4]] {
    let words = at::<u64>(record);
    // SAFETY: the caller vouches for the record, whose layout the module comment gives.
    unsafe {
        let head = *words.add(3);
        let mut i = 3;
        if head & TAGGED != 0 {
            i += 1;
            for _ in 0..*value.cast::<u64>() {
                i += 1 + 4 * *words.add(i) as usize;
            }
        }
        let n = *words.add(i) as usize;
        std::slice::from_raw_parts(words.add(i + 1).cast(), n)
    }
}

fn at<T>(address: u64) -> *mut T {
    ptr::with_exposed_provenance_mut(address as usize)
}
