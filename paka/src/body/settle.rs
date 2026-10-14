//! A type fixed after an expression was checked open is put into it and into each of its parts
//! still open, so the type map holds no letter that was fixed.

use super::call::Bound;
use super::stmt::values;
use super::{Cx, Res, span};
use crate::ast::{Arg, Expr, ExprKind as K};
use crate::resolve::Binding;
use crate::sig::Ty;

impl Cx<'_, '_> {
    /// `e` has the type `ty`; each open part is visited once, by a loop. A part that cannot be
    /// of the type its place gives it is a mismatch.
    pub(super) fn settle(&mut self, e: &Expr, ty: Ty) -> Res {
        let mut stack = vec![(e, ty)];
        while let Some((e, ty)) = stack.pop() {
            self.info.insert(span(e), ty);
            for (part, t) in self.parts(e, ty) {
                let Some(&had) = self.info.get(&span(part)).filter(|&&p| self.free(p)) else {
                    continue;
                };
                if !self.types.fits(had, t) {
                    return Err(self.mismatch_at(part, t, had));
                }
                if !self.free(t) {
                    stack.push((part, t));
                }
            }
        }
        Ok(())
    }

    /// The parts of `e` whose types follow from its type `ty`.
    fn parts<'x>(&mut self, e: &'x Expr<'x>, ty: Ty) -> Vec<(&'x Expr<'x>, Ty)> {
        match &e.kind {
            K::List(items) => match self.arg(ty, "List", 0) {
                Some(t) => items.iter().map(|x| (x, t)).collect(),
                None => Vec::new(),
            },
            K::Map(pairs) => match (self.arg(ty, "Map", 0), self.arg(ty, "Map", 1)) {
                (Some(k), Some(v)) => pairs.iter().flat_map(|(a, b)| [(a, k), (b, v)]).collect(),
                _ => Vec::new(),
            },
            K::Call(f, args) => self.arguments(f, args, ty),
            _ => values(e).into_iter().map(|x| (x, ty)).collect(),
        }
    }

    /// The arguments of a call of a built-in with one row, typed from the row bound by `ty` and
    /// by the arguments' own types.
    fn arguments<'x>(&mut self, f: &Expr, args: &'x [Arg<'x>], ty: Ty) -> Vec<(&'x Expr<'x>, Ty)> {
        let (fns, variants) = (self.fns, self.variants);
        let row = match (matches!(f.kind, K::Name(_)), self.resolved.get(f.at)) {
            (true, Some(Binding::Builtin(b))) => match (variants.get(b), fns.get(b)) {
                (Some(sig), _) => Some(sig),
                (_, Some(rows)) if rows.len() == 1 => rows.first(),
                _ => None,
            },
            _ => None,
        };
        let Some(sig) = row else {
            return Vec::new();
        };
        let mut bound = Bound::new();
        if let Some(r) = sig.result {
            self.bind(r, ty, &mut bound);
        }
        for (a, p) in args.iter().zip(&sig.params) {
            if let Some(&t) = self.info.get(&span(&a.value)) {
                self.bind(p.ty, t, &mut bound);
            }
        }
        let each = |(a, p): (&'x Arg<'x>, &crate::sig::Param)| (&a.value, self.subst(p.ty, &bound));
        args.iter().zip(&sig.params).map(each).collect()
    }
}
