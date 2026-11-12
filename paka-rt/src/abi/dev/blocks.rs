//! A run's heap (docs/decisions/M4-05.md section 9). Every block is in its ledger, so a leak, a
//! double free or a free of the wrong size is reported instead of corrupting memory, and the run's
//! cap is checked before the host is asked.
//!
//! A block of at most `SMALL` bytes is a slot of a chunk: `CHUNK` bytes on a `CHUNK` boundary,
//! all its slots of one size class. A word before the block holds its number in the order made
//! and its size; it is 0 while the slot is free, and the slot's next word then links the next free
//! slot of its class. A larger block comes from the host and is kept in a table.

use std::alloc::{self, Layout};
use std::hash::{BuildHasherDefault, Hasher};
use std::ptr;

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

const SMALL: u64 = 1024;
/// The low bits of a slot's word, which hold the block's size.
const SIZE_BITS: u32 = 11;
const SIZE_MASK: u64 = (1 << SIZE_BITS) - 1;
const CHUNK: usize = 1 << 20;
/// Class `c` holds blocks of up to `8c` bytes; class 0 is unused.
const CLASSES: usize = SMALL as usize / 8 + 1;

/// Hashes an address by one multiply.
#[derive(Default)]
struct AddressHasher(u64);

impl Hasher for AddressHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("a key is an address");
    }

    fn write_usize(&mut self, address: usize) {
        // The high bits of the product mix every bit of the address; the table reads both ends.
        let mixed = (address as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.0 = mixed ^ (mixed >> 32);
    }
}

/// A table by address. Nothing a program sees depends on its order.
#[allow(clippy::disallowed_types)]
type Table<V> = std::collections::HashMap<usize, V, BuildHasherDefault<AddressHasher>>;

/// Each chunk's class, 0 for none, in a table per span of `SPAN` chunks: a run's chunks lie in
/// few spans, so a lookup is a compare or two and a load.
#[derive(Default)]
struct Chunks(Vec<(usize, Box<[u8; SPAN]>)>);

const SPAN: usize = 1 << 12;
const SPAN_BITS: u32 = CHUNK.trailing_zeros() + SPAN.trailing_zeros();

impl Chunks {
    /// The class of the chunk holding `address`, or 0.
    fn class(&self, address: usize) -> usize {
        let (span, at) = (address >> SPAN_BITS, address / CHUNK % SPAN);
        let found = self.0.iter().find(|(s, _)| *s == span);
        found.map_or(0, |(_, classes)| classes[at].into())
    }

    fn insert(&mut self, base: usize, class: usize) {
        let span = base >> SPAN_BITS;
        let i = match self.0.iter().position(|(s, _)| *s == span) {
            Some(i) => i,
            None => {
                self.0.push((span, Box::new([0; SPAN])));
                self.0.len() - 1
            }
        };
        self.0[i].1[base / CHUNK % SPAN] = class as u8;
    }

    /// Each chunk's base and class.
    fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.0.iter().flat_map(|(span, classes)| {
            let chunks = classes.iter().enumerate().filter(|&(_, &c)| c != 0);
            chunks.map(move |(i, &c)| ((span << SPAN_BITS) + i * CHUNK, c.into()))
        })
    }
}

/// One class's slots: the first free one, and the uncarved rest of its newest chunk.
#[derive(Clone, Copy, Default)]
struct Pool {
    free: usize,
    bump: usize,
    end: usize,
}

/// Where a live block is kept.
enum Kept {
    /// The slot's start, and its class.
    Slot(usize, usize),
    Large,
}

/// A run's blocks and the faults met freeing them. Live blocks are freed when it drops.
pub struct Heap {
    /// The most bytes live at once.
    cap: u64,
    used: u64,
    /// Blocks made so far.
    made: u64,
    /// Blocks live now.
    live: u64,
    pools: [Pool; CLASSES],
    chunks: Chunks,
    /// Each large block's size, and its number in the order made.
    large: Table<(u64, u64)>,
    faults: Vec<Fault>,
}

impl Heap {
    pub fn new(cap: u64) -> Heap {
        Heap {
            cap,
            used: 0,
            made: 0,
            live: 0,
            pools: [Pool::default(); CLASSES],
            chunks: Chunks::default(),
            large: Table::default(),
            faults: vec![],
        }
    }

    /// A new block of `bytes`, aligned 8; `None` past the cap or when the host refuses.
    pub fn alloc(&mut self, bytes: u64) -> Option<*mut u8> {
        let used = self.used.checked_add(bytes).filter(|&u| u <= self.cap)?;
        let n = self.made + 1;
        let address = if bytes <= SMALL {
            let slot = self.carve(class(bytes))?;
            // SAFETY: `carve` gives a free slot, whose words are the heap's.
            unsafe { *word(slot) = n << SIZE_BITS | bytes };
            slot + 8
        } else {
            // SAFETY: the layout's size is at least 1.
            let block = unsafe { alloc::alloc(layout(bytes)?) };
            if block.is_null() {
                return None;
            }
            self.large.insert(block.expose_provenance(), (bytes, n));
            block.addr()
        };
        (self.used, self.made, self.live) = (used, n, self.live + 1);
        Some(ptr::with_exposed_provenance_mut(address))
    }

    /// Frees the block at `ptr`, or records why it cannot. Only memory the ledger holds as a
    /// block is read, so a bad pointer is never touched.
    pub fn release(&mut self, ptr: *mut u8, bytes: u64) {
        let address = ptr.addr();
        let Some((kept, allocated)) = self.find(address) else {
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
        self.forget(kept, address, allocated);
    }

    /// Frees the block at `address` with its own size when it is live; anything else is left.
    pub fn release_live(&mut self, address: usize) {
        if let Some((kept, bytes)) = self.find(address) {
            self.forget(kept, address, bytes);
        }
    }

    /// The blocks still live, as (address, bytes) in address order.
    pub fn leaks(&self) -> Vec<(usize, u64)> {
        if self.live == 0 {
            return vec![];
        }
        let slots = self.slots().map(|(slot, _, w)| (slot + 8, w & SIZE_MASK));
        let large = self.large.iter().map(|(&a, &(bytes, _))| (a, bytes));
        let mut live: Vec<_> = slots.chain(large).collect();
        live.sort_unstable();
        live
    }

    /// Frees every block made after the first `made` and still live: what a trap left.
    pub fn sweep(&mut self, made: u64) {
        let slots = self.slots().filter(|&(_, _, w)| w >> SIZE_BITS > made);
        let slots =
            slots.map(|(slot, class, w)| (Kept::Slot(slot, class), slot + 8, w & SIZE_MASK));
        let large = self.large.iter().filter(|&(_, &(_, n))| n > made);
        let large = large.map(|(&a, &(bytes, _))| (Kept::Large, a, bytes));
        let doomed: Vec<_> = slots.chain(large).collect();
        for (kept, address, bytes) in doomed {
            self.forget(kept, address, bytes);
        }
    }

    /// How many blocks the run has made, freed or not.
    pub fn made(&self) -> u64 {
        self.made
    }

    pub fn faults(&self) -> &[Fault] {
        &self.faults
    }

    /// A free slot of `class`: the first on its free list, else carved from its newest chunk, or
    /// from a new one; `None` when the host refuses a chunk.
    fn carve(&mut self, class: usize) -> Option<usize> {
        let size = slot_size(class);
        let pool = &mut self.pools[class];
        if pool.free != 0 {
            let slot = pool.free;
            // SAFETY: a free slot's second word links the next free slot of its class.
            pool.free = unsafe { *word(slot + 8) } as usize;
            return Some(slot);
        }
        if pool.end - pool.bump < size {
            // SAFETY: `CHUNK` is not 0.
            let chunk = unsafe { alloc::alloc(chunk_layout()) };
            if chunk.is_null() {
                return None;
            }
            let base = chunk.expose_provenance();
            self.chunks.insert(base, class);
            (pool.bump, pool.end) = (base, base + CHUNK);
        }
        pool.bump += size;
        Some(pool.bump - size)
    }

    /// The block at `address` and its size, if it is live.
    fn find(&self, address: usize) -> Option<(Kept, u64)> {
        let (base, class) = (address & !(CHUNK - 1), self.chunks.class(address));
        if class == 0 {
            let &(bytes, _) = self.large.get(&address)?;
            return Some((Kept::Large, bytes));
        }
        let (slot, size) = (address.wrapping_sub(8), slot_size(class));
        let offset = slot.wrapping_sub(base);
        // The quotient by a multiply, exact below `CHUNK`, as a division is slow.
        let on_slot = || ((offset as u64 * RECIPROCALS[class]) >> 32) as usize * size == offset;
        if offset >= self.carved(base, class) - base || !on_slot() {
            return None;
        }
        // SAFETY: the slot is carved from a chunk the heap holds, so its word is the heap's.
        let w = unsafe { *word(slot) };
        (w != 0).then_some((Kept::Slot(slot, class), w & SIZE_MASK))
    }

    /// Frees the live block at `address`, kept as `kept`, of `bytes`.
    fn forget(&mut self, kept: Kept, address: usize, bytes: u64) {
        (self.used, self.live) = (self.used - bytes, self.live - 1);
        match kept {
            Kept::Slot(slot, class) => {
                let pool = &mut self.pools[class];
                // SAFETY: the slot is live, so its words are the heap's; it joins the free list.
                unsafe { (*word(slot), *word(slot + 8)) = (0, pool.free as u64) };
                pool.free = slot;
            }
            Kept::Large => {
                self.large.remove(&address);
                // SAFETY: the table held `address` as a live block of `bytes`, and drops it here.
                unsafe { free(address, bytes) };
            }
        }
    }

    /// The end of the slots carved from the chunk at `base` of `class`.
    fn carved(&self, base: usize, class: usize) -> usize {
        let pool = &self.pools[class];
        match pool.end == base + CHUNK {
            true => pool.bump,
            false => base + CHUNK / slot_size(class) * slot_size(class),
        }
    }

    /// Each live slot: its start, class and word.
    fn slots(&self) -> impl Iterator<Item = (usize, usize, u64)> + '_ {
        self.chunks.iter().flat_map(move |(base, class)| {
            let carved = base..self.carved(base, class);
            // SAFETY: the slots are carved from a chunk the heap holds, so their words are its.
            let slots = carved
                .step_by(slot_size(class))
                .map(move |s| (s, class, unsafe { *word(s) }));
            slots.filter(|&(_, _, w)| w != 0)
        })
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        for (base, _) in self.chunks.iter() {
            // SAFETY: the chunk was allocated with this layout, and nothing uses it after.
            unsafe { alloc::dealloc(ptr::with_exposed_provenance_mut(base), chunk_layout()) };
        }
        for (&address, &(bytes, _)) in &self.large {
            // SAFETY: the table held `address` as a live block of `bytes`.
            unsafe { free(address, bytes) };
        }
    }
}

/// The class of a block of `bytes`, at most `SMALL`.
fn class(bytes: u64) -> usize {
    bytes.div_ceil(8).max(1) as usize
}

/// A slot of `class`: its word, then the block.
const fn slot_size(class: usize) -> usize {
    8 + 8 * class
}

/// Per class, 2^32 over its slot size, rounded up.
const RECIPROCALS: [u64; CLASSES] = {
    let mut r = [0; CLASSES];
    let mut class = 1;
    while class < CLASSES {
        r[class] = (1u64 << 32).div_ceil(slot_size(class) as u64);
        class += 1;
    }
    r
};

fn word(address: usize) -> *mut u64 {
    ptr::with_exposed_provenance_mut(address)
}

fn chunk_layout() -> Layout {
    Layout::from_size_align(CHUNK, CHUNK).expect("a chunk's layout")
}

/// The layout of a large block of `bytes`.
fn layout(bytes: u64) -> Option<Layout> {
    Layout::from_size_align(usize::try_from(bytes).ok()?, super::super::table::ALIGN).ok()
}

/// # Safety
///
/// `address` is a live large block of `bytes` from `Heap::alloc`, which exposed its provenance.
unsafe fn free(address: usize, bytes: u64) {
    let layout = layout(bytes).expect("the layout it was allocated with");
    // SAFETY: `alloc::alloc` made the block with this layout, and the caller vouches it is live.
    unsafe { alloc::dealloc(ptr::with_exposed_provenance_mut(address), layout) };
}
