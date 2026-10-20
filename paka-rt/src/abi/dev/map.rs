//! The Map rows (M4-38b, docs/decisions/M2-14.md sections 1.9 and 1.11). A Map is one word: null,
//! or a block holding its length and capacity, then its entries in rising key order. An entry is
//! the key, an Int or a String, then the value. A key is found by binary search.

use super::records::{rt_copy, rt_drop, text};
use super::words::{ListWords, TextWords, as_str};
use super::{Context, MEMORY, OUT_OF_BOUNDS, Trap};
use crate::ops::CAP;
use std::cmp::Ordering;
use std::ptr;

/// The rows of `rt.map`, by its `row` word.
/// The value's address, or an `out_of_bounds` trap: `m[key]` that needs the key.
pub const AT: u64 = 0;
/// The value's address, or 0.
pub const FIND: u64 = 1;
/// The value's address; a new entry holds a copy of the key and a zeroed value.
pub const PUT: u64 = 2;
pub const REMOVE: u64 = 3;
/// A new List of copies of the keys, or of the values, into `out`.
pub const KEYS: u64 = 4;
pub const VALUES: u64 = 5;

/// Set in the shape word when the key is a String; the low bits are the value's size.
pub const STRING_KEY: u64 = 1 << 63;

/// The words before the entries: length and capacity.
const HEAD: u64 = 16;

/// `rt.map`: Map row `row` on the Map whose word is at `map`, its value type's record `record` (0
/// when values own nothing) and `shape`. `key` is an Int key, or a String key's address; the key
/// is borrowed. A result goes to `out`. A trap is recorded at `site`.
///
/// # Safety
///
/// `map` holds a Map of that shape, `key` a key of its type, and `out` has room for the result.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "sysv64" fn rt_map(
    ctx: &mut Context,
    row: u64,
    site: u64,
    record: u64,
    shape: u64,
    map: *mut *mut u8,
    key: u64,
    out: *mut u8,
) -> i32 {
    let s = Shape::new(record, shape);
    // SAFETY: the caller vouches for every operand the row reads.
    let done = unsafe {
        let keyed = matches!(row, AT | FIND | PUT | REMOVE);
        let found = if keyed { s.search(*map, key) } else { Err(0) };
        let place = match (row, found) {
            (AT | FIND | PUT, Ok(i)) => Ok(Some(i)),
            (AT, Err(_)) => Err((OUT_OF_BOUNDS, [if s.string { 0 } else { key }, 0])),
            (FIND, Err(_)) => Ok(None),
            (PUT, Err(i)) => s.add(ctx, &mut *map, i, key).map(|()| Some(i)),
            (REMOVE, found) => {
                if let Ok(i) = found {
                    s.remove(ctx, *map, i);
                }
                Ok(None)
            }
            (KEYS | VALUES, _) => s.list(ctx, *map, row == KEYS, out.cast()).map(|()| None),
            _ => unreachable!("no Map row {row}"),
        };
        place.map(|place| {
            if matches!(row, AT | FIND | PUT) {
                out.cast::<u64>()
                    .write(place.map_or(0, |i| s.address(*map, i)));
            }
        })
    };
    match done {
        Ok(()) => 0,
        Err((kind, [x, y])) => {
            ctx.trap = Trap {
                kind,
                site,
                operands: [x, y, 0],
                ..Trap::default()
            };
            1
        }
    }
}

type Done = Result<(), (u64, [u64; 2])>;

const NO_ROOM: (u64, [u64; 2]) = (MEMORY, [0, 0]);

/// A Map type: whether its key is a String, its value's record and size.
pub(super) struct Shape {
    string: bool,
    record: u64,
    size: u64,
}

impl Shape {
    pub(super) fn new(record: u64, shape: u64) -> Shape {
        Shape {
            string: shape & STRING_KEY != 0,
            record,
            size: shape & !STRING_KEY,
        }
    }

    fn key_size(&self) -> u64 {
        if self.string { 32 } else { 8 }
    }

    fn entry(&self) -> u64 {
        self.key_size() + self.size
    }

    /// The bytes of a block with room for `cap` entries.
    pub(super) fn bytes(&self, cap: u64) -> u64 {
        HEAD + cap * self.entry()
    }

    /// The length and capacity of the Map whose block is `block`.
    ///
    /// # Safety
    ///
    /// `block` is null or a Map block.
    pub(super) unsafe fn head(block: *const u8) -> (u64, u64) {
        if block.is_null() {
            return (0, 0);
        }
        // SAFETY: as the caller vouches; the head is two words.
        unsafe { (*block.cast::<u64>(), *block.cast::<u64>().add(1)) }
    }

    pub(super) fn key(&self, block: *mut u8, i: u64) -> *mut u8 {
        block.wrapping_add((HEAD + i * self.entry()) as usize)
    }

    pub(super) fn value(&self, block: *mut u8, i: u64) -> *mut u8 {
        self.key(block, i).wrapping_add(self.key_size() as usize)
    }

    fn address(&self, block: *mut u8, i: u64) -> u64 {
        self.value(block, i).expose_provenance() as u64
    }

    pub(super) fn string(&self) -> bool {
        self.string
    }

    pub(super) fn record(&self) -> u64 {
        self.record
    }

    /// The position of `key`, or where it would go.
    ///
    /// # Safety
    ///
    /// `block` is null or a Map block of this shape, and `key` a key of its type.
    unsafe fn search(&self, block: *mut u8, key: u64) -> Result<u64, u64> {
        // SAFETY: as the caller vouches.
        let (len, _) = unsafe { Shape::head(block) };
        let at = |i: u64| self.key(block, i).expose_provenance() as u64;
        let order = |i: u64| -> Ordering {
            // SAFETY: `i` is below the length, so its key is there.
            unsafe {
                match self.string {
                    true => as_str(at(i)).cmp(as_str(key)),
                    false => (*self.key(block, i).cast::<i64>()).cmp(&(key as i64)),
                }
            }
        };
        let (mut low, mut high) = (0, len);
        while low < high {
            let mid = low + (high - low) / 2;
            match order(mid) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => return Ok(mid),
            }
        }
        Err(low)
    }

    /// A new entry at `i`: a copy of `key` and a zeroed value, the entries from `i` on one place
    /// up.
    ///
    /// # Safety
    ///
    /// `map` holds a Map block of this shape, or null; `i` is at most its length.
    unsafe fn add(&self, ctx: &mut Context, map: &mut *mut u8, i: u64, key: u64) -> Done {
        // SAFETY: as the caller vouches.
        let (len, cap) = unsafe { Shape::head(*map) };
        let mut words = [key, 0, 0, 0];
        if self.string {
            let from = ptr::with_exposed_provenance::<TextWords>(key as usize);
            // SAFETY: a String key is the address of a String's words, which `words` copies.
            unsafe {
                ptr::copy_nonoverlapping(from.cast(), words.as_mut_ptr(), 4);
                text(ctx, from, words.as_mut_ptr().cast()).ok_or(NO_ROOM)?;
            }
        }
        if len == cap {
            let grown = (len < CAP as u64).then_some(len + 1).ok_or(NO_ROOM);
            let more = grown.inspect_err(|_| self.free_key(ctx, &words))?;
            let cap = more.max(cap * 2).max(4).min(CAP as u64);
            let Some(block) = ctx.heap.alloc(self.bytes(cap)) else {
                self.free_key(ctx, &words);
                return Err(NO_ROOM);
            };
            if !map.is_null() {
                // SAFETY: the old block holds `len` entries; the new one is apart and larger.
                unsafe { ptr::copy_nonoverlapping(*map, block, self.bytes(len) as usize) };
                ctx.heap.release(*map, self.bytes(len));
            }
            // SAFETY: the new block has room for its head.
            unsafe { block.cast::<[u64; 2]>().write([len, cap]) };
            *map = block;
        }
        let (to, entry) = (self.key(*map, i), self.entry() as usize);
        // SAFETY: the block has room for one more entry; the entries from `i` move up by one.
        unsafe {
            ptr::copy(to, to.add(entry), ((len - i) * self.entry()) as usize);
            ptr::copy_nonoverlapping(words.as_ptr().cast(), to, self.key_size() as usize);
            ptr::write_bytes(self.value(*map, i), 0, self.size as usize);
            *map.cast::<u64>() = len + 1;
        }
        Ok(())
    }

    /// Frees the block of a String key not yet placed.
    fn free_key(&self, ctx: &mut Context, words: &[u64; 4]) {
        if self.string && words[3] != 0 {
            ctx.heap.release(
                ptr::with_exposed_provenance_mut(words[0] as usize),
                words[3],
            );
        }
    }

    /// Drops entry `i` and moves the entries after it one place down.
    ///
    /// # Safety
    ///
    /// `block` holds a Map block of this shape with entry `i`.
    unsafe fn remove(&self, ctx: &mut Context, block: *mut u8, i: u64) {
        // SAFETY: as the caller vouches.
        unsafe {
            let (len, _) = Shape::head(block);
            let key = self.key(block, i);
            if self.string {
                let words = &*key.cast::<TextWords>();
                if words.cap != 0 {
                    ctx.heap.release(words.ptr, words.cap);
                }
            }
            if self.record != 0 {
                let value = self.value(block, i).expose_provenance() as u64;
                rt_drop(ctx, self.record, value);
            }
            let entry = self.entry() as usize;
            ptr::copy(key.add(entry), key, ((len - i - 1) * self.entry()) as usize);
            *block.cast::<u64>() = len - 1;
        }
    }

    /// A new List of copies of the keys, or else of the values, into `out`.
    ///
    /// # Safety
    ///
    /// `block` is null or a Map block of this shape, and `out` has room for a List.
    unsafe fn list(
        &self,
        ctx: &mut Context,
        block: *mut u8,
        keys: bool,
        out: *mut ListWords,
    ) -> Done {
        // SAFETY: as the caller vouches.
        let (len, _) = unsafe { Shape::head(block) };
        let size = if keys { self.key_size() } else { self.size };
        let mut xs = ListWords {
            ptr: ptr::null_mut(),
            len,
            cap: len,
        };
        if len != 0 {
            xs.ptr = ctx.heap.alloc(len * size).ok_or(NO_ROOM)?;
        }
        for i in 0..len {
            let to = xs.ptr.wrapping_add((i * size) as usize);
            let (from, record) = match keys {
                true => (self.key(block, i), 0),
                false => (self.value(block, i), self.record),
            };
            // SAFETY: entry `i` is there, and `to` has room for a key or a value apart from it.
            unsafe {
                match (keys && self.string, record) {
                    (true, _) => {
                        ptr::copy_nonoverlapping(from, to, size as usize);
                        text(ctx, from.cast(), to.cast()).ok_or(NO_ROOM)?;
                    }
                    (false, 0) => ptr::copy_nonoverlapping(from, to, size as usize),
                    (false, record) => {
                        let (from, to) = (from.expose_provenance(), to.expose_provenance());
                        if rt_copy(ctx, 0, record, from as u64, to as u64) != 0 {
                            return Err(NO_ROOM);
                        }
                    }
                }
            }
        }
        // SAFETY: the caller vouches `out` has room for a List.
        unsafe { out.write(xs) };
        Ok(())
    }
}
