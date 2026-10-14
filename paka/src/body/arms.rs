//! `match`: the variants it must cover, the arms and the type they give.

use super::branch::Joined;
use super::pattern::{Key, Seen, Space};
use super::stmt::ends;
use super::text::{called, plain};
use super::{Cx, Out, Res, Use, span};
use crate::ast::{Arm, ArmBody, Expr, ExprKind as K, Stmt, StmtKind as S};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Item, Ty};

/// What each pattern of a match matches, in order, up to the first that does not fit.
type Keys<'p> = Vec<Res<Option<Key<'p>>>>;

/// The most variants a message names.
const NAMED: usize = 5;

impl Cx<'_, '_> {
    /// `match` as a statement or as a value. A missing arm is reported first, at the `match`.
    pub(super) fn match_expr(&mut self, e: &Expr, used: Use) -> Res<Out> {
        let K::Match { subject, arms } = &e.kind else {
            unreachable!("a match")
        };
        let ty = match self.expr_val(subject, None) {
            Ok(ty) => ty,
            Err(d) => return Err(self.uncovered(e, subject, arms).unwrap_or(d)),
        };
        if self.free(ty) {
            let part = self.needs(subject);
            if matches!(part.kind, K::Hole(_)) {
                return Err(self.cannot_infer(part));
            }
            let message = format!(
                "cannot infer the type of {}; match a value with a known type",
                called(part)
            );
            return Err(self.fail(plain(part), "cannot-infer", message));
        }
        let Some(sp) = self.space(ty) else {
            let want = "Int, String, Bool or an enum";
            return Err(self.mismatch(span(subject), want, &self.types.show(ty), ""));
        };
        let keys = self.keys(sp, ty, arms);
        self.covers(e, sp, &keys)?;
        let (mut keys, mut seen) = (keys.into_iter(), Seen::new());
        let joined = self.branches(e, &ends(e), used, |cx, i, u, again| {
            if !again {
                for p in &arms[i].patterns {
                    let key = keys.next().unwrap_or(Ok(None))?;
                    cx.pattern(sp, ty, p, key, &mut seen)?;
                }
            }
            cx.arm(&arms[i].body, u)
        })?;
        Ok(match (used, joined) {
            (_, Joined { leaves: true, .. }) => Out::Never,
            (Use::Value(_), Joined { ty, .. }) => Out::Val(ty.unwrap_or_else(|| self.unknown())),
            (Use::Dropped, _) => Out::Unit,
        })
    }

    fn keys<'p>(&self, sp: Space, ty: Ty, arms: &'p [Arm]) -> Keys<'p> {
        let mut keys = Vec::new();
        for p in arms.iter().flat_map(|a| &a.patterns) {
            let key = self.pat_key(sp, ty, p);
            let failed = key.is_err();
            keys.push(key);
            if failed {
                break;
            }
        }
        keys
    }

    /// The missing arm of a match whose subject has an error, when the subject's declared type
    /// shows it.
    fn uncovered(&mut self, e: &Expr, subject: &Expr, arms: &[Arm]) -> Option<Diagnostic> {
        let ty = self.declared(subject)?;
        let sp = self.space(ty)?;
        self.covers(e, sp, &self.keys(sp, ty, arms)).err()
    }

    /// The type a call is declared to give, whatever its arguments are.
    fn declared(&mut self, call: &Expr) -> Option<Ty> {
        let K::Call(callee, _) = &call.kind else {
            return None;
        };
        match self.resolved.get(callee.at)? {
            Binding::Def(i) => match &self.defs[i].item {
                Item::Fn(sig) => sig.result,
                _ => None,
            },
            Binding::Variant(i, _) => self.types.user(i),
            _ => None,
        }
    }

    /// Every value of the subject has an arm. Anything the arms get wrong is left to them.
    fn covers(&self, e: &Expr, sp: Space, keys: &Keys) -> Res {
        let mut seen = Seen::new();
        for key in keys {
            match key {
                Ok(Some(k)) => drop(seen.insert(*k)),
                _ => return Ok(()),
            }
        }
        if seen.contains(&Key::Wild) {
            return Ok(());
        }
        let gap = match sp {
            Space::Open(t) => format!("other {t} values; add _ => ..."),
            _ => match self.gaps(sp, &seen) {
                Some(gap) => gap,
                None => return Ok(()),
            },
        };
        Err(self.fail((e.at, 5), "missing-arm", format!("no arm for {gap}")))
    }

    /// The variants no arm has, and the arms to add; none when there are none.
    fn gaps(&self, sp: Space, seen: &Seen) -> Option<String> {
        let names = self.names(sp);
        let mut gaps = (names.iter().enumerate())
            .filter(|(i, _)| !seen.contains(&Key::Variant(*i)))
            .map(|(_, gap)| *gap);
        let shown: Vec<_> = gaps.by_ref().take(NAMED).collect();
        let more = gaps.count();
        let noun = match (sp, shown.len() + more) {
            (_, 0) => return None,
            (Space::Bool, _) => "",
            (_, 1) => "variant ",
            _ => "variants ",
        };
        let arm = |&(name, fields): &(&str, usize)| {
            let blanks = vec!["_"; fields].join(", ");
            let parts = if fields == 0 {
                String::new()
            } else {
                format!("({blanks})")
            };
            format!("{name}{parts} => ...")
        };
        let heads = shown.iter().map(|g| g.0).collect::<Vec<_>>().join(", ");
        let adds: Vec<_> = shown.iter().map(arm).collect();
        let (heads, adds) = match (adds.split_last(), more) {
            (_, 1..) => (
                format!("{heads} and {more} more"),
                format!("{} and {more} more", adds.join(", ")),
            ),
            (Some((last, rest)), 0) if !rest.is_empty() => {
                (heads, format!("{} and {last}", rest.join(", ")))
            }
            _ => (heads, adds.join(", ")),
        };
        Some(format!("{noun}{heads}; add {adds}"))
    }

    fn arm(&mut self, body: &ArmBody, used: Use) -> Res<Out> {
        let line = match (body, used) {
            (ArmBody::Block(b), _) => return self.block(b, used),
            (
                ArmBody::Line(Stmt {
                    kind: S::Expr(e), ..
                }),
                Use::Value(want),
            ) => return self.tail(e, want),
            (ArmBody::Line(s), _) => s,
        };
        let out = self.stmt(line)?;
        match (out, used) {
            (Out::Unit, Use::Value(_)) => {
                let message = "this arm must give a value";
                Err(self.fail((line.at, line.len), "missing-value", message))
            }
            _ => Ok(out),
        }
    }
}
