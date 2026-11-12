//! The dev runtime's context and allocator (M4-25, docs/decisions/M4-05.md sections 4 and 9).

pub mod app;
mod blocks;
pub mod items;
pub mod map;
pub mod records;
pub mod view;
pub mod windowless;
pub mod words;

pub use blocks::{Fault, Heap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Trap kinds (M4-05 section 6).
pub const OVERFLOW: u64 = 1;
pub const DIVIDE_BY_ZERO: u64 = 2;
pub const OUT_OF_BOUNDS: u64 = 3;
pub const BAD_CONVERSION: u64 = 4;
pub const ASSERT: u64 = 5;
pub const HOLE: u64 = 6;
pub const STACK: u64 = 7;
pub const MEMORY: u64 = 8;

/// A run's context (M4-05 section 4): four words generated code reads, then the runtime's own.
#[repr(C)]
pub struct Context {
    pub slots: *const usize,
    pub rt: &'static Rt,
    pub stack_limit: usize,
    pub interrupt: AtomicU64,
    pub trap: Trap,
    pub heap: Heap,
    /// The tree a view is drawing (`view`).
    pub tree: Vec<u64>,
}

/// Runs `f` on `ctx`, setting its interrupt once `limit` passes (docs/decisions/M4-04.md); and
/// whether it was set.
pub fn until<R>(
    ctx: &mut Context,
    limit: Duration,
    f: impl FnOnce(&mut Context) -> R,
) -> (R, bool) {
    let flag = (&raw const ctx.interrupt).expose_provenance();
    let interrupt = move || {
        // SAFETY: the context outlives `until`, and its interrupt is an atomic that the run only
        // ever loads.
        let interrupt = unsafe { &*std::ptr::with_exposed_provenance::<AtomicU64>(flag) };
        interrupt.store(1, Ordering::Relaxed);
    };
    crate::limit::until(limit, interrupt, || f(ctx))
}

impl Context {
    /// A context whose heap holds at most `cap` bytes; the slot table is set when a run starts.
    /// Its `stack_limit` is 0, which turns the stack check off: runs that may recurse use
    /// `on_runner`.
    pub fn new(cap: u64) -> Context {
        Context {
            slots: std::ptr::null(),
            rt: &RT,
            stack_limit: 0,
            interrupt: AtomicU64::new(0),
            trap: Trap::default(),
            heap: Heap::new(cap),
            tree: vec![],
        }
    }
}

/// The trap record (M4-05 section 6); kind 0 when nothing trapped.
#[derive(Debug, Default, PartialEq)]
pub struct Trap {
    pub kind: u64,
    /// `definition << 32 | site`; 0 for `stack`, whose position is its first frame.
    pub site: u64,
    /// The detail's operands; Float operands as their bits.
    pub operands: [u64; 3],
    /// The call sites the trap passed through, innermost first: the first eight.
    pub frames: Vec<u64>,
    /// More than eight frames were pushed.
    pub more: bool,
}

/// The frames a trap keeps (M2-06 section 3).
const FRAMES: usize = 8;

/// The runtime routines generated code calls, at fixed offsets.
#[repr(C)]
pub struct Rt {
    pub alloc: extern "sysv64" fn(&mut Context, u64, &mut *mut u8) -> i32,
    pub release: extern "sysv64" fn(&mut Context, *mut u8, u64),
    pub trap: extern "sysv64" fn(&mut Context, u64, u64, u64, u64, u64),
    pub push_frame: extern "sysv64" fn(&mut Context, u64),
    pub text: unsafe extern "sysv64" fn(&mut Context, u64, u64, u64, u64, u64, *mut u64) -> i32,
    pub list:
        unsafe extern "sysv64" fn(&mut Context, u64, u64, u64, u64, *mut words::ListWords) -> i32,
    pub copy: unsafe extern "sysv64" fn(&mut Context, u64, u64, u64, u64) -> i32,
    pub drop: unsafe extern "sysv64" fn(&mut Context, u64, u64),
    #[allow(clippy::type_complexity)]
    pub items: unsafe extern "sysv64" fn(
        &mut Context,
        u64,
        u64,
        u64,
        u64,
        *mut words::ListWords,
        u64,
        u64,
        *mut u8,
    ) -> i32,
    pub sort_by:
        unsafe extern "sysv64" fn(&mut Context, u64, u64, *const words::ListWords, u64) -> i32,
    #[allow(clippy::type_complexity)]
    pub map: unsafe extern "sysv64" fn(
        &mut Context,
        u64,
        u64,
        u64,
        u64,
        *mut *mut u8,
        u64,
        *mut u8,
    ) -> i32,
    pub view: unsafe extern "sysv64" fn(&mut Context, u64, u64, u64, u64, u64, u64) -> i32,
}

pub static RT: Rt = Rt {
    alloc: rt_alloc,
    release: rt_release,
    trap: rt_trap,
    push_frame: rt_push_frame,
    text: words::rt_text,
    list: words::rt_list,
    copy: records::rt_copy,
    drop: records::rt_drop,
    items: items::rt_items,
    sort_by: items::rt_sort_by,
    map: map::rt_map,
    view: view::rt_view,
};

/// `rt.alloc`: a block of `bytes` into `out` and status 0, or a `memory` trap and status 1.
pub extern "sysv64" fn rt_alloc(ctx: &mut Context, bytes: u64, out: &mut *mut u8) -> i32 {
    match ctx.heap.alloc(bytes) {
        Some(ptr) => {
            *out = ptr;
            0
        }
        None => {
            ctx.trap.kind = MEMORY;
            1
        }
    }
}

/// `rt.release`: frees a block; never traps.
pub extern "sysv64" fn rt_release(ctx: &mut Context, ptr: *mut u8, bytes: u64) {
    ctx.heap.release(ptr, bytes);
}

/// `rt.trap`: records a failed check; the caller returns status 1.
pub extern "sysv64" fn rt_trap(ctx: &mut Context, kind: u64, site: u64, a: u64, b: u64, c: u64) {
    ctx.trap = Trap {
        kind,
        site,
        operands: [a, b, c],
        ..Trap::default()
    };
}

/// `rt.push_frame`: records the call site a trap passed through.
pub extern "sysv64" fn rt_push_frame(ctx: &mut Context, site: u64) {
    if ctx.trap.frames.len() < FRAMES {
        ctx.trap.frames.push(site);
    } else {
        ctx.trap.more = true;
    }
}

/// A runner thread's stack, and the room kept below `stack_limit` (M4-05 section 6).
pub const RUNNER_STACK: usize = 64 << 20;
pub(crate) const MARGIN: usize = 1 << 20;

/// Runs `f` on a new thread with a `RUNNER_STACK` stack and a fresh context of heap cap `cap`, its
/// `stack_limit` about `MARGIN` above the bottom. The context sits near the stack's top, so its
/// own address stands for the top; what lies above it only shrinks the margin a little.
pub fn on_runner<R: Send>(cap: u64, f: impl FnOnce(&mut Context) -> R + Send) -> R {
    std::thread::scope(|s| {
        let runner = std::thread::Builder::new().stack_size(RUNNER_STACK);
        let run = runner.spawn_scoped(s, || {
            let mut ctx = Context::new(cap);
            ctx.stack_limit = (&raw const ctx).addr() - (RUNNER_STACK - MARGIN);
            f(&mut ctx)
        });
        let run = run.expect("a runner thread");
        run.join().unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}
