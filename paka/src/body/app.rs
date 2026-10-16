//! An app definition (docs/decisions/M2-15.md): the keys, then `state`, `update`, `start` and
//! `grants` in that order, whatever the order written. A name the resolver could not bind is
//! reported at its turn; a type it could not read leaves the rules that need it silent.

use super::text::render;
use super::{Cx, Res};
use crate::ast::{Entry, Expr, ExprKind, Name, Pos};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Field, Item, Node, Ty};
use std::collections::BTreeSet;

const ORDER: [&str; 4] = ["state", "update", "start", "grants"];
const MISSING: [(&str, &str); 3] = [
    ("missing-start", "start"),
    ("missing-update", "update"),
    ("missing-grants", "grants"),
];

impl<'a> Cx<'_, 'a> {
    /// `state` and `update` are what the table keeps of the first entry of each.
    pub(super) fn app(
        &mut self,
        app: &Name,
        entries: &[Entry],
        (state, update): (Option<Ty>, Option<usize>),
    ) -> Res {
        self.app_keys(app, entries)?;
        let mut ordered: Vec<_> = entries.iter().collect();
        ordered.sort_by_key(|e| ORDER.iter().position(|&k| k == key(e).1));
        let mut message = None;
        for entry in ordered {
            match entry {
                Entry::State(_, n) => self.state(n, state)?,
                Entry::Update(_, n) => message = self.update(n, state, update)?,
                Entry::Start(_, e) => self.start(e, message)?,
                Entry::Grants(_, names) => self.grants(names)?,
            }
        }
        Ok(())
    }

    fn app_keys(&self, app: &Name, entries: &[Entry]) -> Res {
        for (i, entry) in entries.iter().enumerate() {
            let (at, name) = key(entry);
            if entries[..i].iter().any(|e| key(e).1 == name) {
                let message = format!("`{name}` is written twice");
                return Err(self.fail((at, name.len()), "duplicate-app-entry", message));
            }
        }
        for (code, name) in MISSING {
            if !entries.iter().any(|e| key(e).1 == name) {
                let message = format!("the app has no `{name}`");
                return Err(self.fail((app.at, app.text.len()), code, message));
            }
        }
        Ok(())
    }

    /// The name error the resolver kept for the value that starts at `at`.
    fn named(&self, at: Pos) -> Res {
        self.resolved
            .entry_error(at)
            .map_or(Ok(()), |d| Err(d.clone()))
    }

    fn state(&mut self, n: &Name, state: Option<Ty>) -> Res {
        self.named(n.at)?;
        let defs = self.defs;
        let record = state.and_then(|t| match self.types.node(t) {
            Node::User(i) => Some((t, &defs[*i].item)),
            _ => None,
        });
        let Some((ty, Item::Record(fields))) = record else {
            return Err(self.bad_state(n));
        };
        if let Some(field) = self.function_field(ty) {
            let message = format!(
                "state {} holds a function type in field {field}; keep a message value instead \
                 and match on it in update",
                n.text
            );
            return Err(self.fail((n.at, n.text.len()), "type-mismatch", message));
        }
        if fields.iter().all(|f| f.default) {
            return Ok(());
        }
        Err(self.bad_state(n))
    }

    fn bad_state(&self, n: &Name) -> Diagnostic {
        let name = n.text;
        let message = format!(
            "state {name} is not a record type with a default on every field; \
             give every field of {name} a default, or name a record type"
        );
        self.fail((n.at, name.len()), "bad-state", message)
    }

    /// The field that holds the first function type `ty` reaches, depth first, in field order. A
    /// stack of (type, field it is in) instead of recursion, so a long chain of types fits.
    fn function_field(&self, ty: Ty) -> Option<&'a str> {
        let (mut todo, mut seen) = (vec![(ty, "")], BTreeSet::new());
        while let Some((ty, field)) = todo.pop() {
            let next = todo.len();
            match self.types.node(ty) {
                Node::Fn(..) => return Some(field),
                Node::Name(_, args) => todo.extend(args.iter().map(|&a| (a, field))),
                Node::User(i) if seen.insert(*i) => {
                    let held = self.defs[*i].item.each_field();
                    todo.extend(held.map(|f| (f.ty, f.name)));
                }
                _ => {}
            }
            todo[next..].reverse();
        }
        None
    }

    /// The message enum of `update`, when it is known.
    fn update(&mut self, n: &Name, state: Option<Ty>, update: Option<usize>) -> Res<Option<usize>> {
        self.named(n.at)?;
        let defs = self.defs;
        let Some(Item::Fn(sig)) = update.map(|i| &defs[i].item) else {
            return Ok(None);
        };
        let last = sig.params.last();
        if last.is_some_and(|p| matches!(self.types.node(p.ty), Node::Var('_'))) {
            return Ok(None);
        }
        let message = last.and_then(|p| match self.types.node(p.ty) {
            Node::User(i) if matches!(defs[*i].item, Item::Enum(_)) => Some(*i),
            _ => None,
        });
        let first = match (state, &sig.params[..]) {
            (Some(s), [p, _]) => p.inout && !p.labelled && self.types.fits(p.ty, s),
            (None, [_]) => true,
            _ => false,
        };
        let result = message
            .zip(last)
            .zip(sig.result)
            .is_some_and(|((_, p), r)| {
                let effect = self.types.named("Effect", &[p.ty]);
                let want = self.types.named("List", &[effect]);
                self.types.fits(r, want)
            });
        if first && result && last.is_some_and(|p| !p.inout && !p.labelled) {
            return Ok(message);
        }
        let name = message.map_or("Msg", |i| self.defs.name(i));
        let head = state.map_or(String::new(), |s| {
            format!("s: inout {}, ", self.types.show(s))
        });
        let text = format!("update must be fn({head}m: {name}) -> List<Effect<{name}>>");
        Err(self.fail((n.at, n.text.len()), "bad-update-signature", text))
    }

    fn start(&self, e: &Expr, message: Option<usize>) -> Res {
        self.named(e.at)?;
        let Some(en) = message else { return Ok(()) };
        let name = self.defs.name(en);
        let variant = match self.resolved.get(e.at) {
            Some(Binding::Variant(of, v)) if of == en && matches!(e.kind, ExprKind::Name(_)) => v,
            _ => {
                let text = format!("`{}` is not a variant of `{name}`", render(e));
                return Err(self.fail((e.at, e.len), "bad-start", text));
            }
        };
        let Item::Enum(variants) = &self.defs[en].item else {
            return Ok(());
        };
        let text = |f: &Field| match self.types.node(f.ty) {
            Node::Name("String", _) | Node::Var('_') => None,
            _ => Some(format!(
                "`{}` field `{}` is not a String; the first message is filled from the arguments",
                variants[variant].name, f.name
            )),
        };
        match variants[variant].fields.iter().find_map(text) {
            Some(text) => Err(self.fail((e.at, e.len), "bad-start", text)),
            None => Ok(()),
        }
    }

    fn grants(&self, names: &[Name]) -> Res {
        for (i, g) in names.iter().enumerate() {
            self.named(g.at)?;
            if names[..i].iter().any(|p| p.text == g.text) {
                let message = format!("`{}` is listed twice", g.text);
                return Err(self.fail((g.at, g.text.len()), "duplicate-grant", message));
            }
        }
        Ok(())
    }
}

/// The position and spelling of an entry's key.
fn key(entry: &Entry) -> (Pos, &'static str) {
    match entry {
        Entry::State(at, _) => (*at, "state"),
        Entry::Start(at, _) => (*at, "start"),
        Entry::Update(at, _) => (*at, "update"),
        Entry::Grants(at, _) => (*at, "grants"),
    }
}
