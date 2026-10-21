//! The stored types of a version (docs/decisions/M5-07.md section 1.2): the app's state type and
//! the layout of every type it reaches. Defaults, comments and layout of the text are not part of
//! it; names, order, field types and variants are.

use crate::ast::{Def, Entry, Field};
use crate::parser::parse_def;
use crate::workspace::Workspace;
use std::collections::BTreeMap;

const PATH: &str = "main.paka";

#[derive(PartialEq)]
pub struct Stored {
    state: Option<String>,
    types: BTreeMap<String, Layout>,
}

/// A record's fields or an enum's variants, in order, each with what it holds.
#[derive(PartialEq)]
struct Layout {
    record: bool,
    parts: Vec<(String, String)>,
}

impl Stored {
    pub fn of(workspace: &Workspace) -> Stored {
        let app = workspace.defs().find(|(_, text)| text.starts_with("app "));
        let state = app.and_then(|(_, text)| state(text));
        let names = state.as_deref().map(|s| workspace.stored(s));
        let types = (names.into_iter().flatten())
            .filter_map(|name| {
                let layout = Layout::of(workspace.def(&name)?.0)?;
                Some((name, layout))
            })
            .collect();
        Stored { state, types }
    }

    /// How `new` differs from this, one line per changed type.
    pub fn changes(&self, new: &Stored) -> Vec<String> {
        let mut changes = vec![];
        if self.state != new.state {
            let name = |s: &Option<String>| s.clone().unwrap_or_else(|| "none".into());
            let (was, now) = (name(&self.state), name(&new.state));
            changes.push(format!(
                "state: the app's state type is now {now}, was {was}"
            ));
        }
        for (name, old) in &self.types {
            match new.types.get(name) {
                None => changes.push(format!("{name}: no longer stored")),
                Some(new) if new != old => changes.push(format!("{name}: {}", old.changes(new))),
                _ => {}
            }
        }
        let added = new.types.keys().filter(|n| !self.types.contains_key(*n));
        changes.extend(added.map(|name| format!("{name}: newly stored")));
        changes
    }
}

impl Layout {
    fn of(text: &str) -> Option<Layout> {
        let (record, parts) = match parse_def(PATH, text.as_bytes()).ok()? {
            Def::Type { fields, .. } => {
                let parts = fields
                    .iter()
                    .map(|f| (f.name.text.into(), f.ty.to_string()));
                (true, parts.collect())
            }
            Def::Enum { variants, .. } => {
                let held = |fields: &[Field]| fields.iter().map(typed).collect::<Vec<_>>();
                let parts = variants
                    .iter()
                    .map(|v| (v.name.text.into(), held(&v.fields)));
                (false, parts.map(|(n, f)| (n, f.join(", "))).collect())
            }
            _ => return None,
        };
        Some(Layout { record, parts })
    }

    fn changes(&self, new: &Layout) -> String {
        if self.record != new.record {
            return format!("now {}", if new.record { "a record" } else { "an enum" });
        }
        let what = if self.record { "field" } else { "variant" };
        let held = |parts: &[(String, String)], name: &str| {
            parts
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, h)| h.clone())
        };
        let mut changes = vec![];
        for (name, was) in &self.parts {
            match held(&new.parts, name) {
                None => changes.push(format!("{what} {name} removed")),
                Some(now) if now != *was => changes.push(match self.record {
                    true => format!("field {name} is now {now}, was {was}"),
                    false => format!("variant {name} now holds ({now}), was ({was})"),
                }),
                _ => {}
            }
        }
        for (name, _) in new
            .parts
            .iter()
            .filter(|(n, _)| held(&self.parts, n).is_none())
        {
            changes.push(format!("{what} {name} added"));
        }
        if changes.is_empty() {
            changes.push(format!("{what}s reordered"));
        }
        changes.join(", ")
    }
}

/// The state type an app's text names.
fn state(text: &str) -> Option<String> {
    let Ok(Def::App { entries, .. }) = parse_def(PATH, text.as_bytes()) else {
        return None;
    };
    entries.iter().find_map(|e| match e {
        Entry::State(_, name) => Some(name.text.into()),
        _ => None,
    })
}

/// A field as `name: Type`.
fn typed(field: &Field) -> String {
    format!("{}: {}", field.name.text, field.ty)
}
