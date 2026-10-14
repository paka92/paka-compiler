//! The generic built-in rows over List, Map and Option (M2-14 sections 1.3 and 1.8 to 1.10), and
//! `sort` (section 1.13). One routine serves every item type: the checker has already made each
//! use agree on one.

use super::eval::{CAP, Kind, Trap, bound, compare, out_of_bounds};
use super::value::{Items, Key, Value, equals, number, position};
use crate::builtins::{ROWS, name};
use crate::core_form::{Cmp, Site, Tag};
use paka_rt::ops::Int;
use std::convert::Infallible;
use std::mem;
use std::sync::OnceLock;

type Out = Result<Value, Trap>;

/// Whether the routine of row `row` moves argument `arg` into its result or a collection. Such an
/// argument must be a value of its own, not a local lent for the call; the other arguments are
/// only read, or copied when they are kept.
pub fn keeps(row: usize, arg: usize) -> bool {
    static KEPT: OnceLock<Vec<u8>> = OnceLock::new();
    let kept = KEPT.get_or_init(|| ROWS.iter().map(|r| kept_by(name(r.1))).collect());
    kept[row] >> arg & 1 == 1
}

/// Whether the routine of row `row` calls a function value, which only the machine can run.
pub fn calls_back(row: usize) -> bool {
    name(ROWS[row].1) == "sort_by"
}

/// A bit per argument.
fn kept_by(row: &str) -> u8 {
    match row {
        "list_of" | "unwrap_or" | "ok_or" => 0b01,
        "append" | "extend" | "insert" | "get" | "has" | "remove" => 0b10,
        _ => 0,
    }
}

/// Runs the generic row `name` on `args`; `None` when no generic row fits them.
pub fn call(name: &str, args: &mut [Value], site: Site) -> Option<Out> {
    Some(match (name, args) {
        ("len", [Value::List(xs)]) => Ok(count(xs.0.len())),
        ("len", [Value::Map(m)]) => Ok(count(m.0.len())),
        ("is_empty", [Value::List(xs)]) => Ok(Value::Bool(xs.0.is_empty())),
        ("is_empty", [Value::Map(m)]) => Ok(Value::Bool(m.0.is_empty())),
        ("contains", [Value::List(xs), item]) => {
            index_of(xs, item, site).map(|i| Value::Bool(i.is_some()))
        }
        ("find", [Value::List(xs), item]) => index_of(xs, item, site).map(|i| option(i.map(count))),
        ("append", [Value::List(xs), item]) => room(xs.0.len(), 1, site).map(|()| {
            xs.0.push(mem::take(item));
            Value::Unit
        }),
        ("extend", [Value::List(xs), Value::List(more)]) => room(xs.0.len(), more.0.len(), site)
            .map(|()| {
                xs.0.append(&mut more.0);
                Value::Unit
            }),
        ("pop", [Value::List(xs)]) => {
            (xs.0.pop()).ok_or_else(|| out_of_bounds(site, "pop on an empty List".into()))
        }
        ("insert", [Value::List(xs), item, at]) => insert(xs, item, at.int(), site),
        ("remove_at", [Value::List(xs), i]) => {
            bound(i.int(), xs.0.len(), site).map(|i| xs.0.remove(i))
        }
        ("swap", [Value::List(xs), i, j]) => swap(xs, i.int(), j.int(), site),
        ("reverse", [Value::List(xs)]) => {
            xs.0.reverse();
            Ok(Value::Unit)
        }
        ("sort", [Value::List(xs)]) => {
            let mut scratch = Vec::with_capacity(xs.0.len());
            let mut less = |a: &mut Value, b: &mut Value| Ok(compare(&Cmp::Lt, a, b));
            merge_sort::<Infallible>(&mut xs.0, &mut scratch, &mut less)
                .unwrap_or_else(|e| match e {});
            Ok(Value::Unit)
        }
        ("list_of", [item, n]) => list_of(item, n.int(), site),
        ("get", [Value::Map(m), key]) => (m.0.get(&mem::take(key).key()))
            .map(|v| copied(v, site))
            .transpose()
            .map(option),
        ("has", [Value::Map(m), key]) => Ok(Value::Bool(m.0.contains_key(&mem::take(key).key()))),
        ("remove", [Value::Map(m), key]) => {
            m.0.remove(&mem::take(key).key());
            Ok(Value::Unit)
        }
        ("keys", [Value::Map(m)]) => Ok(list(m.0.keys().map(Key::value).collect())),
        ("values", [Value::Map(m)]) => (m.0.values().map(|v| copied(v, site)))
            .collect::<Result<_, _>>()
            .map(list),
        ("unwrap_or", [Value::Variant(n, inside), fallback]) => match *n == number(&Tag::Some) {
            true => Ok(mem::take(&mut inside.0[0])),
            false => copied(fallback, site),
        },
        ("ok_or", [Value::Variant(n, inside), error]) => match *n == number(&Tag::Some) {
            true => Ok(Value::Variant(number(&Tag::Ok), mem::take(inside))),
            false => copied(error, site).map(|e| Value::Variant(number(&Tag::Err), Items(vec![e]))),
        },
        _ => return None,
    })
}

fn count(n: usize) -> Value {
    Value::Int(Int::new(n as i64))
}

fn list(items: Vec<Value>) -> Value {
    Value::List(Items(items))
}

fn option(inside: Option<Value>) -> Value {
    match inside {
        Some(v) => Value::Variant(number(&Tag::Some), Items(vec![v])),
        None => Value::Variant(number(&Tag::None), Items(vec![])),
    }
}

fn copied(v: &Value, site: Site) -> Out {
    v.copy().ok_or_else(|| Trap::bare(Kind::Stack, site))
}

/// Checks M2-06's cap before a value of `len` items grows by `more`.
fn room(len: usize, more: usize, site: Site) -> Result<(), Trap> {
    if len.saturating_add(more) > CAP {
        return Err(Trap::bare(Kind::Memory, site));
    }
    Ok(())
}

fn index_of(xs: &Items, item: &Value, site: Site) -> Result<Option<usize>, Trap> {
    for (i, x) in xs.0.iter().enumerate() {
        if equals(x, item).ok_or_else(|| Trap::bare(Kind::Stack, site))? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

fn insert(xs: &mut Items, item: &mut Value, at: Int, site: Site) -> Out {
    let len = xs.0.len();
    let i = position(at).filter(|i| *i <= len);
    let i = i.ok_or_else(|| out_of_bounds(site, format!("index {at}, length {len}")))?;
    room(len, 1, site)?;
    xs.0.insert(i, mem::take(item));
    Ok(Value::Unit)
}

fn swap(xs: &mut Items, i: Int, j: Int, site: Site) -> Out {
    let len = xs.0.len();
    let (i, j) = (bound(i, len, site)?, bound(j, len, site)?);
    xs.0.swap(i, j);
    Ok(Value::Unit)
}

fn list_of(item: &mut Value, n: Int, site: Site) -> Out {
    let times = position(n).ok_or_else(|| out_of_bounds(site, format!("count {n}")))?;
    room(0, times, site)?;
    let item = mem::take(item);
    let mut items = Vec::with_capacity(times);
    for _ in 1..times {
        items.push(copied(&item, site)?);
    }
    if times > 0 {
        items.push(item);
    }
    Ok(list(items))
}

/// M2-14 section 1.13's procedure. `before(right, left)` says whether the right item must come
/// first; the left one is taken on `false`. `scratch` is the range merged into.
pub fn merge_sort<E>(
    xs: &mut [Value],
    scratch: &mut Vec<Value>,
    before: &mut impl FnMut(&mut Value, &mut Value) -> Result<bool, E>,
) -> Result<(), E> {
    if xs.len() < 2 {
        return Ok(());
    }
    let (left, right) = xs.split_at_mut(xs.len() / 2);
    merge_sort(left, scratch, before)?;
    merge_sort(right, scratch, before)?;
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        let (side, at) = match before(&mut right[j], &mut left[i])? {
            true => (&mut *right, &mut j),
            false => (&mut *left, &mut i),
        };
        scratch.push(mem::take(&mut side[*at]));
        *at += 1;
    }
    scratch.extend(left[i..].iter_mut().map(mem::take));
    scratch.extend(right[j..].iter_mut().map(mem::take));
    for (slot, item) in xs.iter_mut().zip(scratch.drain(..)) {
        *slot = item;
    }
    Ok(())
}
