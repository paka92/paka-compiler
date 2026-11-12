//! Views (docs/decisions/M5-01.md, M5-02.md, M5-04.md): elements and view calls stand only where a
//! view gives one, with the arguments in table order and `key:` last, keyed where M5-02 asks.

use super::call::Bound;
use super::text::render;
use super::{Cx, How, Local, Out, Res, Use, earliest, span};
use crate::ast::{self, Arg, Block, Expr, ExprKind as K, StmtKind as S, StrPart};
use crate::builtins::{self, Element};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Item, Param, Sig};
use paka_rt::ops::Int;

/// Where an element stands.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Place {
    /// What the view gives.
    Root,
    /// A line of a child block.
    Child,
    /// A line a `for` adds.
    Looped,
}

/// A literal key: `3` and `"3"` differ.
#[derive(PartialEq)]
enum Key {
    Int(Int),
    Str(String),
}

fn is_key(arg: &Arg) -> bool {
    arg.label.as_ref().is_some_and(|l| l.text == "key")
}

/// Whether `arg` is written for `p`: bare for a bare one, else by its label.
fn takes(arg: &Arg, p: &Param) -> bool {
    match &arg.label {
        Some(l) => p.labelled && l.text == p.name,
        None => !p.labelled,
    }
}

/// The key a line writes as a literal, if any.
fn literal_key<'x>(e: &'x Expr<'x>) -> Option<(&'x Expr<'x>, Key)> {
    let call = match &e.kind {
        K::Children(call, _) => call,
        _ => e,
    };
    let K::Call(_, args) = &call.kind else {
        return None;
    };
    let value = &args.last().filter(|a| is_key(a))?.value;
    let key = match &value.kind {
        K::Int(text) => Key::Int(Int::parse(text)?),
        K::Str(parts) => Key::Str(
            (parts.iter())
                .map(|p| match p {
                    StrPart::Text(t) => Some(t.as_str()),
                    StrPart::Expr(_) => None,
                })
                .collect::<Option<_>>()?,
        ),
        _ => return None,
    };
    Some((value, key))
}

impl<'a> Cx<'_, 'a> {
    pub(super) fn view(
        &mut self,
        sig: &Sig<'_>,
        params: &[ast::Param<'a>],
        body: &'a Block<'a>,
    ) -> Res {
        self.params(sig, params);
        self.view = sig.result.unwrap_or_else(|| self.unknown());
        (self.result, self.place) = (None, Place::Root);
        self.tails.clear();
        let typed = self.block(body, Use::Elements).err();
        earliest(typed, self.ownership(body)).map_or(Ok(()), Err)
    }

    /// The lines of a view block: `let`s, `for`s and elements.
    pub(super) fn children(&mut self, b: &Block) -> Res {
        let lines = b.stmts.iter().try_for_each(|s| match &s.kind {
            S::Expr(e) => self.element(e),
            S::For { name, list, body } => {
                let ty = self.elements(list)?;
                let how = How::For;
                self.locals.insert(name.at, Local { ty, how });
                self.within(Place::Looped, body)
            }
            _ => self.stmt(s).map(drop),
        });
        earliest(lines.err(), self.same_keys(b)).map_or(Ok(()), Err)
    }

    fn within(&mut self, place: Place, b: &Block) -> Res {
        let outer = std::mem::replace(&mut self.place, place);
        let done = self.children(b);
        self.place = outer;
        done
    }

    /// One element line: a hole, an `if` or `match` of element lines, or a call.
    pub(super) fn element(&mut self, e: &Expr) -> Res {
        match &e.kind {
            K::Hole(_) => {
                self.info.insert(span(e), self.view);
                Ok(())
            }
            K::If {
                otherwise: None, ..
            } if self.place == Place::Root => {
                let message = "an if used as a value needs an else";
                Err(self.fail((e.at, 2), "if-needs-else", message))
            }
            K::If {
                branches,
                otherwise,
            } => (self.if_expr(e, branches, otherwise.as_ref(), Use::Elements)).map(drop),
            K::Match { .. } => self.match_expr(e, Use::Elements).map(drop),
            K::Children(call, b) => self.placed(call, Some(b)),
            _ => self.placed(e, None),
        }
    }

    /// An element or view call with its child block, if written; anything else must give the
    /// view's type, which only these do.
    fn placed(&mut self, e: &Expr, block: Option<&Block>) -> Res {
        let (defs, view) = (self.defs, self.view);
        let called = match &e.kind {
            K::Call(callee, args) => match self.resolved.get(callee.at) {
                Some(Binding::Builtin(b)) => builtins::element(b).map(|el| (callee, args, Ok(el))),
                Some(Binding::Def(i)) => match &defs[i].item {
                    Item::View(sig) => Some((callee, args, Err(sig))),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((callee, args, what)) = called else {
            self.expect(e, view)?;
            return block.map_or(Ok(()), |_| Err(self.no_children(e)));
        };
        let el = what.ok();
        if el.is_some_and(|el| el.children) != block.is_some() {
            return Err(match block {
                Some(_) => self.no_children(callee),
                None => {
                    let (name, call) = (render(callee), render(e));
                    let message = format!("{name} takes a child block; write {call} {{ }}");
                    self.fail_at(callee, "wrong-child-block", message)
                }
            });
        }
        let (args, key) = self.split_key(args)?;
        self.key_place(callee, el, key)?;
        let sig = match what {
            Ok(el) => self.element_sig(el, args),
            Err(sig) => sig.clone(),
        };
        let out = self.call_sig(callee, &sig, args, Some(view))?;
        if let Out::Val(ty) = out
            && !self.types.fits(ty, view)
        {
            let (want, found) = (self.types.show(view), self.types.show(ty));
            return Err(self.mismatch(span(callee), &want, &found, ""));
        }
        self.note(e, &out);
        if el.is_some() {
            self.element_args(args)?;
        }
        if let Some(k) = key {
            self.key_type(&k.value)?;
        }
        block.map_or(Ok(()), |b| self.within(Place::Child, b))
    }

    fn no_children(&self, e: &Expr) -> Diagnostic {
        let message = format!("{} takes no child block", render(e));
        self.fail_at(e, "wrong-child-block", message)
    }

    /// The arguments without the `key:` written last, and that key.
    fn split_key<'x>(&self, args: &'x [Arg<'x>]) -> Res<(&'x [Arg<'x>], Option<&'x Arg<'x>>)> {
        let (rest, key) = match args.split_last() {
            Some((last, rest)) if is_key(last) => (rest, Some(last)),
            _ => (args, None),
        };
        match rest
            .iter()
            .find_map(|a| a.label.as_ref().filter(|l| l.text == "key"))
        {
            Some(l) => Err(self.fail((l.at, 3), "wrong-label", "key: is written last")),
            None => Ok((rest, key)),
        }
    }

    /// A key on stateful elements and on what a loop adds; none on the root (M5-02).
    fn key_place(&self, callee: &Expr, el: Option<&Element>, key: Option<&Arg>) -> Res {
        let name = render(callee);
        let needs = el.is_some_and(|el| el.needs_key);
        let (code, message) = match (self.place, key) {
            (Place::Root, Some(k)) => {
                let label = k.label.as_ref().expect("a key is labelled");
                let message = "the root of a view carries no key; drop key:";
                return Err(self.fail((label.at, 3), "key-on-root", message));
            }
            (Place::Root, None) if needs => (
                "key-on-root",
                format!(
                    "{name} needs a key, so it cannot be a view's root; put it in a Col or Row"
                ),
            ),
            (Place::Looped, None) => (
                "missing-key",
                format!("{name} is added by a for and needs a key; write key: last"),
            ),
            (_, None) if needs => (
                "missing-key",
                format!("{name} holds state and needs a key; write key: last"),
            ),
            _ => return Ok(()),
        };
        Err(self.fail_at(callee, code, message))
    }

    /// The element's parameters the written arguments stand for: one with a default is left out
    /// when no argument is written for it. `M` is the view's message type.
    fn element_sig(&mut self, el: &Element, args: &[Arg]) -> Sig<'static> {
        let table = &self.elements[el.name];
        let msg = (self.arg(self.view, "View", 0)).unwrap_or_else(|| self.unknown());
        let shown = Bound::from([('M', msg)]);
        let (mut next, mut params) = (0, Vec::new());
        for (i, (p, row)) in table.params.iter().zip(el.args).enumerate() {
            let default = row.2.is_some();
            let later = |a: &Arg| table.params[i + 1..].iter().any(|q| takes(a, q));
            match args.get(next) {
                Some(a) if takes(a, p) => next += 1,
                Some(a) if default && later(a) => continue,
                None if default => continue,
                _ => {}
            }
            let ty = self.subst(p.ty, &shown);
            params.push(Param { ty, ..p.clone() });
        }
        Sig {
            params,
            result: Some(self.view),
        }
    }

    /// A literal gap is not negative; `on_change:` names a variant (M5-04 section 3).
    fn element_args(&self, args: &[Arg]) -> Res {
        for arg in args {
            let (label, v) = (arg.label.as_ref().map(|l| l.text), &arg.value);
            match (label, &v.kind) {
                (Some("gap"), K::Int(text)) if text.starts_with('-') => {
                    let message = format!("{text} is not a gap; a gap is 0 or more");
                    return Err(self.fail_at(v, "literal-out-of-range", message));
                }
                (Some("on_change"), K::Name(_))
                    if matches!(self.resolved.get(v.at), Some(Binding::Variant(..))) => {}
                (Some("on_change"), _) => {
                    let message = format!(
                        "on_change: takes a variant with one String field, not {}",
                        render(v)
                    );
                    let d = self.fail_at(v, "type-mismatch", message);
                    return Err(d.with_types(&"a variant", &render(v)));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn key_type(&mut self, value: &Expr) -> Res {
        let ty = self.expr_val(value, None)?;
        if self.is_var(ty) || self.is(ty, "Int") || self.is(ty, "String") {
            return Ok(());
        }
        let d = self.fail_at(value, "type-mismatch", "a key is an Int or a String");
        Err(d.with_types(&"Int or String", &self.types.show(ty)))
    }

    /// The second line of `b` that writes a literal key an earlier line wrote.
    fn same_keys(&self, b: &Block) -> Option<Diagnostic> {
        let mut seen = Vec::new();
        for s in &b.stmts {
            let S::Expr(e) = &s.kind else { continue };
            let Some((value, key)) = literal_key(e) else {
                continue;
            };
            if seen.contains(&key) {
                let shown = match key {
                    Key::Int(n) => n.to_string(),
                    Key::Str(s) => format!("{s:?}"),
                };
                let message = format!("the key {shown} is given twice in this block");
                return Some(self.fail_at(value, "duplicate-key", message));
            }
            seen.push(key);
        }
        None
    }

    /// An element, a view or a child block where no element may stand.
    pub(super) fn outside_view(&self, e: &Expr) -> Diagnostic {
        let message = format!(
            "{} stands only where a view gives an element: its result or a line of a child block",
            render(e)
        );
        self.fail_at(e, "element-outside-view", message)
    }
}
