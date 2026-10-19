//! The dev runtime's context and allocator (M4-25, docs/decisions/M4-05.md sections 4 and 9).
//! Every block of a run is in a ledger, so a leak, a double free or a free of the wrong size is reported instead of
//! corrupting memory, and the run's cap is checked before the host is asked.

use std::alloc::{self, Layout};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;

/// The `memory` trap kind (M4-05 section 6).
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
    pub rt: *const usize,
    pub stack_limit: usize,
    pub interrupt: AtomicU64,
    /// The trap kind; 0 when none.
    pub trap: u64,
    pub heap: Heap,
}

/// `rt.alloc`: a block of `bytes` into `out` and status 0, or a `memory` trap and status 1.
pub extern "sysv64" fn rt_alloc(ctx: &mut Context, bytes: u64, out: &mut *mut u8) -> i32 {
    match ctx.heap.alloc(bytes) {
        Some(ptr) => {
            *out = ptr;
            0
        }
        None => {
            ctx.trap = MEMORY;
            1
        }
    }
}

/// `rt.release`: frees a block; never traps.
pub extern "sysv64" fn rt_release(ctx: &mut Context, ptr: *mut u8, bytes: u64) {
    ctx.heap.release(ptr, bytes);
}
