//! The String built-ins (M2-14 sections 1.3 and 1.5 to 1.7, rules in section 1.14): positions
//! count code points, matching is exact, white space is the six characters 9 to 13 and 32, and
//! case changes touch `a` to `z` only. A row that grows a String checks M2-06's cap first.

use super::eval::{Trap, out_of_bounds};
use super::generic::{list, option};
use super::value::Value;
use crate::core_form::Site;
use paka_rt::ops::{self, Float, Int, Text, white};

type Out = Result<Value, Trap>;

fn text(s: impl Into<String>) -> Value {
    Value::Str(s.into().into())
}

fn strings<'a>(parts: impl IntoIterator<Item = &'a str>) -> Out {
    Ok(list(parts.into_iter().map(text).collect()))
}

/// A grown String, or a `memory` trap past M2-06's cap.
fn grown(made: Result<Text, ops::Trap>, site: Site) -> Out {
    made.map(Value::Str).map_err(|k| Trap::bare(k.into(), site))
}

/// Runs the String row `name` on `args`; `None` when no String row fits them.
pub fn call(name: &str, args: &[Value], site: Site) -> Option<Out> {
    Some(match (name, args) {
        ("len", [Value::Str(t)]) => Ok(Value::Int(t.len())),
        ("is_empty", [Value::Str(t)]) => Ok(Value::Bool(t.as_str().is_empty())),
        ("contains", [Value::Str(t), Value::Str(p)]) => {
            Ok(Value::Bool(t.as_str().contains(p.as_str())))
        }
        ("find", [Value::Str(t), Value::Str(p)]) => Ok(option(t.find(p).map(Value::Int))),
        ("parse_int", [Value::Str(t)]) => Ok(option(Int::parse(t.as_str()).map(Value::Int))),
        ("parse_float", [Value::Str(t)]) => Ok(option(Float::parse(t.as_str()).map(Value::Float))),
        ("code_at", [Value::Str(t), i]) => (t.code_at(i.int()).map(Value::Int))
            .map_err(|_| out_of_bounds(site, format!("index {}, length {}", i.int(), t.len()))),
        ("from_code", [Value::Int(n)]) => (Text::from_code(*n).map(Value::Str))
            .map_err(|k| Trap::new(k, site, format!("from_code({n})"))),
        ("split", [Value::Str(t), Value::Str(sep)]) => match t.split(sep) {
            Ok(parts) => strings(parts),
            Err(k) => Err(Trap::bare(k.into(), site)),
        },
        ("chars", [Value::Str(t)]) => strings(t.chars()),
        ("lines", [Value::Str(t)]) => strings(t.lines()),
        ("words", [Value::Str(t)]) => strings(t.words()),
        ("join", [Value::List(parts), Value::Str(sep)]) => {
            let parts: Vec<_> = parts.0.iter().map(text_of).collect();
            grown(sep.join(&parts), site)
        }
        ("repeat", [Value::Str(t), n]) => match t.repeat(n.int()) {
            Err(ops::Trap::OutOfBounds) => Err(out_of_bounds(site, format!("count {}", n.int()))),
            made => grown(made, site),
        },
        ("pad_left", [Value::Str(t), width]) => grown(t.pad(width.int(), true), site),
        ("pad_right", [Value::Str(t), width]) => grown(t.pad(width.int(), false), site),
        ("trim", [Value::Str(t)]) => Ok(text(t.as_str().trim_matches(white))),
        ("trim_start", [Value::Str(t)]) => Ok(text(t.as_str().trim_start_matches(white))),
        ("trim_end", [Value::Str(t)]) => Ok(text(t.as_str().trim_end_matches(white))),
        ("starts_with", [Value::Str(t), Value::Str(p)]) => {
            Ok(Value::Bool(t.as_str().starts_with(p.as_str())))
        }
        ("ends_with", [Value::Str(t), Value::Str(p)]) => {
            Ok(Value::Bool(t.as_str().ends_with(p.as_str())))
        }
        ("replace", [Value::Str(t), Value::Str(old), Value::Str(new)]) => {
            grown(t.replace(old, new), site)
        }
        ("upper", [Value::Str(t)]) => Ok(text(t.as_str().to_ascii_uppercase())),
        ("lower", [Value::Str(t)]) => Ok(text(t.as_str().to_ascii_lowercase())),
        _ => return None,
    })
}

fn text_of(v: &Value) -> &str {
    match v {
        Value::Str(t) => t.as_str(),
        _ => unreachable!("a checked String"),
    }
}
