//! What may stand left of an assignment or after `&`.

use super::{Cx, Res};
use crate::ast::{AssignOp, Expr, ExprKind as K};

enum Why {
    NotAPlace,
    Immutable,
}

impl Cx<'_, '_> {
    /// A place is a `var` or `inout` name, or a field or list or map element of one.
    fn place(&self, e: &Expr) -> Result<(), Why> {
        let mut cur = e;
        loop {
            match &cur.kind {
                K::Field(x, _) => cur = x,
                K::Index(x, i) => {
                    let text = self
                        .info
                        .get(&(x.at, x.len))
                        .is_some_and(|&t| self.is(t, "String"));
                    if text || matches!(i.kind, K::Range(..)) {
                        return Err(Why::NotAPlace);
                    }
                    cur = x;
                }
                K::Name(_) => break,
                _ => return Err(Why::NotAPlace),
            }
        }
        match self.resolved.get(cur.at) {
            Some(crate::resolve::Binding::Local(decl)) => match self.locals.get(&decl) {
                Some(l) if matches!(l.how, super::How::Var | super::How::Inout) => Ok(()),
                _ => Err(Why::Immutable),
            },
            _ => Err(Why::NotAPlace),
        }
    }

    pub(super) fn inout_target(&self, x: &Expr) -> Res {
        self.place(x).map_err(|_| {
            self.fail_at(
                x,
                "bad-inout-target",
                "`&` takes a `var` or an `inout` name, or part of one",
            )
        })
    }

    /// Type of the target, then what it is, then the value.
    pub(super) fn assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> Res {
        let ty = self.expr_val(target, None)?;
        match op {
            AssignOp::Set => {}
            AssignOp::Add => self.allow(
                target,
                ty,
                &["Int", "Float", "String"],
                "Int, Float or String",
            )?,
            AssignOp::Sub | AssignOp::Mul | AssignOp::Div => {
                self.allow(target, ty, &["Int", "Float"], "Int or Float")?
            }
            AssignOp::Rem => self.allow(target, ty, &["Int"], "Int")?,
        }
        match self.place(target) {
            Err(Why::NotAPlace) => {
                return Err(self.fail_at(
                    target,
                    "bad-assignment-target",
                    "this cannot be assigned to",
                ));
            }
            Err(Why::Immutable) => {
                return Err(self.fail_at(target, "assign-to-let", "this is not a `var`"));
            }
            Ok(()) => {}
        }
        self.expect(value, ty).map(drop)
    }
}
