//! Holes: `??name` stands for a value the program does not have yet.

use super::{Cx, Out, Res, span};
use crate::ast::{Expr, ExprKind as K};
use crate::diagnostic::Diagnostic;
use crate::sig::Ty;

impl Cx<'_, '_> {
    /// A hole takes the type asked for; without one it has no type.
    pub(super) fn hole(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        match want {
            Some(w) if !self.free(w) => Ok(Out::Val(w)),
            _ => Err(self.cannot_infer(e)),
        }
    }

    fn cannot_infer(&self, e: &Expr) -> Diagnostic {
        let K::Hole(name) = e.kind else {
            unreachable!("a hole")
        };
        let message =
            format!("cannot infer the type of ??{name}; write it: let {name}: T = ??{name}");
        self.fail_at(e, "cannot-infer", message)
    }

    /// A hole left of an operator takes the type of the right operand; gives both types.
    pub(super) fn beside_hole(&mut self, hole: &Expr, right: &Expr) -> Res<(Ty, Ty)> {
        if matches!(right.kind, K::Hole(_)) {
            return Err(self.cannot_infer(hole));
        }
        let ty = self.expr_val(right, None)?;
        if self.free(ty) {
            return Err(self.cannot_infer(hole));
        }
        self.info.insert(span(hole), ty);
        Ok((ty, ty))
    }
}
