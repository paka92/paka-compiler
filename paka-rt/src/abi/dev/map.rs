//! The Map rows (M4-38b, docs/decisions/M2-14.md sections 1.9 and 1.11). A Map is one word: null,
//! or a block holding its length and capacity, then its entries in the order they came; `ORDER`
//! puts them in rising key order, and the rows that show order call it first. An entry is the key,
//! an Int or a String, then the value. After room for `cap` entries comes an index that finds a
//! key by its hash; nothing compiled code reads depends on it.

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
/// Puts the entries in rising key order, in place.
pub const ORDER: u64 = 6;

/// Set in the shape word when the key is a String; the low bits are the value's size.
pub const STRING_KEY: u64 = 1 << 63;

/// The words before the entries: length and capacity.
pub const HEAD: u64 = 16;

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
    unsafe {
        if matches!(row, AT | FIND | PUT)
            && let Some((at, i)) = s.find(*map, &s.key_of(key), ctx.probe)
        {
            ctx.probe = s.probe(*map, at);
            out.cast::<u64>().write(s.address(*map, i));
            return 0;
        }
        match s.rest(ctx, row, map, key, out) {
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
}

type Trapped = (u64, [u64; 2]);
type Done = Result<(), Trapped>;

const NO_ROOM: Trapped = (MEMORY, [0, 0]);

/// The index slot a keyed row last met: slot `at` of the block at `block`, of shape `shape`. Only
/// a hint: a row checks the slot finds its key before it trusts it.
#[derive(Clone, Copy, Default)]
pub struct Probe {
    block: usize,
    shape: u64,
    at: u64,
}

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

    /// Row `row` but for a key the Map holds under `AT`, `FIND` or `PUT`, which `rt_map` answers.
    ///
    /// # Safety
    ///
    /// As for `rt_map`; under those rows the Map lacks `key`.
    #[inline(never)]
    unsafe fn rest(
        &self,
        ctx: &mut Context,
        row: u64,
        map: *mut *mut u8,
        key: u64,
        out: *mut u8,
    ) -> Done {
        // SAFETY: as the caller vouches.
        unsafe {
            match row {
                AT => Err((OUT_OF_BOUNDS, [if self.string { 0 } else { key }, 0])),
                FIND => {
                    out.cast::<u64>().write(0);
                    Ok(())
                }
                PUT => {
                    let (at, i) = self.add(ctx, &mut *map, key)?;
                    ctx.probe = self.probe(*map, at);
                    out.cast::<u64>().write(self.address(*map, i));
                    Ok(())
                }
                REMOVE => {
                    if let Some((_, i)) = self.find(*map, &self.key_of(key), ctx.probe) {
                        self.remove(ctx, *map, i);
                    }
                    Ok(())
                }
                KEYS | VALUES => self.list(ctx, *map, row == KEYS, out.cast()),
                ORDER => {
                    self.order(*map);
                    Ok(())
                }
                _ => unreachable!("no Map row {row}"),
            }
        }
    }

    fn probe(&self, block: *mut u8, at: u64) -> Probe {
        Probe {
            block: block.addr(),
            shape: self.word(),
            at,
        }
    }

    /// The shape word this shape was made from.
    fn word(&self) -> u64 {
        self.size | if self.string { STRING_KEY } else { 0 }
    }

    fn key_size(&self) -> u64 {
        if self.string { 32 } else { 8 }
    }

    fn entry(&self) -> u64 {
        self.key_size() + self.size
    }

    /// The bytes of a block with room for `cap` entries.
    pub(super) fn bytes(&self, cap: u64) -> u64 {
        HEAD + cap * self.entry() + 8 * slots(cap)
    }

    /// The index of `block`, whose capacity is `cap`.
    fn index(&self, block: *mut u8, cap: u64) -> Index {
        Index {
            slots: self.key(block, cap).cast(),
            mask: slots(cap) - 1,
        }
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
        // Positions stay below `CAP`, so the product cannot wrap.
        block.wrapping_add(i.wrapping_mul(self.entry()).wrapping_add(HEAD) as usize)
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

    /// Entry `i`'s key as a key operand: an Int, or the address of a String's words.
    #[inline(always)]
    fn operand(&self, block: *mut u8, i: u64) -> u64 {
        let at = self.key(block, i);
        match self.string {
            true => at.expose_provenance() as u64,
            // SAFETY: entry `i` is there, its key an Int.
            false => unsafe { *at.cast::<u64>() },
        }
    }

    /// How entry `i`'s key orders against entry `j`'s.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape with entries `i` and `j`.
    unsafe fn compare(&self, block: *mut u8, i: u64, j: u64) -> Ordering {
        let [a, b] = [i, j].map(|i| self.operand(block, i));
        // SAFETY: as the caller vouches.
        unsafe {
            match self.string {
                true => as_str(a).cmp(as_str(b)),
                false => (a as i64).cmp(&(b as i64)),
            }
        }
    }

    /// Whether entry `i`'s key is `key`.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape with entry `i`, and `key` of its type.
    #[inline(always)]
    unsafe fn holds(&self, block: *mut u8, i: u64, key: &Key) -> bool {
        let own = self.operand(block, i);
        match self.string {
            // SAFETY: as the caller vouches.
            true => same(unsafe { as_str(own) }.as_bytes(), key.bytes),
            false => own == key.word,
        }
    }

    /// `key` as `find` seeks it.
    ///
    /// # Safety
    ///
    /// `key` is a key of this shape's type, live while the result is used.
    #[inline(always)]
    unsafe fn key_of<'a>(&self, key: u64) -> Key<'a> {
        if !self.string {
            let exact = (key.wrapping_add(EXACT / 2) < EXACT).then_some(key & (EXACT - 1));
            return Key::new(key, &[], mix(key >> 32, key), exact);
        }
        // SAFETY: as the caller vouches.
        let bytes = unsafe { as_str(key) }.as_bytes();
        let chunks = bytes.chunks_exact(8);
        let tail = small(chunks.remainder());
        let words = chunks.map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes")));
        let hash = mix(words.fold(bytes.len() as u64, mix), tail);
        let exact = (bytes.len() <= 4).then(|| {
            let packed = bytes.iter().rev().fold(0, |w, &b| w << 8 | u64::from(b));
            (bytes.len() as u64) << 32 | packed
        });
        Key::new(key, bytes, hash, exact)
    }

    /// Entry `i`'s key as `find` seeks it.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape with entry `i`.
    unsafe fn key_at<'a>(&self, block: *mut u8, i: u64) -> Key<'a> {
        // SAFETY: as the caller vouches.
        unsafe { self.key_of(self.operand(block, i)) }
    }

    /// The position in the slot `s`, which is not empty, if it finds `key`.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape whose index holds `s`, and `key` of its type.
    #[inline(always)]
    unsafe fn meets(&self, block: *mut u8, s: u64, key: &Key) -> Option<u64> {
        let i = (s & POS) - 1;
        let exact = key.check & EXACT != 0;
        // SAFETY: as the caller vouches, entry `i` is there.
        (s >> POS_BITS == key.check && (exact || unsafe { self.holds(block, i, key) })).then_some(i)
    }

    /// The slot that finds `key` and its entry's position, if the Map holds it; the slot of
    /// `probe` is tried first. Any Map block's index agrees with its entries, so a stale hint is
    /// only a miss.
    ///
    /// # Safety
    ///
    /// `block` is null or a Map block of this shape, and `key` of its type.
    #[inline(always)]
    unsafe fn find(&self, block: *mut u8, key: &Key, probe: Probe) -> Option<(u64, u64)> {
        // SAFETY: as the caller vouches.
        let (len, cap) = unsafe { Shape::head(block) };
        if len == 0 {
            return None;
        }
        let index = self.index(block, cap);
        let hinted = probe.block == block.addr() && probe.shape == self.word();
        // SAFETY: the mask keeps every slot read inside the index, which has an empty one.
        unsafe {
            if hinted {
                let s = index.get(probe.at);
                if s != 0
                    && let Some(i) = self.meets(block, s, key)
                {
                    return Some((probe.at, i));
                }
            }
            let mut at = home(key.hash);
            loop {
                let s = index.get(at);
                if s == 0 {
                    return None;
                }
                if let Some(i) = self.meets(block, s, key) {
                    return Some((at, i));
                }
                at += 1;
            }
        }
    }

    /// Puts the entries of `block` in rising key order, unless they are already.
    ///
    /// # Safety
    ///
    /// `block` is null or a Map block of this shape.
    unsafe fn order(&self, block: *mut u8) {
        // SAFETY: as the caller vouches; every position compared is below the length.
        unsafe {
            let len = Shape::head(block).0;
            if (1..len).all(|i| self.compare(block, i - 1, i).is_lt()) {
                return;
            }
            let mut order: Vec<u64> = (0..len).collect();
            order.sort_unstable_by(|&i, &j| self.compare(block, i, j));
            let entry = self.entry() as usize;
            let mut entries = Vec::with_capacity(len as usize * entry);
            for i in order {
                entries.extend_from_slice(std::slice::from_raw_parts(self.key(block, i), entry));
            }
            ptr::copy_nonoverlapping(entries.as_ptr(), self.key(block, 0), entries.len());
            self.reindex(block);
        }
    }

    /// Builds the index of `block`'s entries afresh.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape whose entries are set.
    pub(super) unsafe fn reindex(&self, block: *mut u8) {
        // SAFETY: as the caller vouches; the index lies inside the block.
        unsafe {
            let (len, cap) = Shape::head(block);
            let index = self.index(block, cap);
            ptr::write_bytes(index.slots, 0, slots(cap) as usize);
            for i in 0..len {
                index.enter(&self.key_at(block, i), i);
            }
        }
    }

    /// Takes entry `i` out of the index, moving later slots of its run back so no gap breaks it.
    ///
    /// # Safety
    ///
    /// `block` is a Map block of this shape with entry `i` in its index.
    unsafe fn unindex(&self, block: *mut u8, i: u64) {
        // SAFETY: as the caller vouches; every slot met holds an entry that is there.
        unsafe {
            let index = self.index(block, Shape::head(block).1);
            let start = |s: u64| home(self.key_at(block, (s & POS) - 1).hash);
            let mut hole = start(i + 1);
            while index.get(hole) & POS != i + 1 {
                hole += 1;
            }
            for next in hole + 1.. {
                let s = index.get(next);
                if s == 0 {
                    break;
                }
                let gap = next.wrapping_sub(hole) & index.mask;
                if next.wrapping_sub(start(s)) & index.mask >= gap {
                    index.set(hole, s);
                    hole = next;
                }
            }
            index.set(hole, 0);
        }
    }

    /// A new last entry, a copy of `key` and a zeroed value; its slot and position.
    ///
    /// # Safety
    ///
    /// `map` holds a Map block of this shape, or null, which lacks `key`.
    unsafe fn add(
        &self,
        ctx: &mut Context,
        map: &mut *mut u8,
        key: u64,
    ) -> Result<(u64, u64), Trapped> {
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
                unsafe {
                    ptr::copy_nonoverlapping(
                        *map,
                        block,
                        self.key(*map, len).offset_from_unsigned(*map),
                    )
                };
                ctx.heap.release(*map, self.bytes(len));
            }
            // SAFETY: the new block has room for its head, and its `len` entries are set.
            unsafe {
                block.cast::<[u64; 2]>().write([len, cap]);
                self.reindex(block);
            }
            *map = block;
        }
        // SAFETY: the block has room for one more entry, and its index for one more slot.
        unsafe {
            let to = self.key(*map, len);
            ptr::copy_nonoverlapping(words.as_ptr().cast(), to, self.key_size() as usize);
            ptr::write_bytes(self.value(*map, len), 0, self.size as usize);
            *map.cast::<u64>() = len + 1;
            let index = self.index(*map, Shape::head(*map).1);
            Ok((index.enter(&self.key_at(*map, len), len), len))
        }
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

    /// Drops entry `i` and moves the last entry into its place.
    ///
    /// # Safety
    ///
    /// `block` holds a Map block of this shape with entry `i`.
    unsafe fn remove(&self, ctx: &mut Context, block: *mut u8, i: u64) {
        // SAFETY: as the caller vouches.
        unsafe {
            let (len, cap) = Shape::head(block);
            self.unindex(block, i);
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
            let last = len - 1;
            *block.cast::<u64>() = last;
            if i == last {
                return;
            }
            ptr::copy_nonoverlapping(self.key(block, last), key, self.entry() as usize);
            let index = self.index(block, cap);
            let mut at = home(self.key_at(block, i).hash);
            while index.get(at) & POS != last + 1 {
                at += 1;
            }
            index.set(at, index.get(at) - (last - i));
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
        let (len, _) = unsafe {
            self.order(block);
            Shape::head(block)
        };
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

/// A key as `find` seeks it: its word, a String key's bytes, its hash, and the check a slot that
/// finds it holds.
struct Key<'a> {
    word: u64,
    bytes: &'a [u8],
    hash: u64,
    check: u64,
}

impl<'a> Key<'a> {
    /// `exact` is the key in 38 bits when it fits; the check is then the key itself.
    fn new(word: u64, bytes: &'a [u8], hash: u64, exact: Option<u64>) -> Key<'a> {
        let check = exact.map_or(hash >> 26 & (EXACT - 1), |e| EXACT | e);
        Key {
            word,
            bytes,
            hash,
            check,
        }
    }
}

fn mix(h: u64, w: u64) -> u64 {
    (h.rotate_left(5) ^ w).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Whether `a` and `b` are equal; a short key's compare stays inline.
#[inline(always)]
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && if a.len() <= 8 {
            small(a) == small(b)
        } else {
            a == b
        }
}

/// Up to 8 bytes in a word; two runs of the same length are equal just when their words are.
#[inline(always)]
fn small(b: &[u8]) -> u64 {
    let n = b.len();
    let four = |at: usize| u64::from(u32::from_le_bytes(b[at..at + 4].try_into().expect("4")));
    match n {
        0 => 0,
        1..=3 => u64::from(b[0]) | u64::from(b[n / 2]) << 8 | u64::from(b[n - 1]) << 16,
        _ => four(0) | four(n - 4) << 32,
    }
}

/// The bits of an index slot that hold its entry's position + 1; the bits above hold its key's
/// check. A slot of 0 is empty.
const POS_BITS: u32 = 25;
const POS: u64 = (1 << POS_BITS) - 1;
const _: () = assert!((CAP as u64) < POS);

/// Set in a check that is its key: an Int from -2^37 to 2^37 - 1, or a String of at most 4 bytes
/// and its length. Two keys with the same exact check are equal.
const EXACT: u64 = 1 << 38;

/// The slots of the index of a block with room for `cap` entries: a power of two, at least twice
/// `cap`, so a run of full slots stays short.
fn slots(cap: u64) -> u64 {
    (2 * cap).next_power_of_two()
}

/// The first slot a hash `h` tries, before the mask.
fn home(h: u64) -> u64 {
    h >> 32
}

/// A Map block's index: its slots, and their count less one.
struct Index {
    slots: *mut u64,
    mask: u64,
}

impl Index {
    /// # Safety
    ///
    /// The index is a Map block's.
    unsafe fn get(&self, at: u64) -> u64 {
        // SAFETY: the mask keeps `at` among the slots.
        unsafe { *self.slots.add((at & self.mask) as usize) }
    }

    /// # Safety
    ///
    /// The index is a Map block's.
    unsafe fn set(&self, at: u64, slot: u64) {
        // SAFETY: the mask keeps `at` among the slots.
        unsafe { *self.slots.add((at & self.mask) as usize) = slot }
    }

    /// Records entry `i`, whose key is `key`, in the first empty slot from its home; that slot.
    ///
    /// # Safety
    ///
    /// The index is a Map block's, with an empty slot.
    unsafe fn enter(&self, key: &Key, i: u64) -> u64 {
        let mut at = home(key.hash);
        // SAFETY: as the caller vouches.
        unsafe {
            while self.get(at) != 0 {
                at += 1;
            }
            self.set(at, key.check << POS_BITS | (i + 1));
        }
        at
    }
}
