//! The hook of the pattern checker (M2-20): `match`, patterns, holes and examples. Until it is
//! filled in, a `match` checks its subject alone and a hole takes the type asked for.

use super::{Cx, Out, Res, Use};
use crate::ast::{Def, Expr, ExprKind as K};

impl Cx<'_, '_> {
    pub(super) fn hole(&mut self, used: Use) -> Out {
        match used {
            Use::Dropped => Out::Unit,
            Use::Value(want) => Out::Val(want.unwrap_or_else(|| self.letter('T'))),
        }
    }

    pub(super) fn match_expr(&mut self, e: &Expr, used: Use) -> Res<Out> {
        if let K::Match { subject, .. } = &e.kind {
            self.expr_val(subject, None)?;
        }
        Ok(match used {
            Use::Dropped => Out::Unit,
            Use::Value(want) => Out::Val(want.unwrap_or_else(|| self.unknown())),
        })
    }
}

// ---- Entry point of the example and app checks (M2-20, M2-32) --------------------------------

impl Cx<'_, '_> {
    /// Where the body of an example or the definitions of an app will be checked. Until then they
    /// are not looked at.
    pub(super) fn unchecked(&mut self, _def: &Def) -> Res {
        Ok(())
    }
}
