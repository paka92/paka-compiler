//! What may stand left of an assignment or after `&`.

use super::text::{assign_op, fit, render};
use super::{Cx, How, Res, Ty, span};
use crate::ast::{AssignOp, Expr, ExprKind as K};
use crate::resolve::Binding;

/// Why an expression is not a place.
enum Why<'x> {
    NotAPlace,
    Slice,
    /// An item of a String.
    Text,
    /// The root name is a `let`, a parameter or a `for` name.
    Fixed(&'x Expr<'x>, How, Ty),
}

fn kind(how: How) -> &'static str {
    match how {
        How::Param => "a parameter",
        How::For => "a for name",
        How::Pattern => "a pattern name",
        _ => "a let",
    }
}

impl Cx<'_, '_> {
    /// A place is a `var` or `inout` name, or a field or list or map element of one.
    fn place<'x>(&self, e: &'x Expr<'x>) -> Result<(), Why<'x>> {
        let mut cur = e;
        loop {
            match &cur.kind {
                K::Field(x, _) => cur = x,
                K::Index(_, i) if matches!(i.kind, K::Range(..)) => return Err(Why::Slice),
                K::Index(x, _) => {
                    let text = self
                        .info
                        .get(&(x.at, x.len))
                        .is_some_and(|&t| self.is(t, "String"));
                    if text {
                        return Err(Why::Text);
                    }
                    cur = x;
                }
                K::Name(_) => break,
                _ => return Err(Why::NotAPlace),
            }
        }
        let Some(Binding::Local(decl)) = self.resolved.get(cur.at) else {
            return Err(Why::NotAPlace);
        };
        match self.locals.get(&decl) {
            Some(l) if matches!(l.how, How::Var | How::Inout) => Ok(()),
            Some(l) => Err(Why::Fixed(cur, l.how, l.ty)),
            None => Err(Why::NotAPlace),
        }
    }

    /// Whether `e` may stand after `&` or left of an assignment.
    pub(super) fn is_target(&self, e: &Expr) -> bool {
        self.place(e).is_ok()
    }

    pub(super) fn inout_target(&self, x: &Expr) -> Res {
        let Err(why) = self.place(x) else {
            return Ok(());
        };
        let message = match why {
            Why::Fixed(root, How::Param, ty) => {
                let (name, f) = (render(root), self.program.defs[self.def].name().text);
                format!("{name} is read-only; write {name}: inout {} and call {f}(&ys)", self.types.show(ty))
            }
            Why::Fixed(root, how @ (How::For | How::Pattern), _) => {
                format!("{} is {}; it is a copy", render(root), kind(how))
            }
            Why::Fixed(root, ..) => format!("{} is a let; declare it with var", render(root)),
            Why::Slice => "a slice cannot be passed with &; pass the list and the positions".to_string(),
            Why::Text => "a String position is not a place".to_string(),
            Why::NotAPlace => {
                "only a var local, an inout parameter or their fields and items can be passed with &".to_string()
            }
        };
        Err(self.fail_at(x, "bad-inout-target", message))
    }

    /// The place a `for` name walks, as written, when it walks one.
    fn walked(&self, name: &Expr) -> Option<&String> {
        let Some(Binding::Local(decl)) = self.resolved.get(name.at) else {
            return None;
        };
        self.walks.get(&decl)
    }

    /// Type of the target, then what it is, then the value.
    pub(super) fn assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> Res {
        self.writing = Some(target.at);
        let ty = self.expr_val(target, None);
        self.writing = None;
        let (ty, at) = (ty?, span(target));
        match op {
            AssignOp::Set => {}
            AssignOp::Add => {
                self.allow(at, ty, &["Int", "Float", "String"], "Int, Float or String")?
            }
            AssignOp::Sub | AssignOp::Mul | AssignOp::Div => {
                self.allow(at, ty, &["Int", "Float"], "Int or Float")?
            }
            AssignOp::Rem => self.allow(at, ty, &["Int"], "Int")?,
        }
        match self.place(target) {
            Err(Why::Fixed(root, how, _)) => {
                let name = render(root);
                let message = match (self.walked(root), fit(target), fit(value)) {
                    (Some(w), Some(t), Some(v)) => {
                        let (rest, op) = (&t[name.len()..], assign_op(op));
                        format!(
                            "{name} is a for name; write {w}[i]{rest} {op} {v} walking by position"
                        )
                    }
                    _ => format!("{name} is {}; declare it with var", kind(how)),
                };
                return Err(self.fail_at(root, "assign-to-let", message));
            }
            Err(Why::Text) => {
                return Err(self.fail(
                    at,
                    "bad-assignment-target",
                    "a String changes only as a whole",
                ));
            }
            Err(_) => {
                let message = "only a var local, an inout parameter or their fields and items can be assigned";
                return Err(self.fail(at, "bad-assignment-target", message));
            }
            Ok(()) => {}
        }
        self.expect(value, ty).map(drop)
    }
}
