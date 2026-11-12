//! The number built-ins (M2-14 section 1.4): each is one operation of `paka_rt::ops`.

use super::eval::Trap;
use super::value::Value;
use crate::core_form::Site;
use paka_rt::ops::{Int, Text};

type Out = Result<Value, Trap>;

/// Runs the number row `name` on `args`; `None` when no number row fits them.
pub fn call(name: &str, args: &[Value], site: Site) -> Option<Out> {
    Some(match (name, args) {
        ("min", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(Int::min(*a, *b))),
        ("min", [Value::Float(a), Value::Float(b)]) => Ok(Value::Float(a.min(*b))),
        ("min", [Value::Str(a), Value::Str(b)]) => Ok(Value::Str(Ord::min(a, b).clone())),
        ("max", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(Int::max(*a, *b))),
        ("max", [Value::Float(a), Value::Float(b)]) => Ok(Value::Float(a.max(*b))),
        ("max", [Value::Str(a), Value::Str(b)]) => Ok(Value::Str(Ord::max(a, b).clone())),
        ("abs", [Value::Int(n)]) => {
            (n.abs().map(Value::Int)).map_err(|k| Trap::new(k, site, format!("abs({n})")))
        }
        ("abs", [Value::Float(x)]) => Ok(Value::Float(x.abs())),
        ("to_float", [Value::Int(n)]) => Ok(Value::Float(n.to_float())),
        ("to_int", [Value::Float(x)]) => (x.to_int().map(Value::Int))
            .map_err(|k| Trap::new(k, site, format!("to_int({})", x.show()))),
        ("sqrt", [Value::Float(x)]) => Ok(Value::Float(x.sqrt())),
        ("floor", [Value::Float(x)]) => Ok(Value::Float(x.floor())),
        ("ceil", [Value::Float(x)]) => Ok(Value::Float(x.ceil())),
        ("trunc", [Value::Float(x)]) => Ok(Value::Float(x.trunc())),
        ("round", [Value::Float(x)]) => Ok(Value::Float(x.round())),
        ("fixed", [Value::Float(x), digits]) => (x.fixed(digits.int()))
            .map(|text| Value::Str(Text::from(text)))
            .map_err(|k| Trap::new(k, site, format!("digits {}", digits.int()))),
        ("wrap_add", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.wrap_add(*b))),
        ("wrap_sub", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.wrap_sub(*b))),
        ("wrap_mul", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.wrap_mul(*b))),
        ("bit_and", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.bit_and(*b))),
        ("bit_or", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.bit_or(*b))),
        ("bit_xor", [Value::Int(a), Value::Int(b)]) => Ok(Value::Int(a.bit_xor(*b))),
        ("bit_not", [Value::Int(a)]) => Ok(Value::Int(a.bit_not())),
        ("shift_left", [Value::Int(a), Value::Int(count)]) => (a.shift_left(*count))
            .map(Value::Int)
            .map_err(|k| Trap::new(k, site, format!("shift {count}"))),
        ("shift_right", [Value::Int(a), Value::Int(count)]) => (a.shift_right(*count))
            .map(Value::Int)
            .map_err(|k| Trap::new(k, site, format!("shift {count}"))),
        _ => return None,
    })
}
