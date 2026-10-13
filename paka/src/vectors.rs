//! Runs the shared vectors (tests/vectors/ops.vec) through the operations the interpreter calls.

use paka_rt::ops::vectors::{self, Outcome, Value};
use paka_rt::ops::{Float, Int, Text, Trap};
use std::collections::BTreeSet;

const VECTORS: &str = include_str!("../../tests/vectors/ops.vec");
const ROWS: &str = include_str!("../../tests/vectors/rows.txt");

/// An input read out of a vector.
trait Arg: Sized {
    fn read(v: &Value) -> Option<Self>;
}

/// A result turned back into a vector value.
trait Ret {
    fn out(self) -> Outcome;
}

impl Arg for Int {
    fn read(v: &Value) -> Option<Int> {
        let Value::Int(n) = v else { return None };
        Some(Int::new(*n))
    }
}

impl Arg for Float {
    fn read(v: &Value) -> Option<Float> {
        let Value::Float(bits) = v else { return None };
        Some(Float::from_bits(*bits))
    }
}

impl Arg for bool {
    fn read(v: &Value) -> Option<bool> {
        let Value::Bool(b) = v else { return None };
        Some(*b)
    }
}

impl Arg for Text {
    fn read(v: &Value) -> Option<Text> {
        let Value::Str(s) = v else { return None };
        Some(Text::from(s.as_str()))
    }
}

impl Ret for Int {
    fn out(self) -> Outcome {
        Ok(Value::Int(self.get()))
    }
}

impl Ret for Float {
    fn out(self) -> Outcome {
        Ok(Value::Float(self.bits()))
    }
}

impl Ret for bool {
    fn out(self) -> Outcome {
        Ok(Value::Bool(self))
    }
}

impl Ret for Text {
    fn out(self) -> Outcome {
        Ok(Value::Str(self.as_str().into()))
    }
}

impl Ret for String {
    fn out(self) -> Outcome {
        Ok(Value::Str(self))
    }
}

impl<R: Ret> Ret for Result<R, Trap> {
    fn out(self) -> Outcome {
        self.and_then(Ret::out)
    }
}

impl<R: Ret> Ret for Option<R> {
    fn out(self) -> Outcome {
        self.map_or(Ok(Value::None), |r| {
            r.out().map(|v| Value::Some(Box::new(v)))
        })
    }
}

fn arg<A: Arg>(v: &Value) -> A {
    A::read(v).unwrap_or_else(|| panic!("input {v:?} has the wrong type"))
}

fn call1<A: Arg, R: Ret>(args: &[Value], f: impl Fn(A) -> R) -> Outcome {
    let [a] = args else { panic!("want 1 input") };
    f(arg(a)).out()
}

fn call2<A: Arg, B: Arg, R: Ret>(args: &[Value], f: impl Fn(A, B) -> R) -> Outcome {
    let [a, b] = args else {
        panic!("want 2 inputs")
    };
    f(arg(a), arg(b)).out()
}

fn call3<A: Arg, B: Arg, C: Arg, R: Ret>(args: &[Value], f: impl Fn(A, B, C) -> R) -> Outcome {
    let [a, b, c] = args else {
        panic!("want 3 inputs")
    };
    f(arg(a), arg(b), arg(c)).out()
}

fn compare<T: Arg + PartialOrd>(args: &[Value], f: fn(&T, &T) -> bool) -> Outcome {
    call2(args, |a: T, b: T| f(&a, &b))
}

fn run(op: &str, a: &[Value]) -> Outcome {
    match op {
        "int.add" => call2(a, Int::add),
        "int.sub" => call2(a, Int::sub),
        "int.mul" => call2(a, Int::mul),
        "int.neg" => call1(a, Int::neg),
        "int.div" => call2(a, Int::div),
        "int.rem" => call2(a, Int::rem),
        "int.abs" => call1(a, Int::abs),
        "int.min" => call2(a, Int::min),
        "int.max" => call2(a, Int::max),
        "int.eq" => compare::<Int>(a, PartialEq::eq),
        "int.ne" => compare::<Int>(a, PartialEq::ne),
        "int.lt" => compare::<Int>(a, PartialOrd::lt),
        "int.le" => compare::<Int>(a, PartialOrd::le),
        "int.gt" => compare::<Int>(a, PartialOrd::gt),
        "int.ge" => compare::<Int>(a, PartialOrd::ge),
        "int.wrap_add" => call2(a, Int::wrap_add),
        "int.wrap_sub" => call2(a, Int::wrap_sub),
        "int.wrap_mul" => call2(a, Int::wrap_mul),
        "int.bit_and" => call2(a, Int::bit_and),
        "int.bit_or" => call2(a, Int::bit_or),
        "int.bit_xor" => call2(a, Int::bit_xor),
        "int.bit_not" => call1(a, Int::bit_not),
        "int.shift_left" => call2(a, Int::shift_left),
        "int.shift_right" => call2(a, Int::shift_right),
        "int.text" => call1(a, |n: Int| n.to_string()),
        "float.add" => call2(a, Float::add),
        "float.sub" => call2(a, Float::sub),
        "float.mul" => call2(a, Float::mul),
        "float.div" => call2(a, Float::div),
        "float.neg" => call1(a, Float::neg),
        "float.abs" => call1(a, Float::abs),
        "float.sqrt" => call1(a, Float::sqrt),
        "float.floor" => call1(a, Float::floor),
        "float.ceil" => call1(a, Float::ceil),
        "float.trunc" => call1(a, Float::trunc),
        "float.round" => call1(a, Float::round),
        "float.min" => call2(a, Float::min),
        "float.max" => call2(a, Float::max),
        "float.eq" => compare::<Float>(a, PartialEq::eq),
        "float.ne" => compare::<Float>(a, PartialEq::ne),
        "float.lt" => compare::<Float>(a, PartialOrd::lt),
        "float.le" => compare::<Float>(a, PartialOrd::le),
        "float.gt" => compare::<Float>(a, PartialOrd::gt),
        "float.ge" => compare::<Float>(a, PartialOrd::ge),
        "to_float" => call1(a, Int::to_float),
        "to_int" => call1(a, Float::to_int),
        "parse_int" => call1(a, |t: Text| Int::parse(t.as_str())),
        "parse_float" => call1(a, |t: Text| Float::parse(t.as_str())),
        "fixed" => call2(a, Float::fixed),
        "string.len" => call1(a, |t: Text| t.len()),
        "string.index" => call2(a, |t: Text, i: Int| t.index(i)),
        "string.slice" => call3(a, |t: Text, from: Int, to: Int| t.slice(from, to)),
        "string.eq" => compare::<Text>(a, PartialEq::eq),
        "string.ne" => compare::<Text>(a, PartialEq::ne),
        "string.lt" => compare::<Text>(a, PartialOrd::lt),
        "string.le" => compare::<Text>(a, PartialOrd::le),
        "string.gt" => compare::<Text>(a, PartialOrd::gt),
        "string.ge" => compare::<Text>(a, PartialOrd::ge),
        "string.add" => call2(a, |x: Text, y: Text| x.add(&y)),
        "string.min" => call2(a, Text::min),
        "string.max" => call2(a, Text::max),
        "string.text" => call1(a, |t: Text| t),
        "code_at" => call2(a, |t: Text, i: Int| t.code_at(i)),
        "from_code" => call1(a, Text::from_code),
        "bool.not" => call1(a, |b: bool| !b),
        "bool.and" => call2(a, |x: bool, y: bool| x && y),
        "bool.or" => call2(a, |x: bool, y: bool| x || y),
        "bool.eq" => compare::<bool>(a, PartialEq::eq),
        "bool.ne" => compare::<bool>(a, PartialEq::ne),
        "bool.text" => call1(a, |b: bool| b.to_string()),
        _ => panic!("the interpreter has no operation {op}"),
    }
}

#[test]
fn every_vector_passes() {
    let all = vectors::parse(VECTORS).unwrap();
    let wrong = vectors::failures(&all, run);
    assert!(
        wrong.is_empty(),
        "{} failed:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_row_has_a_vector() {
    let used: BTreeSet<_> = vectors::parse(VECTORS)
        .unwrap()
        .into_iter()
        .map(|v| v.op)
        .collect();
    let bare: Vec<_> = vectors::rows(ROWS)
        .into_iter()
        .filter(|r| !used.contains(*r))
        .collect();
    assert!(bare.is_empty(), "rows without a vector: {bare:?}");
}

#[test]
fn every_vector_is_for_a_row() {
    let rows: BTreeSet<_> = vectors::rows(ROWS).into_iter().collect();
    let stray: Vec<_> = vectors::parse(VECTORS)
        .unwrap()
        .into_iter()
        .filter(|v| !rows.contains(v.op.as_str()))
        .map(|v| format!("line {}: {}", v.line, v.op))
        .collect();
    assert!(stray.is_empty(), "vectors for no row: {stray:?}");
}
