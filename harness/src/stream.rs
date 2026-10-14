//! One line of `claude -p --output-format stream-json`, read strictly: a line that does not have
//! the expected shape is an error, never a zero.

use crate::Error;
use crate::json::{self, Value};
use crate::usage::Tokens;

pub enum Event {
    /// An assistant message, with the model that wrote it.
    Assistant { model: String },
    /// The end of a turn.
    Result(Outcome),
    /// The conversation is being, or was, compacted.
    Compaction,
    /// A kind of line that carries nothing counted: `system` (init, hooks, status) and
    /// `rate_limit_event`.
    Ignored,
}

pub enum Outcome {
    Reply {
        text: String,
        tokens: Tokens,
    },
    /// The provider failed the turn; the message is for the log.
    Failed(String),
}

pub fn parse_event(line: &str) -> Result<Event, Error> {
    read(line).map_err(|what| {
        let shown: String = line.chars().take(200).collect();
        Error::Malformed(format!("{what}: {shown}"))
    })
}

fn read(line: &str) -> Result<Event, String> {
    let event = json::parse(line)?;
    match str_field(&event, "type")? {
        "system" => Ok(system(&event)?),
        "assistant" => {
            let model = str_field(field(&event, "message")?, "model")?;
            Ok(Event::Assistant {
                model: model.into(),
            })
        }
        "result" => result(&event).map(Event::Result),
        "rate_limit_event" => Ok(Event::Ignored),
        other => Err(format!("unknown event type {other:?}")),
    }
}

fn system(event: &Value) -> Result<Event, String> {
    let compacting = match str_field(event, "subtype")? {
        "compact_boundary" => true,
        "status" => event.get("status").and_then(Value::as_str) == Some("compacting"),
        _ => false,
    };
    Ok(if compacting {
        Event::Compaction
    } else {
        Event::Ignored
    })
}

fn result(event: &Value) -> Result<Outcome, String> {
    let subtype = str_field(event, "subtype")?;
    let is_error = field(event, "is_error")?
        .as_bool()
        .ok_or("is_error is not a boolean")?;
    if is_error || subtype != "success" {
        let message = event
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or(subtype);
        return Ok(Outcome::Failed(message.into()));
    }
    let usage = field(event, "usage")?;
    let mut input = 0u64;
    for key in [
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        input = input
            .checked_add(count(usage, key)?)
            .ok_or("input tokens overflow")?;
    }
    Ok(Outcome::Reply {
        text: str_field(event, "result")?.into(),
        tokens: Tokens {
            input,
            output: count(usage, "output_tokens")?,
        },
    })
}

fn field<'a>(object: &'a Value, key: &str) -> Result<&'a Value, String> {
    object.get(key).ok_or_else(|| format!("no field {key:?}"))
}

fn str_field<'a>(object: &'a Value, key: &str) -> Result<&'a str, String> {
    field(object, key)?
        .as_str()
        .ok_or_else(|| format!("{key:?} is not a string"))
}

fn count(object: &Value, key: &str) -> Result<u64, String> {
    field(object, key)?
        .as_u64()
        .ok_or_else(|| format!("{key:?} is not a whole count"))
}
