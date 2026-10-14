//! One line of `claude -p --output-format stream-json`, read strictly: a line that does not have
//! the expected shape is an error, never a zero.

use crate::Error;
use crate::json::{self, Value};
use crate::usage::Tokens;

pub enum Event {
    /// The `system` init line; the key source is `none` when no API key is in use.
    Init { api_key_source: Option<String> },
    /// An assistant message, with the model that wrote it.
    Assistant { model: String },
    /// The end of a turn.
    Result(Outcome),
    /// The conversation is being, or was, compacted.
    Compaction,
    /// A kind of line that carries nothing counted: other `system` lines (hooks, status) and
    /// `rate_limit_event`.
    Ignored,
}

/// The end of a turn. Failed or not, the usage it reports was spent.
pub enum Outcome {
    Reply {
        text: String,
        tokens: Tokens,
        stop_reason: Option<String>,
    },
    /// The provider failed the turn; the message is for the log. No usage line counts as none.
    Failed { message: String, tokens: Tokens },
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
        "system" => system(&event),
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
        "init" => return init(event),
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

fn init(event: &Value) -> Result<Event, String> {
    let api_key_source = match event.get("apiKeySource") {
        None => None,
        Some(source) => Some(
            source
                .as_str()
                .ok_or("\"apiKeySource\" is not a string")?
                .into(),
        ),
    };
    Ok(Event::Init { api_key_source })
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
        let tokens = event.get("usage").map_or(Ok(Tokens::default()), usage)?;
        return Ok(Outcome::Failed {
            message: message.into(),
            tokens,
        });
    }
    let stop_reason = match event.get("stop_reason") {
        None | Some(Value::Null) => None,
        Some(reason) => Some(
            reason
                .as_str()
                .ok_or("\"stop_reason\" is not a string")?
                .into(),
        ),
    };
    Ok(Outcome::Reply {
        text: str_field(event, "result")?.into(),
        tokens: usage(field(event, "usage")?)?,
        stop_reason,
    })
}

/// Input is plain, cache-write and cache-read input together.
fn usage(usage: &Value) -> Result<Tokens, String> {
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
    Ok(Tokens {
        input,
        output: count(usage, "output_tokens")?,
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
