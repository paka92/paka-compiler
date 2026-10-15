//! The built-ins the interpreter runs: the generic rows, the String rows, the number rows, `assert`
//! and `unreachable`.

use super::eval::{Done, Kind, Stop, Trap};
use super::value::Value;
use super::{generic, number, text};
use crate::builtins::{ROWS, name};
use crate::core_form::Site;

/// Runs row `row` of the built-in table on `args`, which it may update.
pub fn call(row: usize, args: &mut [Value], site: Site) -> Done {
    let name = name(ROWS[row].1);
    let done = generic::call(name, args, site)
        .or_else(|| text::call(name, args, site))
        .or_else(|| number::call(name, args, site));
    if let Some(done) = done {
        return Ok(done?);
    }
    Ok(match (name, &args[..]) {
        ("assert", [Value::Bool(true)]) => Value::Unit,
        ("assert" | "unreachable", _) => return Err(Trap::bare(Kind::Assert, site).into()),
        _ => {
            let reason = format!("built-in {name} does not take these arguments");
            return Err(Stop::Internal(reason));
        }
    })
}
