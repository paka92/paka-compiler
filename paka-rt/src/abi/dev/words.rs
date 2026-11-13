//! Strings and lists as compiled code holds them (M4-32, docs/decisions/M4-05.md section 8), and
//! the routines that build them. The String rows run `ops::Text`, which the interpreter shares.
//!
//! Each result is a new block its caller owns.

use super::{BAD_CONVERSION, Context, MEMORY, OUT_OF_BOUNDS, Trap};
use crate::ops::{self, CAP, Float, Int, Text, white};
use std::str;
use std::{ptr, slice};

/// A String: UTF-8 bytes, no NUL. With `cap` 0 it owns no block: `ptr` is null or points at a
/// constant.
#[repr(C)]
pub struct TextWords {
    pub ptr: *mut u8,
    pub bytes: u64,
    pub chars: u64,
    pub cap: u64,
}

/// A List: `len` items of one size. With `cap` 0 it owns no block: `ptr` is null or points at a
/// constant.
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
/// The rows from here make a List of Strings: `a`'s words, its pieces by `b`, its code points, its
/// lines.
pub const WORDS: u64 = 9;
pub const SPLIT: u64 = 10;
pub const CHARS: u64 = 11;
pub const LINES: u64 = 12;
/// Bool rows, whose result is a word 0 or 1.
pub const CONTAINS: u64 = 13;
pub const STARTS_WITH: u64 = 14;
pub const ENDS_WITH: u64 = 15;
/// `Option` rows, whose result is a tag word, 0 for `Some`, and the payload: an Int or Float's bits.
pub const FIND: u64 = 16;
pub const PARSE_INT: u64 = 17;
pub const PARSE_FLOAT: u64 = 18;
/// The List of Strings `a` joined by `b`.
pub const JOIN_PARTS: u64 = 19;
pub const REPEAT: u64 = 20;
pub const PAD_LEFT: u64 = 21;
pub const PAD_RIGHT: u64 = 22;
pub const TRIM: u64 = 23;
pub const TRIM_START: u64 = 24;
pub const TRIM_END: u64 = 25;
pub const REPLACE: u64 = 26;
pub const UPPER: u64 = 27;
pub const LOWER: u64 = 28;
/// The Float with bits `a` to `b` places.
pub const FIXED: u64 = 29;
/// The `b` Strings whose words' addresses are the `b` words at `a`, joined in order: an
/// interpolation's parts.
pub const JOIN_ALL: u64 = 30;

/// What a row makes.
enum Made {
    Text(Text),
    Word(u64),
    Option(Option<u64>),
}

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
    let str = |w: u64| unsafe { as_str(w) };
    let text = |w: u64| Text::from(str(w));
    let int = |w: u64| Int::new(w as i64);
    if (WORDS..=LINES).contains(&row) {
        let sep = (row == SPLIT).then(|| text(b));
        // SAFETY: the caller vouches `a` is a String's words.
        let w = unsafe { words(a) };
        let ascii = w.chars == w.bytes;
        return pieces(ctx, row, str(a), ascii, sep.as_ref(), site, out.cast());
    }
    if row == JOIN || row == JOIN_ALL {
        let pair = [a, b];
        let parts = match row {
            JOIN => &pair[..],
            // SAFETY: the caller vouches `a` holds `b` words, `b` at least 1.
            _ => unsafe { slice::from_raw_parts(ptr::with_exposed_provenance(a as usize), b as _) },
        };
        // SAFETY: the caller vouches each part is a String's words.
        let chars = parts.iter().map(|&p| unsafe { words(p).chars }).sum();
        let made = new_text_of(ctx, parts.iter().map(|&p| str(p)), chars);
        return put(ctx, made, site, out);
    }
    let word = |b: bool| Ok(Made::Word(b.into()));
    let made = match row {
        COMPARE => Ok(Made::Word(str(a).cmp(str(b)) as i64 as u64)),
        INDEX => text(a).index(int(b)).map(Made::Text),
        SLICE => text(a).slice(int(b), int(c)).map(Made::Text),
        CODE_AT => (text(a).code_at(int(b))).map(|n| Made::Word(raw(n) as u64)),
        FROM_CODE => Text::from_code(int(a)).map(Made::Text),
        SHOW if b == 0 => Ok(Made::Text(Text::from(int(a).to_string()))),
        SHOW => Ok(Made::Text(Text::from(if a == 0 {
            "false"
        } else {
            "true"
        }))),
        MIN => Ok(Made::Text(str(a).min(str(b)).into())),
        MAX => Ok(Made::Text(str(a).max(str(b)).into())),
        CONTAINS => word(str(a).contains(str(b))),
        STARTS_WITH => word(str(a).starts_with(str(b))),
        ENDS_WITH => word(str(a).ends_with(str(b))),
        FIND => Ok(Made::Option(text(a).find(&text(b)).map(|n| raw(n) as u64))),
        PARSE_INT => Ok(Made::Option(Int::parse(str(a)).map(|n| raw(n) as u64))),
        PARSE_FLOAT => Ok(Made::Option(Float::parse(str(a)).map(Float::bits))),
        JOIN_PARTS => {
            // SAFETY: the caller vouches `a` is a List of Strings' words.
            let parts = unsafe { strings(a) };
            text(b).join(&parts).map(Made::Text)
        }
        REPEAT => text(a).repeat(int(b)).map(Made::Text),
        PAD_LEFT | PAD_RIGHT => text(a).pad(int(b), row == PAD_LEFT).map(Made::Text),
        TRIM => Ok(Made::Text(str(a).trim_matches(white).into())),
        TRIM_START => Ok(Made::Text(str(a).trim_start_matches(white).into())),
        TRIM_END => Ok(Made::Text(str(a).trim_end_matches(white).into())),
        REPLACE => text(a).replace(&text(b), &text(c)).map(Made::Text),
        UPPER => Ok(Made::Text(str(a).to_ascii_uppercase().into())),
        LOWER => Ok(Made::Text(str(a).to_ascii_lowercase().into())),
        FIXED => (Float::from_bits(a).fixed(int(b))).map(|t| Made::Text(t.into())),
        _ => unreachable!("no String row {row}"),
    };
    let made = match made {
        Ok(Made::Text(t)) => t,
        Ok(Made::Word(w)) => {
            // SAFETY: `out` has room for a word.
            unsafe { *out = w };
            return 0;
        }
        Ok(Made::Option(o)) => {
            // SAFETY: `out` has room for an `Option` of a word.
            unsafe {
                out.cast::<[u64; 2]>()
                    .write([u64::from(o.is_none()), o.unwrap_or(0)])
            };
            return 0;
        }
        Err(e) => return fail(ctx, kind(e), site, operands(row, a, b, c)),
    };
    let made = new_text(ctx, made.as_str());
    put(ctx, made, site, out)
}

/// Writes a new String's words to `out`, or traps `memory` at `site` when none was made.
fn put(ctx: &mut Context, made: Option<TextWords>, site: u64, out: *mut u64) -> i32 {
    let Some(words) = made else {
        return fail(ctx, MEMORY, site, [0; 3]);
    };
    // SAFETY: `out` has room for a String's words.
    unsafe { out.cast::<TextWords>().write(words) };
    0
}

/// The trap kind of an operation's trap, by M4-05 section 6's number.
fn kind(e: ops::Trap) -> u64 {
    match e {
        ops::Trap::OutOfBounds => OUT_OF_BOUNDS,
        ops::Trap::BadConversion => BAD_CONVERSION,
        ops::Trap::Memory => MEMORY,
        ops::Trap::Overflow | ops::Trap::DivideByZero => unreachable!("no String row overflows"),
    }
}

/// A trap's operands: `from_code`'s code, an index and the length, or a slice's ends and the
/// length; a count or a digit count alone.
fn operands(row: u64, a: u64, b: u64, c: u64) -> [u64; 3] {
    // SAFETY: the rows that index take a String first.
    let len = || raw(Text::from(unsafe { as_str(a) }).len()) as u64;
    match row {
        FROM_CODE => [a, 0, 0],
        SLICE => [b, c, len()],
        INDEX | CODE_AT => [b, len(), 0],
        _ => [b, 0, 0],
    }
}

/// The List of Strings row `row` makes of `t`, by `sep` for `split`, into `out`; `ascii` when
/// each of `t`'s bytes is a code point.
fn pieces(
    ctx: &mut Context,
    row: u64,
    t: &str,
    ascii: bool,
    sep: Option<&Text>,
    site: u64,
    out: *mut ListWords,
) -> i32 {
    let list = match row {
        WORDS => list_of(ctx, t.split(white).filter(|w| !w.is_empty()), ascii),
        _ => {
            let t = Text::from(t);
            let parts = match row {
                SPLIT => t.split(sep.expect("a separator")),
                CHARS => Ok(t.chars().collect()),
                _ => Ok(t.lines()),
            };
            let Ok(parts) = parts else {
                return fail(ctx, MEMORY, site, [0; 3]);
            };
            list_of(ctx, parts.iter().copied(), ascii)
        }
    };
    let Some(list) = list else {
        return fail(ctx, MEMORY, site, [0; 3]);
    };
    // SAFETY: `out` has room for a List's words.
    unsafe { out.write(list) };
    0
}

/// # Safety
///
/// `at` is the address of a List of Strings' words, live and unchanged while the result is used.
unsafe fn strings<'a>(at: u64) -> Vec<&'a str> {
    // SAFETY: the caller vouches `at` holds a List's words.
    let list = unsafe { &*ptr::with_exposed_provenance::<ListWords>(at as usize) };
    let item = |i| list.ptr as u64 + i * size_of::<TextWords>() as u64;
    // SAFETY: each of the List's items is a String's words.
    (0..list.len).map(|i| unsafe { as_str(item(i)) }).collect()
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

/// A new List of Strings holding `parts`, each a code point per byte when `ascii`; `None`, with
/// nothing left made, when the heap refuses.
fn list_of<'a>(
    ctx: &mut Context,
    parts: impl Iterator<Item = &'a str> + Clone,
    ascii: bool,
) -> Option<ListWords> {
    const SIZE: usize = size_of::<TextWords>();
    let count = parts.clone().count();
    let block = match count {
        0 => ptr::null_mut(),
        _ => ctx.heap.alloc((count * SIZE) as u64)?,
    };
    for (i, part) in parts.enumerate() {
        let chars = if ascii {
            part.len()
        } else {
            part.chars().count()
        };
        let Some(made) = new_text_of(ctx, [part].into_iter(), chars as u64) else {
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
    new_text_of(ctx, [s].into_iter(), s.chars().count() as u64)
}

/// A new String block holding `parts` in turn, `chars` characters in all; `None` past the cap,
/// checked before the block is made, or when the heap refuses.
fn new_text_of<'a>(
    ctx: &mut Context,
    parts: impl Iterator<Item = &'a str> + Clone,
    chars: u64,
) -> Option<TextWords> {
    let bytes = parts
        .clone()
        .fold(0, |n: usize, p| n.saturating_add(p.len()));
    if bytes > CAP {
        return None;
    }
    let ptr = match bytes {
        0 => ptr::null_mut(),
        _ => {
            let ptr = ctx.heap.alloc(bytes as u64)?;
            let mut at = 0;
            for p in parts {
                // SAFETY: `ptr` is a new block of `bytes`, apart from each part.
                unsafe { ptr::copy_nonoverlapping(p.as_ptr(), ptr.add(at), p.len()) };
                at += p.len();
            }
            ptr
        }
    };
    let bytes = bytes as u64;
    Some(TextWords {
        ptr,
        bytes,
        chars,
        cap: bytes,
    })
}

/// # Safety
///
/// `at` is the address of a String's words, whose bytes stay live and unchanged while it is used.
pub(crate) unsafe fn as_str<'a>(at: u64) -> &'a str {
    // SAFETY: the caller vouches `at` holds a String's words.
    let words = unsafe { words(at) };
    if words.bytes == 0 {
        return "";
    }
    // SAFETY: a String's words point to `bytes` bytes of UTF-8.
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(words.ptr, words.bytes as usize)) }
}

/// # Safety
///
/// `at` is the address of a String's words, which stay live while they are used.
unsafe fn words<'a>(at: u64) -> &'a TextWords {
    // SAFETY: the caller vouches `at` holds a String's words.
    unsafe { &*ptr::with_exposed_provenance::<TextWords>(at as usize) }
}

/// An Int leaves Paka here, into the runtime's words.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}
