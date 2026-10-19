//! The ledger reports every leak and bad free, and a refused allocation is a `memory` trap.

use paka_rt::abi::dev::{Context, Fault, Heap, MEMORY, rt_alloc, rt_release};
use std::ptr;

fn context(cap: u64) -> Context {
    Context {
        slots: ptr::null(),
        rt: ptr::null(),
        stack_limit: 0,
        interrupt: Default::default(),
        trap: 0,
        heap: Heap::new(cap),
    }
}

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
    let mut ctx = context(32);
    let mut out = ptr::null_mut();
    assert_eq!(rt_alloc(&mut ctx, 24, &mut out), 0);
    let first = out;
    assert_eq!(rt_alloc(&mut ctx, 16, &mut out), 1);
    assert_eq!((ctx.trap, out), (MEMORY, first), "out is unwritten");
    rt_release(&mut ctx, first, 24);
    assert_eq!(rt_alloc(&mut ctx, 16, &mut out), 0);
}

#[test]
fn a_host_refusal_is_a_memory_trap() {
    let mut ctx = context(u64::MAX);
    let mut out = ptr::null_mut();
    assert_eq!(rt_alloc(&mut ctx, u64::MAX, &mut out), 1);
    assert_eq!(rt_alloc(&mut ctx, 1 << 50, &mut out), 1);
    assert_eq!(ctx.trap, MEMORY);
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
