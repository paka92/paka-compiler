//! What awaits the owner, as the gate and an edit reply list it: one line per item, or the
//! pending list of docs/decisions/M7-01.md section 1.6.

use crate::approvals::{Change, Record};
use paka_json::Value;
use std::collections::BTreeSet;

pub const NEXT: &str = "The owner runs: paka approve. Or revert the change.";

pub enum Item {
    /// A record the source or an owned file no longer matches.
    Change(Change),
    /// A human-owned example that does not pass, and how.
    Failing {
        program: String,
        name: String,
        result: String,
    },
    /// The key, the store or a gated program refused as a whole.
    Refused(String),
}

impl Item {
    /// Kind, program and name, then what differs.
    pub fn line(&self) -> String {
        let change = match self {
            Item::Change(change) => change,
            Item::Failing {
                program,
                name,
                result,
            } => return format!("example {program} {name}: {result}"),
            Item::Refused(why) => return why.clone(),
        };
        let what = match (&change.approved, &change.now) {
            (Some(Record::Grants { grants: a, .. }), Some(Record::Grants { grants: b, .. })) => {
                format!("approved {}, found {}", list(a), list(b))
            }
            (None, Some(Record::Grants { grants, .. })) => {
                format!("never approved, found {}", list(grants))
            }
            _ => what(change).into(),
        };
        match change.approved.as_ref().or(change.now.as_ref()) {
            Some(Record::Grants { program, app, .. }) => format!("grants {program} {app}: {what}"),
            Some(Record::Example { program, name, .. }) => {
                format!("example {program} {name}: {what}")
            }
            Some(Record::File { path, .. }) => format!("file {path}: {what}"),
            None => what,
        }
    }

    pub fn json(&self) -> Value {
        let change = match self {
            Item::Change(change) => change,
            Item::Failing {
                program,
                name,
                result,
            } => {
                return Value::object([
                    ("kind", "example".into()),
                    ("program", program.as_str().into()),
                    ("name", name.as_str().into()),
                    ("result", result.as_str().into()),
                ]);
            }
            Item::Refused(why) => {
                return Value::object([
                    ("kind", "refused".into()),
                    ("message", why.as_str().into()),
                ]);
            }
        };
        let grants = |r: &Option<Record>| match r {
            Some(Record::Grants { grants, .. }) => {
                Value::from(grants.iter().map(String::as_str).collect::<Vec<_>>())
            }
            _ => Value::Null,
        };
        match change.approved.as_ref().or(change.now.as_ref()) {
            Some(Record::Grants { program, app, .. }) => Value::object([
                ("kind", "grants".into()),
                ("program", program.as_str().into()),
                ("app", app.as_str().into()),
                ("approved", grants(&change.approved)),
                ("found", grants(&change.now)),
            ]),
            Some(Record::Example { program, name, .. }) => Value::object([
                ("kind", "example".into()),
                ("program", program.as_str().into()),
                ("name", name.as_str().into()),
                ("change", what(change).into()),
            ]),
            Some(Record::File { path, .. }) => Value::object([
                ("kind", "file".into()),
                ("path", path.as_str().into()),
                ("change", what(change).into()),
            ]),
            None => Value::Null,
        }
    }
}

/// The fields `pending`, `blocks` and `next`; with no item, nothing blocks and nothing is next.
pub fn fields(items: &[Item]) -> Vec<(&'static str, Value)> {
    let pending: Vec<_> = items.iter().map(Item::json).collect();
    let (blocks, next) = match items.is_empty() {
        true => (vec![], None),
        false => (vec!["merge", "release"], Some(NEXT)),
    };
    vec![
        ("pending", pending.into()),
        ("blocks", blocks.into()),
        ("next", next.into()),
    ]
}

fn what(change: &Change) -> &'static str {
    match (&change.approved, &change.now) {
        (None, _) => "never approved",
        (Some(_), Some(_)) => "edited",
        (Some(_), None) => "renamed or deleted",
    }
}

fn list(grants: &BTreeSet<String>) -> String {
    let names: Vec<_> = grants.iter().map(String::as_str).collect();
    format!("[{}]", names.join(", "))
}
