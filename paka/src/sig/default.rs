//! The default of a record field (M2-02 section 1.9): a constant of the field's type. Its form is
//! checked here and its outermost type; what a list, a map, a variant or a record holds is left to
//! the expression checker (M2-19).

use super::{Convert, NOT_CONSTANT, Node, Res, Ty};
use crate::ast::{self, Def, Expr, ExprKind, StrPart, UnOp};
use crate::resolve::Binding;

impl<'a> Convert<'_, 'a> {
    pub(super) fn default(&mut self, field: &str, want: Ty, e: &Expr<'a>) -> Res {
        if let Some(bad) = self.non_constant(e) {
            return Err(self.error(bad.at, bad.len, "unexpected-token", NOT_CONSTANT));
        }
        let found = self.head(e);
        if self.types.fits(found, want) {
            return Ok(());
        }
        let (found, want) = (self.types.show(found), self.types.show(want));
        let message = format!("the default of {field} is {found}, not {want}");
        Err(self
            .error(e.at, e.len, "type-mismatch", message)
            .with_types(&want, &found))
    }

    /// The enum a name or callee is a variant of and how many fields the variant has.
    fn variant(&self, e: &Expr) -> Option<(Ty, usize)> {
        match self.resolved.get(e.at)? {
            Binding::Variant(i, j) => {
                let Def::Enum { variants, .. } = &self.program.defs[i] else {
                    return None;
                };
                Some((self.types.user(i), variants[j].fields.len()))
            }
            Binding::Builtin(b) => {
                let sig = self.variants.get(b)?;
                Some((sig.result?, sig.params.len()))
            }
            _ => None,
        }
    }

    /// The first part of `e` that is not a constant.
    fn non_constant<'e>(&self, e: &'e Expr<'a>) -> Option<&'e Expr<'a>> {
        let args = |args: &'e [ast::Arg<'a>]| args.iter().find_map(|a| self.non_constant(&a.value));
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) => None,
            ExprKind::Str(parts) if parts.iter().all(|p| matches!(p, StrPart::Text(_))) => None,
            ExprKind::Unary(UnOp::Neg, x)
                if matches!(x.kind, ExprKind::Int(_) | ExprKind::Float(_)) =>
            {
                None
            }
            ExprKind::List(items) => items.iter().find_map(|x| self.non_constant(x)),
            ExprKind::Map(pairs) => pairs
                .iter()
                .find_map(|(k, v)| self.non_constant(k).or_else(|| self.non_constant(v))),
            ExprKind::Name(_) => self
                .variant(e)
                .filter(|v| v.1 == 0)
                .map_or(Some(e), |_| None),
            ExprKind::Record(_, fields) => args(fields),
            ExprKind::Call(callee, fields) if self.variant(callee).is_some() => args(fields),
            _ => Some(e),
        }
    }

    /// The outermost type of a constant; a part it does not decide is a variable.
    fn head(&mut self, e: &Expr<'a>) -> Ty {
        let any = self.types.intern(Node::Var('_'));
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
            ExprKind::Name(_) => self.variant(e).map_or(any, |v| v.0),
            ExprKind::Call(callee, _) => self.variant(callee).map_or(any, |v| v.0),
            ExprKind::Record(..) => match self.resolved.get(e.at) {
                Some(Binding::Def(i)) => self.types.user(i),
                _ => any,
            },
            _ => any,
        }
    }
}
