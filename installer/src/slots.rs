//! The slot table and the entry into installed code (M4-24, docs/decisions/M4-05.md sections 4, 5
//! and 7). Compiled code calls a definition by loading its slot, so a swap redirects every caller.
//! Swaps and runs exclude each other: a run always finishes on the code it started with.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

const IDLE: u8 = 0;
const RUNNING: u8 = 1;
const SWAPPING: u8 = 2;

/// Why a swap or a run was refused; nothing was changed or called.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// A run or a swap is under way.
    Busy,
    /// The table has no slot with this id.
    NoSlot(usize),
}

/// One word per definition id: its code address, or its type record address for a type.
pub struct Slots {
    words: Box<[AtomicUsize]>,
    state: AtomicU8,
}

impl Slots {
    /// `len` slots, each 0 until swapped.
    pub fn new(len: usize) -> Slots {
        Slots {
            words: (0..len).map(|_| AtomicUsize::new(0)).collect(),
            state: AtomicU8::new(IDLE),
        }
    }

    /// Writes each (id, address) of `group`; all of them, or none when one id has no slot or a run
    /// is under way.
    pub fn swap(&self, group: &[(usize, usize)]) -> Result<(), Refused> {
        if let Some(&(id, _)) = group.iter().find(|(id, _)| *id >= self.words.len()) {
            return Err(Refused::NoSlot(id));
        }
        let _held = self.hold(SWAPPING)?;
        for &(id, address) in group {
            self.words[id].store(address, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Calls slot `id` as `fn(ctx, out) -> i32` and returns its status, after pointing the first
    /// word of `ctx` at this table. No swap happens until it returns.
    ///
    /// # Safety
    ///
    /// As `Session::call`, `out` being the one argument.
    pub unsafe fn run(
        &self,
        id: usize,
        ctx: *mut c_void,
        out: *mut c_void,
    ) -> Result<i32, Refused> {
        // SAFETY: the caller vouches as `Session::call` asks, `ctx` among it.
        unsafe {
            let session = self.session(ctx)?;
            session.call(id, ctx, &[out.expose_provenance() as u64])
        }
    }

    /// Holds the table for a run of many calls, after pointing the first word of `ctx` at it: no
    /// swap happens until the session drops.
    ///
    /// # Safety
    ///
    /// `ctx` points to a runtime context (M4-05 section 4).
    pub unsafe fn session(&self, ctx: *mut c_void) -> Result<Session<'_>, Refused> {
        let held = self.hold(RUNNING)?;
        // SAFETY: the caller vouches `ctx` is a context, whose first word is the slot table base.
        unsafe { ctx.cast::<*const AtomicUsize>().write(self.words.as_ptr()) };
        Ok(Session {
            words: &self.words,
            _held: held,
        })
    }

    /// Moves the table from idle to `state` until the guard drops.
    fn hold(&self, state: u8) -> Result<Held<'_>, Refused> {
        (self.state)
            .compare_exchange(IDLE, state, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| Held(&self.state))
            .map_err(|_| Refused::Busy)
    }
}

/// A held table: calls into its installed code.
pub struct Session<'a> {
    words: &'a [AtomicUsize],
    _held: Held<'a>,
}

impl Session<'_> {
    /// Calls slot `id` as `fn(ctx, args...) -> i32` and returns its status, after pointing the
    /// first word of `ctx` at this table, so a context that ran other code runs this; up to four
    /// argument words.
    ///
    /// # Safety
    ///
    /// Slot `id` holds installed code of that type, and every slot the run reaches holds what its
    /// definition's callers expect (M4-05 sections 5 and 7). `ctx` is the session's context and
    /// each argument what the definition takes, all valid for the whole call.
    pub unsafe fn call(&self, id: usize, ctx: *mut c_void, args: &[u64]) -> Result<i32, Refused> {
        type C = *mut c_void;
        let entry = self.words.get(id).ok_or(Refused::NoSlot(id))?;
        let entry = entry.load(Ordering::Relaxed);
        // SAFETY: the caller vouches `ctx` is the session's context and `entry` code taking it
        // and these argument words.
        unsafe {
            ctx.cast::<*const AtomicUsize>().write(self.words.as_ptr());
            Ok(match *args {
                [a] => {
                    std::mem::transmute::<usize, extern "sysv64" fn(C, u64) -> i32>(entry)(ctx, a)
                }
                [a, b] => std::mem::transmute::<usize, extern "sysv64" fn(C, u64, u64) -> i32>(
                    entry,
                )(ctx, a, b),
                [a, b, c] => std::mem::transmute::<
                    usize,
                    extern "sysv64" fn(C, u64, u64, u64) -> i32,
                >(entry)(ctx, a, b, c),
                [a, b, c, d] => std::mem::transmute::<
                    usize,
                    extern "sysv64" fn(C, u64, u64, u64, u64) -> i32,
                >(entry)(ctx, a, b, c, d),
                _ => unimplemented!("a call with {} argument words", args.len()),
            })
        }
    }
}

/// Returns the table to idle when dropped, even when the run panics.
struct Held<'a>(&'a AtomicU8);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.0.store(IDLE, Ordering::Release);
    }
}
