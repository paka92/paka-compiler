//! The default of a record field (M2-02 section 1.9): a constant of the field's type, which the
//! parser has read as one. Only its outermost type is checked here; what a variant or a record
//! holds, and whether a variant lacks fields, is left to the expression checker (M2-19).

use super::{Convert, Node, Ty, earliest};
use crate::ast::{Expr, ExprKind};
use crate::resolve::Binding;

impl<'a> Convert<'_, 'a> {
    pub(super) fn default(&mut self, field: &str, want: Ty, e: &Expr<'a>) {
        if matches!(self.types.node(want), Node::Fn(..)) {
            return;
        }
        let found = self.head(e);
        if self.types.fits(found, want) {
            return;
        }
        let (found, want) = (self.types.show(found), self.types.show(want));
        let message = format!("the default of {field} is {found}, not {want}");
        let d = (self.error(e.at, e.len, "type-mismatch", message)).with_types(&want, &found);
        earliest(&mut self.error, d);
    }

    /// The enum a name or callee is a variant of.
    fn variant(&self, e: &Expr) -> Option<Ty> {
        match self.resolved.get(e.at)? {
            Binding::Variant(i, _) => self.types.user(i),
            Binding::Builtin(b) => self.variants.get(b)?.result,
            _ => None,
        }
    }

    /// The outermost type of a constant; a part it does not decide is a variable.
    fn head(&mut self, e: &Expr<'a>) -> Ty {
        let any = self.types.unknown();
        match &e.kind {
            ExprKind::Int(_) => self.types.named("Int", &[]),
            ExprKind::Float(_) => self.types.named("Float", &[]),
            ExprKind::Bool(_) => self.types.named("Bool", &[]),
            ExprKind::Str(_) => self.types.named("String", &[]),
            ExprKind::Unary(_, x) => self.head(x),
            ExprKind::List(_) => {
                let t = self.types.intern(Node::Var('T'));
                self.types.named("List", &[t])
            }
            ExprKind::Map(_) => {
                let (k, v) = (
                    self.types.intern(Node::Var('K')),
                    self.types.intern(Node::Var('V')),
                );
                self.types.named("Map", &[k, v])
            }
            ExprKind::Name(_) => self.variant(e).unwrap_or(any),
            ExprKind::Call(callee, _) => self.variant(callee).unwrap_or(any),
            ExprKind::Record(..) => match self.resolved.get(e.at) {
                Some(Binding::Def(i)) => self.types.user(i).unwrap_or(any),
                _ => any,
            },
            _ => any,
        }
    }
}
