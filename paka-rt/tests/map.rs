//! A Map shows its entries in rising key order and finds each one through many puts, removes,
//! grows and reorders, and a copy of it works on its own. Keys span both short ones, told apart by
//! their check alone, and long ones, compared in full.

use paka_rt::abi::dev::map::{AT, FIND, KEYS, PUT, REMOVE, STRING_KEY, VALUES, rt_map};
use paka_rt::abi::dev::records::{Part, Records, Spec, rt_copy, rt_drop};
use paka_rt::abi::dev::words::{ListWords, TextWords};
use paka_rt::abi::dev::{Context, OUT_OF_BOUNDS};
use std::collections::BTreeMap;
use std::{ptr, slice};

/// Type 0 a String; 1 an Int; 2 `List[Int]`; 3 `List[String]`; 4 `Map[Int, Int]`;
/// 5 `Map[String, Int]`.
fn records() -> Records {
    let one = |size, part| {
        Some(Spec {
            size,
            tagged: false,
            alternatives: vec![vec![(0, part)]],
        })
    };
    Records::new(&[
        one(32, Part::String),
        None,
        one(24, Part::List { item: 1, size: 8 }),
        one(24, Part::List { item: 0, size: 32 }),
        one(8, Part::Map { value: 1, shape: 8 }),
        one(
            8,
            Part::Map {
                value: 1,
                shape: STRING_KEY | 8,
            },
        ),
    ])
}

/// A key type: how a key is passed to `rt_map` and read back from a List of keys.
trait Key: Ord + Clone + std::fmt::Debug {
    const SHAPE: u64;
    const MAP: usize;
    const LIST: usize;
    fn new(n: i64) -> Self;
    fn with<R>(&self, f: impl FnOnce(u64) -> R) -> R;
    fn read(list: &ListWords) -> Vec<Self>;
}

impl Key for i64 {
    const SHAPE: u64 = 8;
    const MAP: usize = 4;
    const LIST: usize = 2;
    fn new(n: i64) -> i64 {
        (n * 7 - 1000) << (n.rem_euclid(3) * 20)
    }
    fn with<R>(&self, f: impl FnOnce(u64) -> R) -> R {
        f(*self as u64)
    }
    fn read(list: &ListWords) -> Vec<i64> {
        words(list).to_vec()
    }
}

impl Key for String {
    const SHAPE: u64 = STRING_KEY | 8;
    const MAP: usize = 5;
    const LIST: usize = 3;
    fn new(n: i64) -> String {
        "k".repeat(n as usize % 13) + &n.to_string()
    }
    fn with<R>(&self, f: impl FnOnce(u64) -> R) -> R {
        let n = self.len() as u64;
        let mut words = TextWords {
            ptr: self.as_ptr().cast_mut(),
            bytes: n,
            chars: n,
            cap: 0,
        };
        f(address(&mut words))
    }
    fn read(list: &ListWords) -> Vec<String> {
        let texts: &[TextWords] = words(list);
        // SAFETY: each is a String's words, its bytes UTF-8.
        let read = |t: &TextWords| unsafe { slice::from_raw_parts(t.ptr, t.bytes as usize) };
        texts
            .iter()
            .map(|t| String::from_utf8(read(t).to_vec()).unwrap())
            .collect()
    }
}

fn words<T>(list: &ListWords) -> &[T] {
    match list.len {
        0 => &[],
        // SAFETY: the List holds `len` items of `T`.
        n => unsafe { slice::from_raw_parts(list.ptr.cast(), n as usize) },
    }
}

fn address<T>(v: &mut T) -> u64 {
    ptr::from_mut(v).expose_provenance() as u64
}

/// A Map of one key type on one context.
struct Map<'a, K> {
    ctx: &'a mut Context,
    records: &'a Records,
    block: *mut u8,
    key: std::marker::PhantomData<K>,
}

impl<K: Key> Map<'_, K> {
    fn row(&mut self, row: u64, key: &K) -> (i32, u64) {
        let mut out = 0u64;
        let map = &raw mut self.block;
        let ctx = &mut *self.ctx;
        // SAFETY: the block is a Map of `K::SHAPE`, the key of its type, and `out` a word.
        let status = key
            .with(|k| unsafe { rt_map(ctx, row, 0, 0, K::SHAPE, map, k, (&raw mut out).cast()) });
        (status, out)
    }

    fn put(&mut self, key: &K, value: u64) {
        let (status, at) = self.row(PUT, key);
        assert_eq!(status, 0);
        // SAFETY: PUT gave the address of the entry's value word.
        unsafe { ptr::with_exposed_provenance_mut::<u64>(at as usize).write(value) };
    }

    fn get(&mut self, key: &K) -> Option<u64> {
        let (status, at) = self.row(FIND, key);
        assert_eq!(status, 0);
        // SAFETY: a found entry's value word is at `at`.
        (at != 0).then(|| unsafe { *ptr::with_exposed_provenance::<u64>(at as usize) })
    }

    /// The keys and the values, each as a new List the context then drops.
    fn entries(&mut self) -> (Vec<K>, Vec<u64>) {
        let mut lists = [K::LIST, 2].map(|t| {
            (
                t,
                ListWords {
                    ptr: ptr::null_mut(),
                    len: 0,
                    cap: 0,
                },
            )
        });
        for ((_, list), row) in lists.iter_mut().zip([KEYS, VALUES]) {
            let map = &raw mut self.block;
            // SAFETY: the block is a Map of `K::SHAPE`, and `list` has room for a List.
            let status = unsafe {
                rt_map(
                    self.ctx,
                    row,
                    0,
                    0,
                    K::SHAPE,
                    map,
                    0,
                    ptr::from_mut(list).cast(),
                )
            };
            assert_eq!(status, 0);
        }
        let read = (K::read(&lists[0].1), words(&lists[1].1).to_vec());
        for (t, mut list) in lists {
            // SAFETY: each List is of its type and dropped once.
            unsafe { rt_drop(self.ctx, self.records.address(t), address(&mut list)) };
        }
        read
    }

    fn check(&mut self, model: &BTreeMap<K, u64>) {
        let (keys, values) = self.entries();
        assert_eq!(keys, model.keys().cloned().collect::<Vec<_>>());
        assert_eq!(values, model.values().copied().collect::<Vec<_>>());
        for (k, &v) in model {
            assert_eq!(self.get(k), Some(v));
        }
    }

    fn drop_map(&mut self) {
        // SAFETY: the block is a Map of its type, dropped once.
        unsafe {
            rt_drop(
                self.ctx,
                self.records.address(K::MAP),
                address(&mut self.block),
            )
        };
        self.block = ptr::null_mut();
    }
}

/// Many puts, finds and removes on keys that come and go, against a model; then a copy.
fn churn<K: Key>() {
    let records = records();
    let mut ctx = Context::new(1 << 26);
    let mut map = Map::<K> {
        ctx: &mut ctx,
        records: &records,
        block: ptr::null_mut(),
        key: std::marker::PhantomData,
    };
    let mut model = BTreeMap::new();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for step in 0..30_000u64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if step == 15_000 {
            for k in std::mem::take(&mut model).keys() {
                assert_eq!(map.row(REMOVE, k).0, 0);
            }
            map.check(&model);
        }
        // A key range that widens, then narrows, so lookups meet new keys and old ones.
        let range = 1 + (step % 15_000).min(15_000 - step % 15_000) / 10;
        let key = K::new((seed % range) as i64);
        match seed >> 60 {
            0..=8 => {
                map.put(&key, step);
                model.insert(key, step);
            }
            9..=13 => {
                assert_eq!(map.row(REMOVE, &key).0, 0);
                model.remove(&key);
            }
            _ => assert_eq!(map.get(&key), model.get(&key).copied()),
        }
        if step % 1000 == 0 {
            map.check(&model);
        }
    }
    map.check(&model);
    let missing = K::new(-1);
    assert_eq!(map.row(AT, &missing).0, 1);
    assert_eq!(map.ctx.trap.kind, OUT_OF_BOUNDS);

    let mut from = map.block;
    map.block = ptr::null_mut();
    // SAFETY: both are Maps of one type; the copy's word is empty.
    let status = unsafe {
        rt_copy(
            map.ctx,
            0,
            records.address(K::MAP),
            address(&mut from),
            address(&mut map.block),
        )
    };
    assert_eq!(status, 0);
    map.check(&model);
    for k in model.keys().step_by(2) {
        assert_eq!(map.row(REMOVE, k).0, 0);
    }
    map.put(&missing, 7);
    assert_eq!(map.get(&missing), Some(7));
    map.drop_map();
    map.block = from;
    map.check(&model);
    map.drop_map();
    assert_eq!(ctx.heap.leaks(), []);
    assert_eq!(ctx.heap.faults(), []);
}

#[test]
fn an_int_keyed_map_keeps_order_through_churn() {
    churn::<i64>();
}

#[test]
fn a_string_keyed_map_keeps_order_through_churn() {
    churn::<String>();
}
