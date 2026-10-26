//! The effects a handler returns, decoded from the interpreter's values and performed by the
//! shared performer (`paka_rt::effects`, M4-05 section 3).

use super::value::{FnKey, Value, position};
use crate::core_form::{App, Effect as Number, Foreign};
use paka_rt::effects::{self, Effect, Next};
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
/// after the last.
pub fn perform(
    app: &App,
    foreign: &[Foreign],
    answer: &mut Value,
    world: &mut impl World,
    queue: &mut VecDeque<Queued>,
) -> Option<End> {
    let Value::List(effects) = answer else {
        unreachable!("a checked List");
    };
    for effect in &mut effects.0 {
        let Value::Variant(number, fields) = effect else {
            unreachable!("a checked Effect");
        };
        if *number == FOREIGN {
            match call(app, foreign, &fields.0) {
                Ok(answer) => queue.push_back(answer),
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

/// A foreign effect: refused without its grant, and failed with `no host` until a host runs it.
fn call(app: &App, foreign: &[Foreign], fields: &[Value]) -> Result<Queued, End> {
    let (Some(Value::Int(id)), Some(Value::Fn(or))) = (fields.first(), fields.last()) else {
        unreachable!("a checked foreign effect");
    };
    let effect = &foreign[position(*id).expect("an effect index")];
    if !app.foreign.contains(&effect.grant) {
        let (name, grant) = (&effect.name, &effect.grant);
        let why = format!("effect {name} refused: grant {grant} is missing");
        return Err((3, format!("{}: {why}\n", app.name)));
    }
    Ok(Queued::Answer(*or, Value::Str("no host".into())))
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
