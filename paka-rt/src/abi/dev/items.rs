//! The generic List rows (M4-38, docs/decisions/M2-09.md section 3): one body for every item
//! type, told the item's record and size. An item handed in is moved: the caller gave up its
//! blocks. An item handed out is moved too, and its blocks are the caller's.

use super::super::table::CONSTANT;
use super::records::{rt_copy, rt_drop};
use super::words::{ListWords, as_str};
use super::{Context, MEMORY, OUT_OF_BOUNDS, Trap};
use crate::ops::{CAP, Float, sorted};
use std::cmp::Ordering;
use std::convert::Infallible;
use std::mem::transmute;
use std::ptr;

/// The rows of `rt.items`, by its `row` word.
pub const APPEND: u64 = 0;
pub const EXTEND: u64 = 1;
pub const POP: u64 = 2;
pub const INSERT: u64 = 3;
pub const REMOVE_AT: u64 = 4;
pub const SWAP: u64 = 5;
pub const REVERSE: u64 = 6;
/// `a` is the order: `INT`, `FLOAT` or `HELD` for Strings.
pub const SORT: u64 = 7;
pub const LIST_OF: u64 = 8;

/// How an item is read: as an Int word, a Float, or by address, the way `sort` reads a String and
/// `sort_by` hands over any held item.
pub const INT: u64 = 0;
pub const FLOAT: u64 = 1;
pub const HELD: u64 = 2;

/// `rt.items`: List row `row` on the List at `xs`, whose items have record `record` (0 when they
/// own nothing) and `size` bytes. `a` and `b` are the row's other operands: an item's address, a
/// List's address or an Int. A result goes to `out`. A trap is recorded at `site`.
///
/// # Safety
///
/// `xs` holds a List of such items (`LIST_OF` ignores it), each address operand holds the value the
/// row takes, and `out` has room for the result.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "sysv64" fn rt_items(
    ctx: &mut Context,
    row: u64,
    site: u64,
    record: u64,
    size: u64,
    xs: *mut ListWords,
    a: u64,
    b: u64,
    out: *mut u8,
) -> i32 {
    let item = Item { record, size };
    let at = |i: u64| ptr::with_exposed_provenance_mut::<u8>(i as usize);
    // SAFETY: the caller vouches for every operand the row reads.
    let done = unsafe {
        match row {
            LIST_OF => item.list_of(ctx, at(a), b as i64, out.cast()),
            APPEND => item.insert(ctx, &mut *xs, at(a), (*xs).len),
            INSERT => match b <= (*xs).len {
                true => item.insert(ctx, &mut *xs, at(a), b),
                false => Err((OUT_OF_BOUNDS, [b, (*xs).len])),
            },
            EXTEND => item.extend(ctx, &mut *xs, &*at(a).cast()),
            POP => match (*xs).len {
                0 => Err((OUT_OF_BOUNDS, [0, 0])),
                len => item.take(&mut *xs, len - 1, out),
            },
            REMOVE_AT => bound(a, &*xs).and_then(|i| item.take(&mut *xs, i, out)),
            SWAP => bound(a, &*xs)
                .and_then(|i| Ok((i, bound(b, &*xs)?)))
                .map(|(i, j)| {
                    ptr::swap_nonoverlapping(
                        item.at(&*xs, i),
                        item.at(&*xs, j),
                        (i != j) as usize * size as usize,
                    )
                }),
            REVERSE => {
                item.permute(&*xs, (0..(*xs).len as usize).rev().collect());
                Ok(())
            }
            SORT => item.sort(&*xs, a),
            _ => unreachable!("no List row {row}"),
        }
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

/// `before` as `sort_by` calls it, by how it takes an item (M4-05 section 5).
type Ask = unsafe extern "sysv64" fn(*mut Context, u64, u64, *mut u64) -> i32;
type AskFloat = unsafe extern "sysv64" fn(*mut Context, Float, Float, *mut u64) -> i32;

/// `rt.sort_by`: sorts the List at `xs` of `size`-byte items by `before`, the slot id of a
/// `fn(T, T) -> Bool` asked with the right item first (M2-14 section 1.13). `pass` is how it takes
/// an item: `INT`, `FLOAT` or `HELD`. A status other than 0 from `before` is returned as it is,
/// the List unchanged.
///
/// # Safety
///
/// `xs` holds a List of such items, and slot `before` the code of such a function.
pub unsafe extern "sysv64" fn rt_sort_by(
    ctx: &mut Context,
    size: u64,
    pass: u64,
    xs: *const ListWords,
    before: u64,
) -> i32 {
    let item = Item { record: 0, size };
    // SAFETY: as the caller vouches.
    let (xs, code) = unsafe { (&*xs, *ctx.slots.add(before as usize)) };
    let ctx: *mut Context = ctx;
    let ask = |r: usize, l: usize| {
        let (r, l) = (item.at(xs, r as u64), item.at(xs, l as u64));
        let mut out = 0u64;
        // SAFETY: the code takes items as `pass` says, and an Int or Float item is one word.
        let status = unsafe {
            match pass {
                INT => transmute::<usize, Ask>(code)(ctx, *r.cast(), *l.cast(), &mut out),
                FLOAT => transmute::<usize, AskFloat>(code)(ctx, *r.cast(), *l.cast(), &mut out),
                _ => {
                    let (r, l) = (r.expose_provenance() as u64, l.expose_provenance() as u64);
                    transmute::<usize, Ask>(code)(ctx, r, l, &mut out)
                }
            }
        };
        match status {
            0 => Ok(out as u8 != 0),
            status => Err(status),
        }
    };
    match sorted(xs.len as usize, ask) {
        Ok(order) => {
            item.permute(xs, order);
            0
        }
        Err(status) => status,
    }
}

/// A trap's kind and operands.
type Done = Result<(), (u64, [u64; 2])>;

const NO_ROOM: (u64, [u64; 2]) = (MEMORY, [0, 0]);

/// Index `i` of `xs`, or an `out_of_bounds` trap naming it and the length.
fn bound(i: u64, xs: &ListWords) -> Result<u64, (u64, [u64; 2])> {
    match i < xs.len {
        true => Ok(i),
        false => Err((OUT_OF_BOUNDS, [i, xs.len])),
    }
}

/// An item type: its record, 0 when it owns nothing, and its size in bytes.
struct Item {
    record: u64,
    size: u64,
}

impl Item {
    fn at(&self, xs: &ListWords, i: u64) -> *mut u8 {
        xs.ptr.wrapping_add((i * self.size) as usize)
    }

    /// Room for `more` items past `xs.len`, growing the block to twice its size at least.
    fn room(&self, ctx: &mut Context, xs: &mut ListWords, more: u64) -> Done {
        let len = xs.len.checked_add(more).filter(|&n| n <= CAP as u64);
        let len = len.ok_or(NO_ROOM)?;
        if len <= xs.cap {
            return Ok(());
        }
        let cap = len.max(xs.cap * 2).max(4).min(CAP as u64);
        let block = ctx.heap.alloc(cap * self.size).ok_or(NO_ROOM)?;
        if xs.len != 0 {
            // SAFETY: the old items, in a block or a constant, are apart from the new block,
            // which is larger.
            unsafe { ptr::copy_nonoverlapping(xs.ptr, block, (xs.len * self.size) as usize) };
        }
        if xs.cap != CONSTANT {
            ctx.heap.release(xs.ptr, xs.cap * self.size);
        }
        (xs.ptr, xs.cap) = (block, cap);
        Ok(())
    }

    /// Moves the item at `from` into `xs` at `i`, the items from `i` on one place up.
    ///
    /// # Safety
    ///
    /// `from` holds an item apart from `xs`, and `i` is at most its length.
    unsafe fn insert(&self, ctx: &mut Context, xs: &mut ListWords, from: *mut u8, i: u64) -> Done {
        self.room(ctx, xs, 1)?;
        let (to, size) = (self.at(xs, i), self.size as usize);
        // SAFETY: the block has room for one more item; the items from `i` move up by one.
        unsafe {
            ptr::copy(to, to.add(size), ((xs.len - i) * self.size) as usize);
            ptr::copy_nonoverlapping(from, to, size);
        }
        xs.len += 1;
        Ok(())
    }

    /// Moves the items of `more` to the end of `xs` and frees its block.
    ///
    /// # Safety
    ///
    /// `more` holds a List apart from `xs` whose blocks the callee may take.
    unsafe fn extend(&self, ctx: &mut Context, xs: &mut ListWords, more: &ListWords) -> Done {
        self.room(ctx, xs, more.len)?;
        // SAFETY: `xs` has room for `more.len` items past its length, apart from `more`'s.
        unsafe {
            ptr::copy_nonoverlapping(
                more.ptr,
                self.at(xs, xs.len),
                (more.len * self.size) as usize,
            )
        };
        xs.len += more.len;
        if more.cap != 0 {
            ctx.heap.release(more.ptr, more.cap * self.size);
        }
        Ok(())
    }

    /// Moves item `i` of `xs` to `out`, the items after it one place down.
    ///
    /// # Safety
    ///
    /// `i` is below the length, and `out` has room for an item.
    unsafe fn take(&self, xs: &mut ListWords, i: u64, out: *mut u8) -> Done {
        let (from, size) = (self.at(xs, i), self.size as usize);
        // SAFETY: as the caller vouches; the items after `i` move down by one.
        unsafe {
            ptr::copy_nonoverlapping(from, out, size);
            ptr::copy(
                from.add(size),
                from,
                ((xs.len - i - 1) * self.size) as usize,
            );
        }
        xs.len -= 1;
        Ok(())
    }

    /// A new List of `count` items into `out`: copies of the item at `from`, which the last takes.
    ///
    /// # Safety
    ///
    /// `from` holds an item, and `out` has room for a List.
    unsafe fn list_of(
        &self,
        ctx: &mut Context,
        from: *mut u8,
        count: i64,
        out: *mut ListWords,
    ) -> Done {
        let n = u64::try_from(count).map_err(|_| (OUT_OF_BOUNDS, [count as u64, 0]))?;
        let mut xs = ListWords {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
        };
        if n == 0 {
            // SAFETY: the item is the callee's, and nothing uses it after.
            unsafe { self.drop(ctx, from) };
        } else {
            self.room(ctx, &mut xs, n)?;
            for i in 0..n - 1 {
                // SAFETY: the block has room for `n` items, apart from `from`.
                unsafe { self.copy(ctx, from, self.at(&xs, i)) }?;
                xs.len += 1;
            }
            // SAFETY: as above; the last place takes the item itself.
            unsafe { ptr::copy_nonoverlapping(from, self.at(&xs, n - 1), self.size as usize) };
            xs.len = n;
        }
        // SAFETY: the caller vouches `out` has room for a List.
        unsafe { out.write(xs) };
        Ok(())
    }

    /// # Safety
    ///
    /// `from` holds an item, and `to` has room for one apart from it.
    unsafe fn copy(&self, ctx: &mut Context, from: *mut u8, to: *mut u8) -> Done {
        if self.record == 0 {
            // SAFETY: as the caller vouches.
            unsafe { ptr::copy_nonoverlapping(from, to, self.size as usize) };
            return Ok(());
        }
        let (from, to) = (
            from.expose_provenance() as u64,
            to.expose_provenance() as u64,
        );
        // SAFETY: the record is the item's, and the caller vouches for both places.
        match unsafe { rt_copy(ctx, 0, self.record, from, to) } {
            0 => Ok(()),
            _ => Err(NO_ROOM),
        }
    }

    /// # Safety
    ///
    /// `at` holds an item no one uses after.
    unsafe fn drop(&self, ctx: &mut Context, at: *mut u8) {
        if self.record != 0 {
            // SAFETY: as the caller vouches; the record is the item's.
            unsafe { rt_drop(ctx, self.record, at.expose_provenance() as u64) };
        }
    }

    /// Sorts `xs` by the order of `kind` (M2-14 section 1.13).
    fn sort(&self, xs: &ListWords, kind: u64) -> Done {
        let word = |i: usize| {
            // SAFETY: `i` is below the length, and an Int or Float item is one word.
            unsafe { *self.at(xs, i as u64).cast::<u64>() }
        };
        let order = |r: usize, l: usize| match kind {
            INT => (word(r) as i64).cmp(&(word(l) as i64)),
            FLOAT => {
                let (r, l) = (Float::from_bits(word(r)), Float::from_bits(word(l)));
                r.partial_cmp(&l).unwrap_or(Ordering::Equal)
            }
            // SAFETY: a HELD item is a String's words.
            _ => unsafe {
                as_str(self.at(xs, r as u64).addr() as u64)
                    .cmp(as_str(self.at(xs, l as u64).addr() as u64))
            },
        };
        let Ok(order) =
            sorted::<Infallible>(xs.len as usize, |r, l| Ok(order(r, l) == Ordering::Less));
        self.permute(xs, order);
        Ok(())
    }

    /// Puts item `order[i]` at place `i`.
    fn permute(&self, xs: &ListWords, order: Vec<usize>) {
        let bytes = (xs.len * self.size) as usize;
        let mut old = vec![0u8; bytes];
        // SAFETY: the block holds `bytes` of items, apart from `old`.
        unsafe { ptr::copy_nonoverlapping(xs.ptr, old.as_mut_ptr(), bytes) };
        let size = self.size as usize;
        for (i, from) in order.into_iter().enumerate() {
            // SAFETY: both places are items of a List of `len` items.
            unsafe {
                ptr::copy_nonoverlapping(old.as_ptr().add(from * size), self.at(xs, i as u64), size)
            };
        }
    }
}
