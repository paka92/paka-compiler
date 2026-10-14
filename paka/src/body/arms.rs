//! `match`: the variants it must cover, the arms and the type they give.

use super::pattern::{Key, Seen, Space};
use super::{Cx, Out, Res, Use, span};
use crate::ast::{Arm, ArmBody, Expr, ExprKind as K, Stmt, StmtKind as S};
use crate::sig::Ty;

impl Cx<'_, '_> {
    /// `match` as a statement or as a value. A missing arm is reported first, at the `match`.
    pub(super) fn match_expr(&mut self, e: &Expr, used: Use) -> Res<Out> {
        let K::Match { subject, arms } = &e.kind else {
            unreachable!("a match")
        };
        let ty = self.expr_val(subject, None)?;
        let Some(sp) = self.space(ty) else {
            let want = "Int, String, Bool or an enum";
            return Err(self.mismatch(span(subject), want, &self.types.show(ty), ""));
        };
        self.covers(e, sp, ty, arms)?;
        let mut seen = Seen::new();
        let (mut used, mut all_leave) = (used, true);
        for arm in arms {
            for p in &arm.patterns {
                self.pattern(sp, ty, p, &mut seen)?;
            }
            let out = self.arm(&arm.body, used)?;
            all_leave &= matches!(out, Out::Never);
            if let (Out::Val(t), Use::Value(acc)) = (out, used)
                && acc.is_none_or(|a| self.free(a))
            {
                used = Use::Value(Some(t));
            }
        }
        Ok(match used {
            _ if all_leave => Out::Never,
            Use::Value(Some(t)) => Out::Val(t),
            Use::Value(None) => Out::Val(self.unknown()),
            Use::Dropped => Out::Unit,
        })
    }

    /// Every value of the subject has an arm. Anything the arms get wrong is left to them.
    fn covers(&self, e: &Expr, sp: Space, ty: Ty, arms: &[Arm]) -> Res {
        let mut seen = Seen::new();
        for p in arms.iter().flat_map(|a| &a.patterns) {
            match self.pat_key(sp, ty, p) {
                Ok(Some(k)) => drop(seen.insert(k)),
                _ => return Ok(()),
            }
        }
        if seen.contains(&Key::Wild) {
            return Ok(());
        }
        let gap = match sp {
            Space::Open(t) => format!("other {t} values; add _ => ..."),
            _ => {
                let names = self.names(sp);
                let gaps = names.iter().enumerate();
                let gaps: Vec<_> = gaps
                    .filter(|(i, _)| !seen.contains(&Key::Variant(*i)))
                    .map(|(_, n)| *n)
                    .collect();
                let noun = match (sp, gaps.len()) {
                    (_, 0) => return Ok(()),
                    (Space::Bool, _) => "",
                    (_, 1) => "variant ",
                    _ => "variants ",
                };
                format!("{noun}{}", gaps.join(", "))
            }
        };
        Err(self.fail((e.at, 5), "missing-arm", format!("no arm for {gap}")))
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
            (Out::Unit, Use::Value(want)) => {
                let of = want.map_or(String::new(), |w| {
                    format!(" of type {}", self.types.show(w))
                });
                let message = format!("this arm must give a value{of}");
                Err(self.fail((line.at, line.len), "missing-value", message))
            }
            _ => Ok(out),
        }
    }
}
