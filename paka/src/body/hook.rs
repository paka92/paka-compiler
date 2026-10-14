//! The hook of the pattern checker (M2-20): `match`, patterns, holes and examples. Until it is
//! filled in, a `match` checks its subject alone and a hole takes the type asked for.

use super::{Cx, Out, Res, Use};
use crate::ast::Expr;

impl Cx<'_, '_> {
    pub(super) fn hole(&mut self, used: Use) -> Out {
        match used {
            Use::Dropped => Out::Unit,
            Use::Value(want) => Out::Val(want.unwrap_or_else(|| self.letter('T'))),
        }
    }

    pub(super) fn match_expr(&mut self, subject: &Expr, used: Use) -> Res<Out> {
        self.expr_val(subject, None)?;
        Ok(match used {
            Use::Dropped => Out::Unit,
            Use::Value(want) => Out::Val(want.unwrap_or_else(|| self.unknown())),
        })
    }
}
