//! The tool interface's transport and dispatch (M4-16, docs/decisions/M4-03.md): one JSON request
//! per line in, one reply per line out, handled in arrival order. Each request is checked against
//! its call's fields before a handler sees it, so a handler gets only well-formed calls.

use paka_json::{Value, parse};
use std::io::{self, BufRead, Write};

pub const VERSION: usize = 1;

/// A call that passed the envelope and field checks.
pub struct Call {
    pub name: &'static str,
    id: Value,
    fields: Value,
}

impl Call {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key).filter(|v| **v != Value::Null)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    pub fn int(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(Value::as_u64)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }

    pub fn strs(&self, key: &str) -> Option<Vec<&str>> {
        match self.get(key)? {
            Value::Array(items) => items.iter().map(Value::as_str).collect(),
            _ => None,
        }
    }
}

/// The bytes of the reply line that answers `call` with `fields`, without its line feed.
pub fn line_len(call: &Call, fields: &[(&'static str, Value)]) -> usize {
    reply_line(call.id.clone(), Ok(fields.to_vec()))
        .to_string()
        .len()
}

/// A refused call: its kind, message and the extra fields its kind lists.
pub struct Failure {
    pub kind: &'static str,
    pub message: String,
    pub extra: Vec<(&'static str, Value)>,
}

impl Failure {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Failure {
        Failure {
            kind,
            message: message.into(),
            extra: vec![],
        }
    }
}

pub type Reply = Result<Vec<(&'static str, Value)>, Failure>;

/// Answers each line of `input` on `output` until the input ends.
pub fn serve(
    input: impl BufRead,
    mut output: impl Write,
    mut handle: impl FnMut(&Call) -> Reply,
) -> io::Result<()> {
    for line in input.split(b'\n') {
        let (id, reply) = match request(&line?) {
            Ok(call) => (call.id.clone(), handle(&call)),
            Err((id, failure)) => (id, Err(failure)),
        };
        writeln!(output, "{}", reply_line(id, reply))?;
        output.flush()?;
    }
    Ok(())
}

fn reply_line(id: Value, reply: Reply) -> Value {
    let head = [("v", Value::from(VERSION)), ("id", id)];
    let rest = match reply {
        Ok(fields) => [("ok", true.into())].into_iter().chain(fields).collect(),
        Err(f) => {
            let error = [("kind", f.kind.into()), ("message", f.message.into())];
            let error = Value::object(error.into_iter().chain(f.extra));
            vec![("ok", false.into()), ("error", error)]
        }
    };
    Value::object(head.into_iter().chain(rest))
}

/// The field types a request may hold.
#[derive(Clone, Copy)]
enum Ty {
    Str,
    /// A string or null.
    Base,
    Int,
    Bool,
    Strs,
    /// A run call's fields, inside an edit.
    Run,
}

/// A field's name, type and whether it is required.
type Fields = &'static [(&'static str, Ty, bool)];

/// Each call's fields.
const CALLS: [(&str, Fields); 4] = [
    (
        "find",
        &[
            ("name", Ty::Str, false),
            ("users_of", Ty::Str, false),
            ("takes", Ty::Strs, false),
            ("gives", Ty::Str, false),
            ("kind", Ty::Str, false),
            ("failing", Ty::Bool, false),
            ("budget", Ty::Int, false),
            ("after", Ty::Str, false),
        ],
    ),
    (
        "view",
        &[("name", Ty::Str, true), ("budget", Ty::Int, false)],
    ),
    (
        "edit",
        &[
            ("name", Ty::Str, true),
            ("base", Ty::Base, true),
            ("text", Ty::Base, true),
            ("run", Ty::Run, false),
            ("state", Ty::Str, false),
        ],
    ),
    ("run", RUN),
];

const RUN: Fields = &[
    ("app", Ty::Str, true),
    ("args", Ty::Strs, false),
    ("limit_ms", Ty::Int, false),
    ("max_bytes", Ty::Int, false),
];

const ENVELOPE: [&str; 3] = ["v", "id", "call"];

/// The call on one line, or the refusal and the id to echo with it.
fn request(line: &[u8]) -> Result<Call, (Value, Failure)> {
    let bad = |message: String| Failure::new("bad_request", message);
    let object = std::str::from_utf8(line).ok().and_then(|l| parse(l).ok());
    let Some(Value::Object(fields)) = object else {
        let message = "a request is one JSON object per line";
        return Err((Value::Null, bad(message.into())));
    };
    let object = Value::Object(fields);
    let id = match object.get("id") {
        Some(id @ (Value::Num(_) | Value::Str(_))) => id.clone(),
        _ => return Err((Value::Null, bad("`id` must be a number or a string".into()))),
    };
    let refuse = |failure| Err((id.clone(), failure));
    match object.get("v") {
        Some(Value::Num(v)) if v.parse() == Ok(VERSION) => {}
        Some(Value::Num(_)) => {
            let message = format!("this server speaks version {VERSION}");
            return refuse(Failure::new("bad_version", message));
        }
        _ => return refuse(bad("`v` must be a number".into())),
    }
    let Some(name) = object.get("call").and_then(Value::as_str) else {
        return refuse(bad("`call` must be a string".into()));
    };
    let Some((name, spec)) = CALLS.iter().find(|(n, _)| *n == name) else {
        let message = format!("no call `{name}`; calls are find, view, edit, run");
        return refuse(Failure::new("unknown_call", message));
    };
    match fields_fit(name, spec, &object, &ENVELOPE) {
        Ok(()) => Ok(Call {
            name,
            id,
            fields: object,
        }),
        Err(message) => refuse(bad(message)),
    }
}

/// Checks the fields of `object` against `spec`, past the keys in `skip`.
fn fields_fit(call: &str, spec: Fields, object: &Value, skip: &[&str]) -> Result<(), String> {
    let Value::Object(fields) = object else {
        return Err(format!("`{call}` must be an object"));
    };
    for (key, value) in fields.iter().filter(|(k, _)| !skip.contains(&k.as_str())) {
        let Some((_, ty, _)) = spec.iter().find(|(k, ..)| k == key) else {
            return Err(format!("{call} has no field `{key}`"));
        };
        if !fits(*ty, value) {
            return Err(format!("`{key}` must be {}", ty_name(*ty)));
        }
        if let Ty::Run = ty {
            fields_fit("run", RUN, value, &[])?;
        }
    }
    match spec
        .iter()
        .find(|(k, _, required)| *required && object.get(k).is_none())
    {
        Some((key, ..)) => Err(format!("{call} needs the field `{key}`")),
        None => Ok(()),
    }
}

fn fits(ty: Ty, value: &Value) -> bool {
    match (ty, value) {
        (Ty::Str, Value::Str(_)) | (Ty::Base, Value::Str(_) | Value::Null) => true,
        (Ty::Int, v) => v.as_u64().is_some(),
        (Ty::Bool, Value::Bool(_)) | (Ty::Run, Value::Object(_)) => true,
        (Ty::Strs, Value::Array(items)) => items.iter().all(|i| i.as_str().is_some()),
        _ => false,
    }
}

fn ty_name(ty: Ty) -> &'static str {
    match ty {
        Ty::Str => "a string",
        Ty::Base => "a string or null",
        Ty::Int => "a whole number",
        Ty::Bool => "true or false",
        Ty::Strs => "a list of strings",
        Ty::Run => "an object",
    }
}
