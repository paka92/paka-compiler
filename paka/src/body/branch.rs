//! Branches that give one value, in `if`, `match` and the items of a list or a map: the first
//! branch with a type of its own gives it to the others, whatever the order. Branches that are
//! all open share the type they join to.

use super::{Cx, Out, Res, Use, span};
use crate::ast::{Expr, ExprKind as K};
use crate::sig::Ty;

pub(super) struct Joined {
    /// The type the branches share, when one is known.
    pub ty: Option<Ty>,
    /// Every branch leaves control.
    pub leaves: bool,
}

impl Cx<'_, '_> {
    /// Checks the branches of `e` in order, `ends` being the value that ends each. `run(cx, i,
    /// used, again)` checks branch `i`; `again` is set when a branch before the one that gave the
    /// type is checked a second time.
    pub(super) fn branches(
        &mut self,
        e: &Expr,
        ends: &[Option<&Expr>],
        used: Use,
        mut run: impl FnMut(&mut Self, usize, Use, bool) -> Res<Out>,
    ) -> Res<Joined> {
        let mut carrier = used.want().filter(|&w| !self.free(w));
        let (mut exp, mut open, mut quiet, mut leaves) = (used, Vec::new(), None, true);
        let mut joined: Option<Ty> = None;
        for (i, end) in ends.iter().enumerate() {
            let out = run(self, i, exp, false)?;
            leaves &= matches!(out, Out::Never);
            let Out::Val(t) = out else { continue };
            let silent = self.is_var(t) && !self.free(t);
            if silent {
                quiet.get_or_insert(t);
            }
            if carrier.is_some() || silent {
                continue;
            }
            if self.free(t) {
                joined = Some(match joined {
                    None => t,
                    Some(j) => {
                        (self.join(j, t)).ok_or_else(|| self.mismatch_at(end.unwrap_or(e), j, t))?
                    }
                });
                open.push(i);
                continue;
            }
            carrier = Some(t);
            exp = Use::Value(carrier);
            for j in open.drain(..) {
                run(self, j, exp, true)?;
            }
        }
        if carrier.is_none() && joined.is_some() {
            let hole = self.needs(e);
            if matches!(hole.kind, K::Hole(_)) {
                return Err(self.cannot_infer(hole));
            }
        }
        if let (None, Some(j)) = (carrier.or(quiet), joined) {
            for x in open.iter().filter_map(|&i| ends[i]) {
                self.info.insert(span(x), j);
            }
        }
        Ok(Joined {
            ty: carrier.or(quiet).or(joined),
            leaves,
        })
    }

    /// The type shared by `n` items of a list or a map, `hint` when there are none. Each item is
    /// checked against the hint until one has a type of its own, then against that type. The first
    /// of Map `keys` must be an Int or a String before the others are checked against it.
    pub(super) fn items<'x>(
        &mut self,
        e: &Expr,
        n: usize,
        keys: bool,
        item: impl Fn(usize) -> &'x Expr<'x>,
        hint: Option<Ty>,
    ) -> Res<Option<Ty>> {
        let ends: Vec<_> = (0..n).map(|i| Some(item(i))).collect();
        let fixed = hint.filter(|&h| !self.free(h));
        let joined = self.branches(e, &ends, Use::Value(fixed), |cx, i, u, _| {
            let out = match (u.want(), hint) {
                (None, Some(h)) => {
                    let out = cx.expr(item(i), Some(h))?;
                    cx.val(out, item(i)).map(|_| out)
                }
                (want, _) => cx.tail(item(i), want),
            }?;
            if let (true, 0, Out::Val(ty)) = (keys, i, out) {
                cx.map_key(span(item(0)), ty)?;
            }
            Ok(out)
        })?;
        let ty = joined.ty.or(hint);
        // An item that never returns takes the item type.
        if let Some(t) = ty {
            for i in 0..n {
                self.info.entry(span(item(i))).or_insert(t);
            }
        }
        Ok(ty)
    }
}
