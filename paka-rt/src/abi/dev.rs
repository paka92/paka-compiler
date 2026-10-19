//! The dev runtime's context and allocator (M4-25, docs/decisions/M4-05.md sections 4 and 9).
//! Every block of a run is in a ledger, so a leak, a double free or a free of the wrong size is reported instead of
//! corrupting memory, and the run's cap is checked before the host is asked.

use std::alloc::{self, Layout};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;

/// Trap kinds (M4-05 section 6).
pub const OVERFLOW: u64 = 1;
pub const DIVIDE_BY_ZERO: u64 = 2;
pub const OUT_OF_BOUNDS: u64 = 3;
pub const BAD_CONVERSION: u64 = 4;
pub const ASSERT: u64 = 5;
pub const HOLE: u64 = 6;
pub const STACK: u64 = 7;
pub const MEMORY: u64 = 8;

/// A free the ledger refused; the memory stays as it was.
#[derive(Debug, PartialEq)]
pub enum Fault {
    /// The address is not a live block: freed already, or never allocated.
    NotLive { address: usize },
    /// The block was freed with another size; it is freed with its own.
    WrongSize {
        address: usize,
        given: u64,
        allocated: u64,
    },
}

/// A run's blocks and the faults met freeing them. Live blocks are freed when it drops.
pub struct Heap {
    /// The most bytes live at once.
    cap: u64,
    used: u64,
    /// Address to size.
    blocks: BTreeMap<usize, u64>,
    faults: Vec<Fault>,
}

impl Heap {
    pub fn new(cap: u64) -> Heap {
        Heap {
            cap,
            used: 0,
            blocks: BTreeMap::new(),
            faults: vec![],
        }
    }

    /// A new block of `bytes`, aligned 8; `None` past the cap or when the host refuses.
    pub fn alloc(&mut self, bytes: u64) -> Option<*mut u8> {
        let used = self.used.checked_add(bytes).filter(|&u| u <= self.cap)?;
        let layout = layout(bytes)?;
        // SAFETY: the layout's size is at least 1.
        let ptr = unsafe { alloc::alloc(layout) };
        if ptr.is_null() {
            return None;
        }
        self.used = used;
        self.blocks.insert(ptr.expose_provenance(), bytes);
        Some(ptr)
    }

    /// Frees the block at `ptr`, or records why it cannot. Only the ledger's own record of the
    /// block is freed, so a bad pointer is never touched.
    pub fn release(&mut self, ptr: *mut u8, bytes: u64) {
        let address = ptr.addr();
        let Some(allocated) = self.blocks.remove(&address) else {
            self.faults.push(Fault::NotLive { address });
            return;
        };
        if bytes != allocated {
            self.faults.push(Fault::WrongSize {
                address,
                given: bytes,
                allocated,
            });
        }
        self.used -= allocated;
        // SAFETY: the ledger held `address` as a live block of `allocated` bytes.
        unsafe { free(address, allocated) };
    }

    /// The blocks still live, as (address, bytes) in address order.
    pub fn leaks(&self) -> Vec<(usize, u64)> {
        self.blocks.iter().map(|(&a, &b)| (a, b)).collect()
    }

    pub fn faults(&self) -> &[Fault] {
        &self.faults
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        for (address, bytes) in std::mem::take(&mut self.blocks) {
            // SAFETY: the ledger held `address` as a live block of `bytes`.
            unsafe { free(address, bytes) };
        }
    }
}

/// The layout of a block of `bytes`; one byte at least, so that every block is its own address.
fn layout(bytes: u64) -> Option<Layout> {
    Layout::from_size_align(usize::try_from(bytes.max(1)).ok()?, 8).ok()
}

/// # Safety
///
/// `address` is a live block of `bytes` from `Heap::alloc`, which exposed its provenance.
unsafe fn free(address: usize, bytes: u64) {
    let layout = layout(bytes).expect("the layout it was allocated with");
    let ptr = std::ptr::with_exposed_provenance_mut(address);
    // SAFETY: `alloc::alloc` made the block with this layout, and the caller vouches it is live.
    unsafe { alloc::dealloc(ptr, layout) };
}

/// A run's context (M4-05 section 4): four words generated code reads, then the runtime's own.
#[repr(C)]
pub struct Context {
    pub slots: *const usize,
    pub rt: &'static Rt,
    pub stack_limit: usize,
    pub interrupt: AtomicU64,
    pub trap: Trap,
    pub heap: Heap,
}

impl Context {
    /// A context whose heap holds at most `cap` bytes; the slot table is set when a run starts.
    pub fn new(cap: u64) -> Context {
        Context {
            slots: std::ptr::null(),
            rt: &RT,
            stack_limit: 0,
            interrupt: AtomicU64::new(0),
            trap: Trap::default(),
            heap: Heap::new(cap),
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
}

pub static RT: Rt = Rt {
    alloc: rt_alloc,
    release: rt_release,
    trap: rt_trap,
    push_frame: rt_push_frame,
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
    match ctx.trap.frames.len() < FRAMES {
        true => ctx.trap.frames.push(site),
        false => ctx.trap.more = true,
    }
}

/// A runner thread's stack, and the room kept below `stack_limit` (M4-05 section 6).
pub const RUNNER_STACK: usize = 64 << 20;
const MARGIN: usize = 1 << 20;

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
