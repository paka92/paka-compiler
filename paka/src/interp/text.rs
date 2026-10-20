//! The String built-ins (M2-14 sections 1.3 and 1.5 to 1.7, rules in section 1.14): positions
//! count code points, matching is exact, white space is the six characters 9 to 13 and 32, and
//! case changes touch `a` to `z` only. A row that grows a String checks M2-06's cap first.

use super::eval::{CAP, Kind, Trap, out_of_bounds};
use super::generic::{count, list, option};
use super::value::{Value, position};
use crate::core_form::Site;
use paka_rt::ops::{Float, Int, Text, white};

type Out = Result<Value, Trap>;

fn text(s: impl Into<String>) -> Value {
    Value::Str(s.into().into())
}

fn strings<S: Into<String>>(parts: impl Iterator<Item = S>) -> Out {
    Ok(list(parts.map(text).collect()))
}

/// Checks M2-06's cap before a String of `bytes` bytes is made.
fn room(bytes: u128, site: Site) -> Result<(), Trap> {
    match bytes > CAP as u128 {
        true => Err(Trap::bare(Kind::Memory, site)),
        false => Ok(()),
    }
}

/// Runs the String row `name` on `args`; `None` when no String row fits them.
pub fn call(name: &str, args: &[Value], site: Site) -> Option<Out> {
    Some(match (name, args) {
        ("len", [Value::Str(t)]) => Ok(Value::Int(t.len())),
        ("is_empty", [Value::Str(t)]) => Ok(Value::Bool(t.as_str().is_empty())),
        ("contains", [Value::Str(t), Value::Str(p)]) => {
            Ok(Value::Bool(t.as_str().contains(p.as_str())))
        }
        ("find", [Value::Str(t), Value::Str(p)]) => {
            let at = t.as_str().find(p.as_str());
            Ok(option(at.map(|b| count(t.as_str()[..b].chars().count()))))
        }
        ("parse_int", [Value::Str(t)]) => Ok(option(Int::parse(t.as_str()).map(Value::Int))),
        ("parse_float", [Value::Str(t)]) => Ok(option(Float::parse(t.as_str()).map(Value::Float))),
        ("code_at", [Value::Str(t), i]) => (t.code_at(i.int()).map(Value::Int))
            .map_err(|_| out_of_bounds(site, format!("index {}, length {}", i.int(), t.len()))),
        ("from_code", [Value::Int(n)]) => (Text::from_code(*n).map(Value::Str))
            .map_err(|k| Trap::new(k, site, format!("from_code({n})"))),
        ("split", [Value::Str(t), Value::Str(sep)]) => split(t.as_str(), sep.as_str(), CAP, site),
        ("chars", [Value::Str(t)]) => strings(t.as_str().chars()),
        ("lines", [Value::Str(t)]) => {
            let mut parts: Vec<_> = t.as_str().split('\n').collect();
            if parts.last() == Some(&"") {
                parts.pop();
            }
            strings(parts.into_iter())
        }
        ("words", [Value::Str(t)]) => strings(t.words()),
        ("join", [Value::List(parts), Value::Str(sep)]) => join(&parts.0, sep, site),
        ("repeat", [Value::Str(t), n]) => repeat(t, n.int(), site),
        ("pad_left", [Value::Str(t), width]) => pad(t, width.int(), true, site),
        ("pad_right", [Value::Str(t), width]) => pad(t, width.int(), false, site),
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
            replace(t.as_str(), old.as_str(), new.as_str(), site)
        }
        ("upper", [Value::Str(t)]) => Ok(text(t.as_str().to_ascii_uppercase())),
        ("lower", [Value::Str(t)]) => Ok(text(t.as_str().to_ascii_lowercase())),
        _ => return None,
    })
}

fn join(parts: &[Value], sep: &Text, site: Site) -> Out {
    let texts: Vec<_> = parts.iter().map(text_of).collect();
    let bytes: usize = texts.iter().map(|t| t.len()).sum();
    room(
        bytes as u128 + sep.as_str().len() as u128 * texts.len().saturating_sub(1) as u128,
        site,
    )?;
    Ok(text(texts.join(sep.as_str())))
}

fn text_of(v: &Value) -> &str {
    match v {
        Value::Str(t) => t.as_str(),
        _ => unreachable!("a checked String"),
    }
}

fn repeat(t: &Text, n: Int, site: Site) -> Out {
    let times = position(n).ok_or_else(|| out_of_bounds(site, format!("count {n}")))?;
    room(t.as_str().len() as u128 * times as u128, site)?;
    Ok(text(t.as_str().repeat(times)))
}

/// Spaces at the front or the back until the String is `width` code points long.
fn pad(t: &Text, width: Int, front: bool, site: Site) -> Out {
    let fill = position(width)
        .unwrap_or(0)
        .saturating_sub(t.as_str().chars().count());
    room(t.as_str().len() as u128 + fill as u128, site)?;
    let spaces = " ".repeat(fill);
    Ok(text(match front {
        true => spaces + t.as_str(),
        false => t.as_str().to_owned() + &spaces,
    }))
}

/// One piece per code point for an empty `sep`, which cannot pass the cap; otherwise a `cap`-th
/// match would make one piece too many, and only a text of `cap` copies of `sep` can hold one.
fn split(t: &str, sep: &str, cap: usize, site: Site) -> Out {
    match sep {
        "" => strings(t.chars()),
        _ if t.len() / sep.len() >= cap && t.matches(sep).nth(cap - 1).is_some() => {
            Err(Trap::bare(Kind::Memory, site))
        }
        _ => strings(t.split(sep)),
    }
}

/// Every match of `old`, found as `split` finds them, becomes `new`; an empty `old` matches nothing.
fn replace(s: &str, old: &str, new: &str, site: Site) -> Out {
    if old.is_empty() {
        return Ok(text(s));
    }
    let matches = s.matches(old).count() as i128;
    room(
        (s.len() as i128 + matches * (new.len() as i128 - old.len() as i128)) as u128,
        site,
    )?;
    Ok(text(s.replace(old, new)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(out: Out) -> bool {
        matches!(out, Err(t) if (t.kind, t.site) == (Kind::Memory, 7))
    }

    #[test]
    fn growth_stops_one_past_the_cap() {
        let (a, cap) = (Text::from("a"), CAP as i64);
        assert!(repeat(&a, Int::new(cap), 7).is_ok());
        assert!(memory(repeat(&a, Int::new(cap + 1), 7)));
        assert!(pad(&a, Int::new(cap), true, 7).is_ok());
        assert!(memory(pad(&a, Int::new(cap + 1), false, 7)));
        assert!(replace("ab", "b", &"c".repeat(CAP - 1), 7).is_ok());
        assert!(memory(replace("ab", "b", &"c".repeat(CAP), 7)));
        let parts = [text("a"), text("b")];
        assert!(join(&parts, &Text::from(""), 7).is_ok());
        assert!(memory(join(&parts, &Text::from("c".repeat(CAP)), 7)));
    }

    #[test]
    fn split_stops_one_past_the_cap() {
        let pieces = |out| matches!(out, Ok(Value::List(xs)) if xs.0.len() == 3);
        assert!(pieces(split("a,b,", ",", 3, 7)));
        assert!(memory(split("a,b,,", ",", 3, 7)));
        assert!(memory(split(&",".repeat(CAP), ",", CAP, 7)));
    }
}
