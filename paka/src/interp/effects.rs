//! The effects a handler returns, decoded from the interpreter's values and performed by the
//! shared performer (`paka_rt::effects`, M4-05 section 3).

use super::value::{FnKey, Value, position};
use crate::core_form::{App, Effect as Number, Program};
use crate::foreign::{self, Hosts};
use paka_rt::effects::{self, Effect, Next};
use paka_rt::ops::Int;
use paka_rt::route::Event;
use std::collections::VecDeque;
use std::mem;

#[cfg(test)]
pub use paka_rt::effects::Reason;
pub use paka_rt::effects::{End, Host, World};

const PRINT: usize = Number::Print as usize;
const READ_FILE: usize = Number::ReadFile as usize;
const WRITE_FILE: usize = Number::WriteFile as usize;
const EXIT: usize = Number::Exit as usize;
const EXIT_WITH: usize = Number::ExitWith as usize;
const FOREIGN: usize = Number::Foreign as usize;

/// A message waiting its turn: given, an answer whose function is applied at the front, or a
/// visual app's event, worked out against the tree of its turn.
pub enum Queued {
    Message(Value),
    Answer(FnKey, Value),
    Event(Event),
}

/// Performs the effects in order, queueing the answers; `Some` when one ends the run, `None`
/// after the last. Without `hosts`, as in a UI example, an allowed foreign effect is dropped.
pub fn perform(
    core: &Program,
    app: &App,
    hosts: Option<&mut Hosts>,
    answer: &mut Value,
    world: &mut impl World,
    queue: &mut VecDeque<Queued>,
) -> Option<End> {
    let Value::List(effects) = answer else {
        unreachable!("a checked List");
    };
    let mut hosts = hosts;
    for effect in &mut effects.0 {
        let Value::Variant(number, fields) = effect else {
            unreachable!("a checked Effect");
        };
        if *number == FOREIGN {
            match call(core, app, hosts.as_deref_mut(), &mut fields.0) {
                Ok(answer) => queue.extend(answer),
                Err(end) => return Some(end),
            }
            continue;
        }
        let effect = decode(*number, &mut fields.0);
        match effects::perform(&app.name, &app.grants, effect, world) {
            Ok(Some(Next::Message(m))) => queue.push_back(Queued::Message(m)),
            Ok(Some(Next::Answer(f, text))) => {
                queue.push_back(Queued::Answer(f, Value::Str(text.into())))
            }
            Ok(None) => {}
            Err(end) => return Some(end),
        }
    }
    None
}

/// A foreign effect: refused without its grant, else run by its declaration's host. The answer
/// is `then` with the result, or `or` with the reason the call failed.
fn call(
    core: &Program,
    app: &App,
    hosts: Option<&mut Hosts>,
    fields: &mut [Value],
) -> Result<Option<Queued>, End> {
    let [Value::Int(id), args @ .., then, Value::Fn(or)] = fields else {
        unreachable!("a checked foreign effect");
    };
    let effect = &core.foreign[position(*id).expect("an effect index")];
    let library = &core.libraries[effect.library];
    if !app.foreign.contains(&library.name) {
        return Err(paka_rt::foreign::refused(
            &app.name,
            &effect.name,
            &library.name,
        ));
    }
    let Some(hosts) = hosts else {
        return Ok(None);
    };
    let args: Vec<_> = args.iter().map(plain).collect();
    Ok(Some(
        match foreign::call(hosts, effect.library, library, effect.number, &args) {
            Ok(Some(result)) => Queued::Answer(answer_fn(then), value(result)),
            Ok(None) => Queued::Message(mem::take(then)),
            Err(reason) => Queued::Answer(*or, Value::Str(reason.into())),
        },
    ))
}

fn answer_fn(then: &Value) -> FnKey {
    let Value::Fn(f) = then else {
        unreachable!("a function with a result answers through a function");
    };
    *f
}

/// An argument as it crosses to C.
#[allow(clippy::disallowed_methods)] // the value leaves Paka for the host
fn plain(v: &Value) -> foreign::Value<'_> {
    match v {
        Value::Int(n) => foreign::Value::Int(n.get()),
        Value::Float(x) => foreign::Value::Float(*x),
        Value::Bool(b) => foreign::Value::Bool(*b),
        Value::Str(t) => foreign::Value::String(t.as_str()),
        _ => unreachable!("a checked plain type"),
    }
}

fn value(result: foreign::Value) -> Value {
    match result {
        foreign::Value::Int(n) => Value::Int(Int::new(n)),
        foreign::Value::Float(x) => Value::Float(x),
        foreign::Value::Bool(b) => Value::Bool(b),
        foreign::Value::String(_) => unreachable!("a result is never a String"),
    }
}

/// Effect `number` with its fields.
fn decode(number: usize, fields: &mut [Value]) -> Effect<'_, FnKey, Value> {
    match (number, fields) {
        (PRINT, [Value::Str(text)]) => Effect::Print(text.as_str()),
        (READ_FILE, [Value::Str(path), Value::Fn(then), Value::Fn(or)]) => {
            Effect::ReadFile(path.as_str(), *then, *or)
        }
        (WRITE_FILE, [Value::Str(path), Value::Str(text), then, Value::Fn(or)]) => {
            Effect::WriteFile(path.as_str(), text.as_str(), mem::take(then), *or)
        }
        (EXIT, _) => Effect::Exit,
        (EXIT_WITH, [Value::Int(status)]) => Effect::ExitWith(*status),
        _ => unreachable!("a checked Effect"),
    }
}
