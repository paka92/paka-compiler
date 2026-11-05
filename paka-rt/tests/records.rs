//! A copy owns new blocks with the same contents, and a drop frees every block a value owns.

use paka_rt::abi::dev::records::{Part, Records, Spec, rt_copy, rt_drop};
use paka_rt::abi::dev::words::{ListWords, TextWords};
use paka_rt::abi::dev::{Context, MEMORY};
use std::{ptr, slice};

/// Type 0 a String; 1 `{name: String, tags: List[String]}`; 2 an enum `none | some(String)`.
fn records() -> Records {
    let string = |offset| vec![(offset, Part::String)];
    let person = vec![(0, Part::String), (32, Part::List { item: 0, size: 32 })];
    Records::new(&[
        Some(Spec {
            size: 32,
            tagged: false,
            alternatives: vec![string(0)],
        }),
        Some(Spec {
            size: 56,
            tagged: false,
            alternatives: vec![person],
        }),
        Some(Spec {
            size: 40,
            tagged: true,
            alternatives: vec![vec![], string(8)],
        }),
    ])
}

fn text(ctx: &mut Context, s: &str) -> TextWords {
    let ptr = ctx.heap.alloc(s.len() as u64).unwrap();
    // SAFETY: `ptr` is a new block of `s.len()` bytes.
    unsafe { ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len()) };
    let n = s.len() as u64;
    TextWords {
        ptr,
        bytes: n,
        chars: n,
        cap: n,
    }
}

fn read(t: &TextWords) -> &str {
    // SAFETY: the words are a String made by `text` or a copy of one.
    unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(t.ptr, t.bytes as usize)) }
}

#[repr(C)]
struct Person {
    name: TextWords,
    tags: ListWords,
}

fn person(ctx: &mut Context) -> Person {
    let block = ctx.heap.alloc(64).unwrap().cast::<TextWords>();
    for (i, s) in ["ab", "cde"].into_iter().enumerate() {
        let t = text(ctx, s);
        // SAFETY: the block has room for two Strings.
        unsafe { block.add(i).write(t) };
    }
    let tags = ListWords {
        ptr: block.cast(),
        len: 2,
        cap: 2,
    };
    Person {
        name: text(ctx, "name"),
        tags,
    }
}

fn address<T>(v: &mut T) -> u64 {
    ptr::from_mut(v).expose_provenance() as u64
}

#[test]
fn a_copied_record_owns_new_blocks_and_both_drop_clean() {
    let records = records();
    let mut ctx = Context::new(1 << 20);
    let mut from = person(&mut ctx);
    let mut to = person(&mut ctx);
    // SAFETY: `to`'s own blocks are dropped first, then it gets a copy of `from`.
    unsafe {
        rt_drop(&mut ctx, records.address(1), address(&mut to));
        assert_eq!(
            rt_copy(
                &mut ctx,
                0,
                records.address(1),
                address(&mut from),
                address(&mut to)
            ),
            0
        );
    }
    assert_ne!(from.name.ptr, to.name.ptr);
    assert_ne!(from.tags.ptr, to.tags.ptr);
    assert_eq!(read(&to.name), "name");
    // SAFETY: the copy's List holds two Strings.
    let tags = unsafe { slice::from_raw_parts(to.tags.ptr.cast::<TextWords>(), 2) };
    assert_eq!([read(&tags[0]), read(&tags[1])], ["ab", "cde"]);
    // SAFETY: each value is dropped once.
    unsafe {
        rt_drop(&mut ctx, records.address(1), address(&mut from));
        rt_drop(&mut ctx, records.address(1), address(&mut to));
    }
    assert_eq!(ctx.heap.leaks(), []);
    assert_eq!(ctx.heap.faults(), []);
}

#[test]
fn a_tagged_value_copies_and_drops_by_its_variant() {
    #[repr(C)]
    struct Maybe {
        tag: u64,
        text: TextWords,
    }
    let records = records();
    let mut ctx = Context::new(1 << 20);
    let mut some = Maybe {
        tag: 1,
        text: text(&mut ctx, "x"),
    };
    let mut copy = Maybe {
        tag: 0,
        text: TextWords {
            ptr: ptr::null_mut(),
            bytes: 0,
            chars: 0,
            cap: 0,
        },
    };
    let mut none = Maybe {
        tag: 0,
        text: TextWords {
            ptr: ptr::null_mut(),
            bytes: 9,
            chars: 9,
            cap: 9,
        },
    };
    // SAFETY: each value is of type 2; `none` owns nothing, so its stray words are never read.
    unsafe {
        assert_eq!(
            rt_copy(
                &mut ctx,
                0,
                records.address(2),
                address(&mut some),
                address(&mut copy)
            ),
            0
        );
        assert_eq!(read(&copy.text), "x");
        assert_ne!(copy.text.ptr, some.text.ptr);
        for v in [&mut some, &mut copy, &mut none] {
            rt_drop(&mut ctx, records.address(2), address(v));
        }
    }
    assert_eq!(ctx.heap.leaks(), []);
    assert_eq!(ctx.heap.faults(), []);
}

#[test]
fn a_copy_past_the_cap_traps_memory() {
    let records = records();
    let mut ctx = Context::new(96);
    let mut from = person(&mut ctx);
    let mut to = Person {
        name: TextWords {
            ptr: ptr::null_mut(),
            bytes: 0,
            chars: 0,
            cap: 0,
        },
        tags: ListWords {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
    };
    // SAFETY: `from` is of type 1 and `to` has room for it.
    let status = unsafe {
        rt_copy(
            &mut ctx,
            7,
            records.address(1),
            address(&mut from),
            address(&mut to),
        )
    };
    assert_eq!(status, 1);
    assert_eq!((ctx.trap.kind, ctx.trap.site), (MEMORY, 7));
}
