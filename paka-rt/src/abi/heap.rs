//! The heap of a release run (ABI section 2, "Memory"): the entries compiled code calls to make,
//! copy and free blocks and to run the String rows. They work on one dev `Context` per call of
//! `update`, on the thread that makes it, so every block is in its ledger.

use super::dev::{BAD_CONVERSION, Context, Heap, MEMORY, OUT_OF_BOUNDS, map, records, words};
use super::{List, Str, Update, record_kind};
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
    /// An entry panicked, or was called outside `call`.
    static BROKEN: Cell<bool> = const { Cell::new(false) };
}

/// What a call of `update` left.
pub struct Called {
    pub status: i32,
    pub heap: Heap,
    pub broken: bool,
}

/// Calls `update` on `msg` with a new heap.
pub fn call(update: Update, msg: u64, out: &mut List) -> Called {
    CONTEXT.set(Some(Context::new(u64::MAX)));
    BROKEN.set(false);
    let status = update(msg, out);
    let ctx = CONTEXT.take().expect("the call's context");
    Called {
        status,
        heap: ctx.heap,
        broken: BROKEN.take(),
    }
}

/// `f` on the call's context; `failed` when there is none or `f` panics.
fn with<R>(failed: R, f: impl FnOnce(&mut Context) -> R) -> R {
    let run = || CONTEXT.with_borrow_mut(|c| c.as_mut().map(f));
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Some(r)) => r,
        _ => {
            BROKEN.set(true);
            failed
        }
    }
}

/// A new block of `bytes`, aligned 8; null when the heap refuses.
// SAFETY: as for `paka_rt1_run`.
#[unsafe(no_mangle)]
pub extern "C" fn paka_rt1_alloc(bytes: u64) -> *mut u8 {
    with(ptr::null_mut(), |ctx| {
        ctx.heap.alloc(bytes).unwrap_or(ptr::null_mut())
    })
}

/// The dev runtime's String row `row` on words `a`, `b` and `c`, its result into `out`. On a
/// trap, records its kind with the text `at`, its position, and returns 1.
///
/// # Safety
///
/// The operands and `out` are as `rt_text` asks, and `at` and `len` are a string as ABI section 2
/// defines it.
// SAFETY: as for `paka_rt1_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn paka_rt1_text(
    row: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
    at: *const u8,
    len: u64,
) -> i32 {
    let (status, kind) = with((1, 0), |ctx| {
        // SAFETY: the caller vouches for the operands and `out`.
        let status = unsafe { words::rt_text(ctx, row, 0, a, b, c, out) };
        (status, ctx.trap.kind)
    });
    // SAFETY: the caller promises an ABI string.
    unsafe { trapped(status, kind, at, len) }
}

/// The dev runtime's Map row `row` (`rt_map`) on the Map whose word is at `map`; on a trap,
/// records its kind with the text `at` and returns 1.
///
/// # Safety
///
/// The operands are as `rt_map` asks, `record` the value type's record or null, and `at` and `len`
/// a string as ABI section 2 defines it.
// SAFETY: as for `paka_rt1_run`.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn paka_rt1_map(
    row: u64,
    record: *const u64,
    shape: u64,
    map: *mut *mut u8,
    key: u64,
    out: *mut u8,
    at: *const u8,
    len: u64,
) -> i32 {
    let record = record.expose_provenance() as u64;
    let (status, kind) = with((1, 0), |ctx| {
        // SAFETY: the caller vouches for the operands and `out`.
        let status = unsafe { map::rt_map(ctx, row, 0, record, shape, map, key, out) };
        (status, ctx.trap.kind)
    });
    // SAFETY: the caller promises an ABI string.
    unsafe { trapped(status, kind, at, len) }
}

/// Records a trap of `kind` at the text `at` when a row's `status` is not 0, and gives `status`.
///
/// # Safety
///
/// `at` and `len` are a string as ABI section 2 defines it.
unsafe fn trapped(status: i32, kind: u64, at: *const u8, len: u64) -> i32 {
    if status != 0 {
        let kind: &[u8] = match kind {
            OUT_OF_BOUNDS => b"out_of_bounds ",
            BAD_CONVERSION => b"bad_conversion ",
            MEMORY => b"memory ",
            _ => b"",
        };
        // SAFETY: the caller promises an ABI string, which is what `from_raw` asks for.
        record_kind(kind, unsafe { Str::from_raw(at, len) }.bytes());
    }
    status
}

/// Copies the value at `from` into `to`, every block it owns new; 1 when the heap refuses.
///
/// # Safety
///
/// `record` is the value's type record (M4-05 section 10), and `to` has room for the value apart
/// from `from`.
// SAFETY: as for `paka_rt1_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn paka_rt1_copy(record: *const u64, from: *const u8, to: *mut u8) -> i32 {
    let (record, from) = (record.expose_provenance(), from.expose_provenance());
    let [record, from, to] = [record, from, to.expose_provenance()].map(|a| a as u64);
    // SAFETY: the caller vouches for the record and both places.
    with(1, |ctx| unsafe {
        records::rt_copy(ctx, 0, record, from, to)
    })
}

/// Frees every block the value at `at` owns.
///
/// # Safety
///
/// `record` is the value's type record, and no one uses the value after.
// SAFETY: as for `paka_rt1_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn paka_rt1_drop(record: *const u64, at: *mut u8) {
    let [record, at] = [record.expose_provenance(), at.expose_provenance()].map(|a| a as u64);
    // SAFETY: the caller vouches for the record and the value.
    with((), |ctx| unsafe { records::rt_drop(ctx, record, at) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{Effect, PRINT};
    use crate::rt::run_with;
    use words::{INDEX, TextWords};

    fn run(update: Update) -> (i32, String, String) {
        let (mut out, mut err) = (vec![], vec![]);
        let status = run_with(&mut out, &mut err, Some(b"app"), Some(update), 0, 1, 0);
        let text = |b| String::from_utf8(b).unwrap();
        (status, text(out), text(err))
    }

    /// Prints a text from a block, its list in a block too, then exits; `leak` makes one more.
    fn made(out: &mut List, leak: bool) -> i32 {
        let (text, fx) = (paka_rt1_alloc(2), paka_rt1_alloc(48).cast::<Effect>());
        if leak {
            paka_rt1_alloc(8);
        }
        // SAFETY: both blocks are new, of the sizes written.
        unsafe {
            text.copy_from(b"hi".as_ptr(), 2);
            let len = 2;
            fx.write(Effect {
                tag: PRINT,
                ptr: text,
                len,
            });
            fx.add(1).write(Effect::EXIT);
        }
        *out = List { ptr: fx, count: 2 };
        0
    }

    extern "C" fn freed(_: u64, out: &mut List) -> i32 {
        made(out, false)
    }

    extern "C" fn leaked(_: u64, out: &mut List) -> i32 {
        made(out, true)
    }

    #[test]
    fn effects_in_blocks_are_freed_once_performed() {
        assert_eq!(run(freed), (0, "hi\n".into(), "".into()));
    }

    #[test]
    fn a_block_left_after_the_effects_is_an_internal_error() {
        let err = "app: internal error: memory ledger not clean\n";
        assert_eq!(run(leaked), (4, "hi\n".into(), err.into()));
    }

    extern "C" fn indexes_past_the_end(_: u64, _: &mut List) -> i32 {
        static AB: &[u8] = b"ab";
        static AT: &[u8] = b"at main.paka:1:2";
        let ab = TextWords {
            ptr: AB.as_ptr().cast_mut(),
            bytes: 2,
            chars: 2,
            cap: 0,
        };
        let a = (&raw const ab).expose_provenance() as u64;
        let mut out = [0u64; 4];
        // SAFETY: `a` is a String's words and `out` has room for one.
        unsafe { paka_rt1_text(INDEX, a, 5, 0, out.as_mut_ptr(), AT.as_ptr(), 16) }
    }

    #[test]
    fn a_string_row_traps_with_its_kind() {
        let err = "app: trap out_of_bounds at main.paka:1:2\n";
        assert_eq!(run(indexes_past_the_end), (3, "".into(), err.into()));
    }
}
