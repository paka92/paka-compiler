//! The ledger reports every leak and bad free, and a refused allocation is a `memory` trap.

use paka_rt::abi::dev::{Context, Fault, Heap, MEMORY, rt_alloc, rt_release};
use std::ptr;

#[test]
fn a_planted_leak_is_reported() {
    let mut heap = Heap::new(1 << 20);
    let kept = heap.alloc(24).unwrap();
    let freed = heap.alloc(8).unwrap();
    heap.release(freed, 8);
    assert_eq!(heap.leaks(), [(kept.addr(), 24)]);
    assert_eq!(heap.faults(), []);
}

#[test]
fn a_double_free_is_reported() {
    let mut heap = Heap::new(1 << 20);
    let block = heap.alloc(16).unwrap();
    heap.release(block, 16);
    heap.release(block, 16);
    let address = block.addr();
    assert_eq!(heap.faults(), [Fault::NotLive { address }]);
    assert_eq!(heap.leaks(), []);
}

#[test]
fn a_free_of_a_pointer_never_allocated_is_reported() {
    let mut heap = Heap::new(1 << 20);
    let mut word = 0u64;
    let stray = (&raw mut word).cast::<u8>();
    heap.release(stray, 8);
    let address = stray.addr();
    assert_eq!(heap.faults(), [Fault::NotLive { address }]);
}

#[test]
fn a_wrong_size_free_is_reported_and_the_block_freed() {
    let mut heap = Heap::new(32);
    let block = heap.alloc(32).unwrap();
    heap.release(block, 8);
    let address = block.addr();
    let fault = Fault::WrongSize {
        address,
        given: 8,
        allocated: 32,
    };
    assert_eq!(heap.faults(), [fault]);
    assert_eq!(heap.leaks(), []);
    assert!(heap.alloc(32).is_some(), "all 32 bytes came back");
}

#[test]
fn passing_the_cap_is_a_memory_trap() {
    let mut ctx = Context::new(32);
    let mut out = ptr::null_mut();
    assert_eq!(rt_alloc(&mut ctx, 24, &mut out), 0);
    let first = out;
    assert_eq!(rt_alloc(&mut ctx, 16, &mut out), 1);
    assert_eq!((ctx.trap.kind, out), (MEMORY, first), "out is unwritten");
    rt_release(&mut ctx, first, 24);
    assert_eq!(rt_alloc(&mut ctx, 16, &mut out), 0);
}

#[test]
fn a_host_refusal_is_a_memory_trap() {
    let mut ctx = Context::new(u64::MAX);
    let mut out = ptr::null_mut();
    assert_eq!(rt_alloc(&mut ctx, u64::MAX, &mut out), 1);
    assert_eq!(rt_alloc(&mut ctx, 1 << 50, &mut out), 1);
    assert_eq!(ctx.trap.kind, MEMORY);
    assert_eq!(ctx.heap.leaks(), []);
}

#[test]
fn blocks_are_aligned_and_distinct_even_when_empty() {
    let mut heap = Heap::new(1 << 20);
    let blocks: Vec<_> = (0..4).map(|n| heap.alloc(n * 8).unwrap()).collect();
    assert!(blocks.iter().all(|b| b.addr() % 8 == 0));
    let mut addresses: Vec<_> = blocks.iter().map(|b| b.addr()).collect();
    addresses.sort_unstable();
    addresses.dedup();
    assert_eq!(addresses.len(), 4);
}

#[test]
fn a_double_free_of_a_large_block_is_reported() {
    let mut heap = Heap::new(1 << 20);
    let block = heap.alloc(4096).unwrap();
    heap.release(block, 4096);
    heap.release(block, 4096);
    let address = block.addr();
    assert_eq!(heap.faults(), [Fault::NotLive { address }]);
    assert_eq!(heap.leaks(), []);
}

#[test]
fn a_free_inside_a_block_is_reported_and_the_block_kept() {
    let mut heap = Heap::new(1 << 20);
    let block = heap.alloc(64).unwrap();
    let inside = block.wrapping_add(8);
    heap.release(inside, 56);
    let address = inside.addr();
    assert_eq!(heap.faults(), [Fault::NotLive { address }]);
    assert_eq!(heap.leaks(), [(block.addr(), 64)]);
}

#[test]
fn a_sweep_frees_only_the_blocks_made_after_its_mark() {
    let mut heap = Heap::new(1 << 20);
    let kept = [heap.alloc(24).unwrap(), heap.alloc(2048).unwrap()];
    let mark = heap.made();
    let freed = heap.alloc(8).unwrap();
    heap.release(freed, 8);
    for bytes in [24, 2048, 40] {
        heap.alloc(bytes).unwrap();
    }
    heap.sweep(mark);
    let mut live = [(kept[0].addr(), 24), (kept[1].addr(), 2048)];
    live.sort_unstable();
    assert_eq!(heap.leaks(), live, "in address order");
    assert_eq!(heap.faults(), []);
    assert_eq!(heap.made(), mark + 4);
}
