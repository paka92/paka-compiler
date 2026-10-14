//! The built-ins the interpreter runs so far; the others end the run (M2-33 and M2-34 add them).

use super::eval::{Done, Kind, Stop, Trap};
use super::generic;
use super::value::Value;
use crate::builtins::{ROWS, name};
use crate::core_form::Site;
use paka_rt::ops::Text;

/// Runs row `row` of the built-in table on `args`, which it may update.
pub fn call(row: usize, args: &mut [Value], site: Site) -> Done {
    let name = name(ROWS[row].1);
    if let Some(done) = generic::call(name, args, site) {
        return Ok(done?);
    }
    Ok(match (name, &args[..]) {
        ("assert", [Value::Bool(true)]) => Value::Unit,
        ("assert" | "unreachable", _) => return Err(Trap::bare(Kind::Assert, site).into()),
        ("abs", [Value::Int(n)]) => n
            .abs()
            .map(Value::Int)
            .map_err(|k| Trap::new(k, site, format!("abs({n})")))?,
        ("abs", [Value::Float(x)]) => Value::Float(x.abs()),
        ("to_int", [Value::Float(x)]) => x
            .to_int()
            .map(Value::Int)
            .map_err(|k| Trap::new(k, site, format!("to_int({})", x.show())))?,
        ("from_code", [Value::Int(n)]) => Text::from_code(*n)
            .map(Value::Str)
            .map_err(|k| Trap::new(k, site, format!("from_code({n})")))?,
        _ => {
            let reason = format!("built-in {name} is not implemented yet");
            return Err(Stop::Internal(reason));
        }
    })
}
