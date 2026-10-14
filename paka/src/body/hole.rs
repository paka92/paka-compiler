//! Holes, `??name`, stand for a value the program does not have yet; they and other values whose
//! type stays open are `cannot-infer`.

use super::text::{called, plain};
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

    pub(super) fn cannot_infer(&self, e: &Expr) -> Diagnostic {
        let K::Hole(name) = e.kind else {
            unreachable!("a hole")
        };
        let message =
            format!("cannot infer the type of ??{name}; write it: let {name}: T = ??{name}");
        self.fail(plain(e), "cannot-infer", message)
    }

    /// `cannot-infer` at the part of `e` that needs the type, `e` being open where nothing will fix
    /// it. A hole names itself; else the text names the local `e` is the value of, or that part.
    pub(super) fn open_value(&self, e: &Expr, local: Option<(&str, &str, Ty)>) -> Diagnostic {
        let part = self.needs(e);
        if matches!(part.kind, K::Hole(_)) {
            return self.cannot_infer(part);
        }
        let (keyword, name, ty, what) = match local {
            Some((keyword, name, ty)) => (keyword, name, Some(ty), name.to_string()),
            None => (
                "let",
                "x",
                self.info.get(&span(part)).copied(),
                called(part),
            ),
        };
        let shown = ty.map_or("T".to_string(), |t| self.types.show(t));
        let message =
            format!("cannot infer the type of {what}; write it: {keyword} {name}: {shown} = ...");
        self.fail(plain(part), "cannot-infer", message)
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
