//! Branches that give one value, in `if`, `match` and the items of a list or a map: the first
//! branch with a type of its own gives it to the others, whatever the order.

use super::{Cx, Out, Res, Use};
use crate::ast::{Expr, ExprKind as K};
use crate::sig::Ty;

pub(super) struct Joined {
    /// The type the branches share, when one is known.
    pub ty: Option<Ty>,
    /// Every branch leaves control.
    pub leaves: bool,
}

impl Cx<'_, '_> {
    /// Checks the `n` branches of `e` in order. `run(cx, i, used, again)` checks branch `i`;
    /// `again` is set when a branch before the one that gave the type is checked a second time.
    pub(super) fn branches(
        &mut self,
        e: &Expr,
        n: usize,
        used: Use,
        mut run: impl FnMut(&mut Self, usize, Use, bool) -> Res<Out>,
    ) -> Res<Joined> {
        let mut carrier = used.want().filter(|&w| !self.free(w));
        let (mut exp, mut open, mut first, mut leaves) = (used, Vec::new(), None, true);
        for i in 0..n {
            let out = run(self, i, exp, false)?;
            leaves &= matches!(out, Out::Never);
            let Out::Val(t) = out else { continue };
            first.get_or_insert(t);
            let silent = self.is_var(t) && !self.free(t);
            if carrier.is_some() || silent {
                continue;
            }
            if self.free(t) {
                open.push(i);
                continue;
            }
            carrier = Some(t);
            exp = Use::Value(carrier);
            for j in open.drain(..) {
                run(self, j, exp, true)?;
            }
        }
        if carrier.is_none() && first.is_some() {
            let hole = self.needs(e);
            if matches!(hole.kind, K::Hole(_)) {
                return Err(self.cannot_infer(hole));
            }
        }
        Ok(Joined {
            ty: carrier.or(first),
            leaves,
        })
    }

    /// The type shared by `n` items of a list or a map, `hint` when there are none. The hint is
    /// what the first item is typed with; the whole decides whether it holds.
    pub(super) fn items<'x>(
        &mut self,
        e: &Expr,
        n: usize,
        item: impl Fn(usize) -> &'x Expr<'x>,
        hint: Option<Ty>,
    ) -> Res<Option<Ty>> {
        let joined = self.branches(e, n, Use::Value(None), |cx, i, u, _| {
            match (u.want(), hint) {
                (None, Some(h)) => {
                    let out = cx.expr(item(i), Some(h))?;
                    cx.val(out, item(i)).map(|_| out)
                }
                (want, _) => cx.tail(item(i), want),
            }
        })?;
        Ok(joined.ty.or(hint))
    }
}
