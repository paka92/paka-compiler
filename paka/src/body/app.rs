//! An app definition (docs/decisions/M2-15.md, M5-01.md): the keys, then `state`, `update`,
//! `start`, `view` and `grants` in that order, whatever the order written. A name the resolver could not bind is
//! reported at its turn; a type it could not read leaves the rules that need it silent.

use super::text::render;
use super::{Cx, Res};
use crate::ast::{Entry, Expr, ExprKind, Name, Pos};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Field, Item, Node, Ty};
use std::collections::BTreeSet;

const ORDER: [&str; 5] = ["state", "update", "start", "view", "grants"];
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
        let visual = entries.iter().any(|e| key(e).1 == "view");
        self.app_keys(app, entries, visual)?;
        let mut ordered: Vec<_> = entries.iter().collect();
        ordered.sort_by_key(|e| ORDER.iter().position(|&k| k == key(e).1));
        let mut message = None;
        for entry in ordered {
            match entry {
                Entry::State(_, n) => self.state(n, state)?,
                Entry::Update(_, n) => message = self.update(n, state, update, visual)?,
                Entry::Start(_, e) => self.start(e, message)?,
                Entry::View(_, n) => self.app_view(n, state, message)?,
                Entry::Grants(_, names) => self.grants(names)?,
            }
        }
        Ok(())
    }

    /// The app's view takes the state and gives `View<M>`, M being `update`'s message enum.
    fn app_view(&mut self, n: &Name, state: Option<Ty>, message: Option<usize>) -> Res {
        self.named(n.at)?;
        let defs = self.defs;
        let Some(Binding::Def(i)) = self.resolved.get(n.at) else {
            return Ok(());
        };
        let Item::View(sig) = &defs[i].item else {
            return Ok(());
        };
        let Some(state) = state else {
            let message = "a visual app needs state:, the type its view takes";
            return Err(self.fail((n.at, n.text.len()), "bad-view-signature", message));
        };
        let msg = message.and_then(|m| self.types.user(m));
        let takes = matches!(&sig.params[..], [p] if !p.labelled && self.types.fits(p.ty, state));
        let gives = match (msg, sig.result) {
            (Some(m), Some(r)) => {
                let want = self.types.named("View", &[m]);
                self.types.fits(r, want)
            }
            _ => true,
        };
        if takes && gives {
            return Ok(());
        }
        let msg = message.map_or("Msg", |m| self.defs.name(m));
        let text = format!(
            "view must be view(s: {}) -> View<{msg}>",
            self.types.show(state)
        );
        Err(self.fail((n.at, n.text.len()), "bad-view-signature", text))
    }

    fn app_keys(&self, app: &Name, entries: &[Entry], visual: bool) -> Res {
        for (i, entry) in entries.iter().enumerate() {
            let (at, name) = key(entry);
            if entries[..i].iter().any(|e| key(e).1 == name) {
                let message = format!("`{name}` is written twice");
                return Err(self.fail((at, name.len()), "duplicate-app-entry", message));
            }
        }
        for (code, name) in MISSING {
            let optional = visual && name == "start";
            if !optional && !entries.iter().any(|e| key(e).1 == name) {
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

    /// The message enum of `update`, when it is known. In a visual app it may have no result.
    fn update(
        &mut self,
        n: &Name,
        state: Option<Ty>,
        update: Option<usize>,
        visual: bool,
    ) -> Res<Option<usize>> {
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
        let result = message.zip(last).is_some_and(|(_, p)| match sig.result {
            Some(r) => {
                let effect = self.types.named("Effect", &[p.ty]);
                let want = self.types.named("List", &[effect]);
                self.types.fits(r, want)
            }
            None => visual,
        });
        if first && result && last.is_some_and(|p| !p.inout && !p.labelled) {
            return Ok(message);
        }
        let name = message.map_or("Msg", |i| self.defs.name(i));
        let head = state.map_or(String::new(), |s| {
            format!("s: inout {}, ", self.types.show(s))
        });
        let either = if visual {
            ", or give no result".to_string()
        } else {
            String::new()
        };
        let text = format!("update must be fn({head}m: {name}) -> List<Effect<{name}>>{either}");
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
        Entry::View(at, _) => (*at, "view"),
    }
}
