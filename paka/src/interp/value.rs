//! The values the interpreter holds: owned trees, never shared.

use crate::core_form::{EnumId, Tag};
use paka_rt::ops::{Float, Int, Text};
use std::collections::BTreeMap;
use std::mem;

/// A copy or a compare reaching deeper than this stops with `stack` (docs/core.md section 9).
const NEST: usize = 10_000;

#[derive(Debug, Default, PartialEq)]
pub enum Value {
    /// No value, or a slot whose value is out on loan to a call.
    #[default]
    Unit,
    Int(Int),
    Float(Float),
    Bool(bool),
    Str(Text),
    List(Items),
    Map(Entries),
    Record(Items),
    /// The variant number and its fields.
    Variant(usize, Items),
    Fn(FnKey),
}

/// A function value: a user function, or the constructor of a user variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FnKey {
    Fn(usize),
    Ctor(EnumId, usize),
}

/// A map key: an Int or a String.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Int(Int),
    Str(Text),
}

impl Key {
    /// The key in its value text form.
    pub fn show(&self) -> String {
        match self {
            Key::Int(n) => n.to_string(),
            Key::Str(t) => quoted(t.as_str()),
        }
    }
}

/// Dropping a deep value must not recurse: both containers hand their children to a worklist.
#[derive(Debug, Default, PartialEq)]
pub struct Items(pub Vec<Value>);

#[derive(Debug, Default, PartialEq)]
pub struct Entries(pub BTreeMap<Key, Value>);

impl Drop for Items {
    fn drop(&mut self) {
        dismantle(mem::take(&mut self.0));
    }
}

impl Drop for Entries {
    fn drop(&mut self) {
        dismantle(mem::take(&mut self.0).into_values().collect());
    }
}

fn dismantle(mut work: Vec<Value>) {
    while let Some(value) = work.pop() {
        match value {
            Value::List(mut i) | Value::Record(mut i) | Value::Variant(_, mut i) => {
                work.append(&mut i.0)
            }
            Value::Map(mut e) => work.extend(mem::take(&mut e.0).into_values()),
            _ => {}
        }
    }
}

impl Value {
    pub fn int(&self) -> Int {
        match self {
            Value::Int(n) => *n,
            _ => unreachable!("a checked Int"),
        }
    }

    pub fn float(&self) -> Float {
        match self {
            Value::Float(x) => *x,
            _ => unreachable!("a checked Float"),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => unreachable!("a checked Bool"),
        }
    }

    pub fn key(self) -> Key {
        match self {
            Value::Int(n) => Key::Int(n),
            Value::Str(t) => Key::Str(t),
            _ => unreachable!("a checked key"),
        }
    }

    /// A deep copy, or `None` when it nests too deep.
    pub fn copy(&self) -> Option<Value> {
        self.copy_within(NEST)
    }

    fn copy_within(&self, budget: usize) -> Option<Value> {
        Some(match self {
            Value::Unit => Value::Unit,
            Value::Int(n) => Value::Int(*n),
            Value::Float(x) => Value::Float(*x),
            Value::Bool(b) => Value::Bool(*b),
            Value::Str(t) => Value::Str(t.clone()),
            Value::Fn(f) => Value::Fn(*f),
            Value::List(i) => Value::List(i.copy_within(budget)?),
            Value::Record(i) => Value::Record(i.copy_within(budget)?),
            Value::Variant(n, i) => Value::Variant(*n, i.copy_within(budget)?),
            Value::Map(e) => {
                let budget = budget.checked_sub(1)?;
                let copy = |(k, v): (&Key, &Value)| Some((k.clone(), v.copy_within(budget)?));
                Value::Map(Entries(e.0.iter().map(copy).collect::<Option<_>>()?))
            }
        })
    }
}

impl Items {
    fn copy_within(&self, budget: usize) -> Option<Items> {
        let budget = budget.checked_sub(1)?;
        let copies = self.0.iter().map(|v| v.copy_within(budget));
        Some(Items(copies.collect::<Option<_>>()?))
    }
}

/// Equality by structure, Float by IEEE; `None` when the values nest too deep.
pub fn equals(a: &Value, b: &Value) -> Option<bool> {
    equal_within(a, b, NEST)
}

fn equal_within(a: &Value, b: &Value, budget: usize) -> Option<bool> {
    Some(match (a, b) {
        (Value::Unit, Value::Unit) => true,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Fn(x), Value::Fn(y)) => x == y,
        (Value::List(x), Value::List(y)) | (Value::Record(x), Value::Record(y)) => {
            same_items(x, y, budget)?
        }
        (Value::Variant(m, x), Value::Variant(n, y)) => m == n && same_items(x, y, budget)?,
        (Value::Map(x), Value::Map(y)) => {
            let budget = budget.checked_sub(1)?;
            if x.0.len() != y.0.len() {
                return Some(false);
            }
            for ((k, p), (l, q)) in x.0.iter().zip(&y.0) {
                if k != l || !equal_within(p, q, budget)? {
                    return Some(false);
                }
            }
            true
        }
        _ => false,
    })
}

fn same_items(x: &Items, y: &Items, budget: usize) -> Option<bool> {
    let budget = budget.checked_sub(1)?;
    if x.0.len() != y.0.len() {
        return Some(false);
    }
    for (p, q) in x.0.iter().zip(&y.0) {
        if !equal_within(p, q, budget)? {
            return Some(false);
        }
    }
    Some(true)
}

/// The variant number of a tag: its position among its enum's variants.
pub fn number(tag: &Tag) -> usize {
    match tag {
        Tag::User(_, n) => *n,
        Tag::None | Tag::Ok => 0,
        Tag::Some | Tag::Err => 1,
        Tag::Effect(e) => *e as usize,
    }
}

/// The index `i` as a position, if it is not negative.
#[allow(clippy::disallowed_methods)]
pub fn position(i: Int) -> Option<usize> {
    usize::try_from(i.get()).ok()
}

/// A String in its value text form: quoted, with the escapes of the language.
pub fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' | '"' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out + "\""
}
