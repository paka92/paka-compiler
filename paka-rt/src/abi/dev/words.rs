//! Strings and lists as compiled code holds them (M4-32, docs/decisions/M4-05.md section 8), and
//! the routines that build them. The String rows run `ops::Text`, which the interpreter shares.
//!
//! Each result is a new block its caller owns.

use super::{BAD_CONVERSION, Context, MEMORY, OUT_OF_BOUNDS, Trap};
use crate::ops::{self, CAP, Int, Text};
use std::str;
use std::{ptr, slice};

/// A String: UTF-8 bytes, no NUL; `ptr` is null when `cap` is 0.
#[repr(C)]
pub struct TextWords {
    pub ptr: *mut u8,
    pub bytes: u64,
    pub chars: u64,
    pub cap: u64,
}

/// A List: `len` items of one size; `ptr` is null when `cap` is 0.
#[repr(C)]
pub struct ListWords {
    pub ptr: *mut u8,
    pub len: u64,
    pub cap: u64,
}

/// The String rows of `rt.text`, by its `row` word.
pub const JOIN: u64 = 0;
pub const COMPARE: u64 = 1;
pub const INDEX: u64 = 2;
pub const SLICE: u64 = 3;
pub const CODE_AT: u64 = 4;
pub const FROM_CODE: u64 = 5;
/// `a` as an Int's decimal text, or with `b` set as a Bool's `true` or `false`.
pub const SHOW: u64 = 6;
pub const MIN: u64 = 7;
pub const MAX: u64 = 8;
/// A List of `a`'s words, into a List's words.
pub const WORDS: u64 = 9;

/// `rt.text`: String row `row` on words `a`, `b`, `c`, where a String is the address of its words.
/// The result goes to `out`: a String's words, or an Int. A trap is recorded at `site`.
///
/// # Safety
///
/// Each String operand is the address of a String's words, and `out` has room for the result.
pub unsafe extern "sysv64" fn rt_text(
    ctx: &mut Context,
    row: u64,
    site: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
) -> i32 {
    // SAFETY: the caller vouches each String operand is a String's words.
    let text = |w: u64| Text::from(unsafe { as_str(w) });
    let int = |w: u64| Int::new(w as i64);
    let made = match row {
        JOIN => Ok(text(a).add(&text(b))),
        COMPARE => {
            let order = text(a).cmp(&text(b)) as i64;
            // SAFETY: `out` has room for an Int.
            unsafe { *out = order as u64 };
            return 0;
        }
        INDEX => text(a).index(int(b)),
        SLICE => text(a).slice(int(b), int(c)),
        CODE_AT => match text(a).code_at(int(b)) {
            Ok(code) => {
                // SAFETY: `out` has room for an Int.
                unsafe { *out = raw(code) as u64 };
                return 0;
            }
            Err(e) => Err(e),
        },
        FROM_CODE => Text::from_code(int(a)),
        SHOW if b == 0 => Ok(Text::from(int(a).to_string())),
        SHOW => Ok(Text::from(if a == 0 { "false" } else { "true" })),
        MIN => Ok(text(a).min(text(b))),
        MAX => Ok(text(a).max(text(b))),
        WORDS => {
            let Some(list) = words(ctx, &text(a)) else {
                return fail(ctx, MEMORY, site, [0; 3]);
            };
            // SAFETY: `out` has room for a List's words.
            unsafe { out.cast::<ListWords>().write(list) };
            return 0;
        }
        _ => unreachable!("no String row {row}"),
    };
    let made = match made {
        Ok(t) => t,
        // `from_code` alone converts; the others index: `[i, len]`, or `[from, to, len]`.
        Err(ops::Trap::BadConversion) => return fail(ctx, BAD_CONVERSION, site, [a, 0, 0]),
        Err(_) => {
            let len = raw(text(a).len()) as u64;
            let operands = match row {
                SLICE => [b, c, len],
                _ => [b, len, 0],
            };
            return fail(ctx, OUT_OF_BOUNDS, site, operands);
        }
    };
    let Some(words) = new_text(ctx, made.as_str()) else {
        return fail(ctx, MEMORY, site, [0; 3]);
    };
    // SAFETY: `out` has room for a String's words.
    unsafe { out.cast::<TextWords>().write(words) };
    0
}

/// `rt.list`: a new List of `count` items of `size` bytes into `out`, copied from `from` unless
/// it is 0; a `memory` trap at `site` past the cap.
///
/// # Safety
///
/// `from` is 0 or the address of `count` items of `size` bytes, and `out` has room for a List.
pub unsafe extern "sysv64" fn rt_list(
    ctx: &mut Context,
    site: u64,
    from: u64,
    count: u64,
    size: u64,
    out: *mut ListWords,
) -> i32 {
    let bytes = count.saturating_mul(size);
    let block = match count {
        0 => ptr::null_mut(),
        _ if count > CAP as u64 => return fail(ctx, MEMORY, site, [0; 3]),
        // A List of 0-byte items still takes a block, so `ptr` is null only when `cap` is 0.
        _ => match ctx.heap.alloc(bytes) {
            Some(block) => block,
            None => return fail(ctx, MEMORY, site, [0; 3]),
        },
    };
    if bytes != 0 && from == 0 {
        // SAFETY: `block` is a new block of `bytes`; items that own blocks start as none.
        unsafe { ptr::write_bytes(block, 0, bytes as usize) };
    } else if bytes != 0 {
        let from = ptr::with_exposed_provenance::<u8>(from as usize);
        // SAFETY: the caller vouches for `count` items at `from`; `block` is a new block of
        // `bytes`, so the two never overlap.
        unsafe { ptr::copy_nonoverlapping(from, block, bytes as usize) };
    }
    let words = ListWords {
        ptr: block,
        len: count,
        cap: count,
    };
    // SAFETY: the caller vouches `out` has room for a List.
    unsafe { out.write(words) };
    0
}

/// A new List of `t`'s words; `None`, with nothing left made, when the heap refuses.
fn words(ctx: &mut Context, t: &Text) -> Option<ListWords> {
    const SIZE: usize = size_of::<TextWords>();
    let count = t.words().count();
    let block = match count {
        0 => ptr::null_mut(),
        _ => ctx.heap.alloc((count * SIZE) as u64)?,
    };
    for (i, word) in t.words().enumerate() {
        let Some(made) = new_text(ctx, word) else {
            // SAFETY: items `..i` were written below; each owns its block.
            let made = unsafe { slice::from_raw_parts(block.cast::<TextWords>(), i) };
            for m in made.iter().filter(|m| m.cap != 0) {
                ctx.heap.release(m.ptr, m.cap);
            }
            ctx.heap.release(block, (count * SIZE) as u64);
            return None;
        };
        // SAFETY: `block` holds `count` items and `i` is below it.
        unsafe { block.cast::<TextWords>().add(i).write(made) };
    }
    let len = count as u64;
    Some(ListWords {
        ptr: block,
        len,
        cap: len,
    })
}

fn fail(ctx: &mut Context, kind: u64, site: u64, operands: [u64; 3]) -> i32 {
    ctx.trap = Trap {
        kind,
        site,
        operands,
        ..Trap::default()
    };
    1
}

/// A new String block holding `s`; `None` past the cap or when the heap refuses.
pub(super) fn new_text(ctx: &mut Context, s: &str) -> Option<TextWords> {
    let bytes = s.len() as u64;
    if s.len() > CAP {
        return None;
    }
    let ptr = match bytes {
        0 => ptr::null_mut(),
        _ => {
            let ptr = ctx.heap.alloc(bytes)?;
            // SAFETY: `ptr` is a new block of `bytes`, apart from `s`.
            unsafe { ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len()) };
            ptr
        }
    };
    Some(TextWords {
        ptr,
        bytes,
        chars: s.chars().count() as u64,
        cap: bytes,
    })
}

/// # Safety
///
/// `at` is the address of a String's words, whose bytes stay live and unchanged while it is used.
pub(super) unsafe fn as_str<'a>(at: u64) -> &'a str {
    // SAFETY: the caller vouches `at` holds a String's words.
    let words = unsafe { &*ptr::with_exposed_provenance::<TextWords>(at as usize) };
    if words.bytes == 0 {
        return "";
    }
    // SAFETY: a String's words point to `bytes` bytes of UTF-8.
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(words.ptr, words.bytes as usize)) }
}

/// An Int leaves Paka here, into the runtime's words.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}
