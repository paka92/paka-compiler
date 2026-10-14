//! The generic built-in rows over List, Map and Option (M2-14 sections 1.8 to 1.10). One routine
//! serves every item type: the checker has already made each use agree on one.

use super::eval::{CAP, Kind, Trap, bound, out_of_bounds};
use super::value::{Items, Key, Value, equals, position};
use crate::core_form::Site;
use paka_rt::ops::Int;
use std::mem;

type Out = Result<Value, Trap>;

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
        ("list_of", [item, n]) => list_of(item, n.int(), site),
        ("get", [Value::Map(m), key]) => (m.0.get(&Key::of(key)))
            .map(|v| copied(v, site))
            .transpose()
            .map(option),
        ("has", [Value::Map(m), key]) => Ok(Value::Bool(m.0.contains_key(&Key::of(key)))),
        ("remove", [Value::Map(m), key]) => {
            m.0.remove(&Key::of(key));
            Ok(Value::Unit)
        }
        ("keys", [Value::Map(m)]) => Ok(list(m.0.keys().map(Key::value).collect())),
        ("values", [Value::Map(m)]) => (m.0.values().map(|v| copied(v, site)))
            .collect::<Result<_, _>>()
            .map(list),
        ("unwrap_or", [Value::Variant(n, inside), fallback]) => Ok(mem::take(if *n == SOME {
            &mut inside.0[0]
        } else {
            fallback
        })),
        ("ok_or", [Value::Variant(n, inside), error]) => Ok(if *n == SOME {
            Value::Variant(OK, mem::take(inside))
        } else {
            Value::Variant(ERR, Items(vec![mem::take(error)]))
        }),
        _ => return None,
    })
}

/// The variant numbers of `Option` and `Result`.
const SOME: usize = 1;
const OK: usize = 0;
const ERR: usize = 1;

fn count(n: usize) -> Value {
    Value::Int(Int::new(n as i64))
}

fn list(items: Vec<Value>) -> Value {
    Value::List(Items(items))
}

fn option(inside: Option<Value>) -> Value {
    match inside {
        Some(v) => Value::Variant(SOME, Items(vec![v])),
        None => Value::Variant(0, Items(vec![])),
    }
}

fn copied(v: &Value, site: Site) -> Out {
    v.copy().ok_or_else(|| Trap::bare(Kind::Stack, site))
}

/// Checks M2-06's cap before a value of `len` items grows by `more`.
fn room(len: usize, more: usize, site: Site) -> Result<(), Trap> {
    match len.saturating_add(more) > CAP {
        true => Err(Trap::bare(Kind::Memory, site)),
        false => Ok(()),
    }
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
