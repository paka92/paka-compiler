//! The built-ins the interpreter runs so far; the others end the run (M2-33 and M2-34 add them).

use super::eval::{Kind, Trap};
use super::value::Value;
use crate::builtins::{ROWS, name};
use crate::core_form::Site;
use paka_rt::ops::{Int, Text};

/// Runs row `row` of the built-in table on `args`, which it may update.
pub fn call(row: usize, args: &mut [Value], site: Site) -> Result<Value, Trap> {
    match (name(ROWS[row].1), &args[..]) {
        ("assert", [Value::Bool(true)]) => Ok(Value::Unit),
        ("assert" | "unreachable", _) => Err(Trap::bare(Kind::Assert, site)),
        ("abs", [Value::Int(n)]) => n
            .abs()
            .map(Value::Int)
            .map_err(|k| Trap::new(k, site, format!("abs({n})"))),
        ("abs", [Value::Float(x)]) => Ok(Value::Float(x.abs())),
        ("to_int", [Value::Float(x)]) => x.to_int().map(Value::Int).map_err(|k| {
            let shown = x.fixed(Int::new(0)).unwrap_or_default();
            Trap::new(k, site, format!("to_int({shown})"))
        }),
        ("from_code", [Value::Int(n)]) => Text::from_code(*n)
            .map(Value::Str)
            .map_err(|k| Trap::new(k, site, format!("from_code({n})"))),
        (other, _) => panic!("built-in {other} is not implemented yet"),
    }
}
