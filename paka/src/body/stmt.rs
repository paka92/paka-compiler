//! Blocks and statements, and the `if` that is both.

use super::{Cx, How, Local, Out, Res, Use};
use crate::ast::{Block, Expr, ExprKind as K, Name, Stmt, StmtKind as S, Type};
use crate::builtins;
use crate::resolve::Binding;
use crate::sig::{Node, Ty};

impl Cx<'_, '_> {
    /// `used` says how the last line counts: as the block's value or as nothing.
    pub(super) fn block(&mut self, b: &Block, used: Use) -> Res<Out> {
        let mut ended = false;
        for (i, s) in b.stmts.iter().enumerate() {
            if ended {
                return Err(self.fail(s.at, s.len, "unreachable-code", "nothing runs after this"));
            }
            ended = match (&s.kind, used) {
                (S::Expr(e), Use::Value(want)) if i + 1 == b.stmts.len() => {
                    return self.tail(e, want);
                }
                _ => matches!(self.stmt(s)?, Out::Never),
            };
        }
        match used {
            Use::Value(_) if !ended => Err(self.fail(
                b.close,
                1,
                "missing-value",
                "the block ends without a value",
            )),
            _ => Ok(if ended { Out::Never } else { Out::Unit }),
        }
    }

    fn tail(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        match want {
            Some(w) => self.expect_out(e, w),
            None => {
                let out = self.expr(e, None)?;
                self.val(out, e).map(|_| out)
            }
        }
    }

    fn stmt(&mut self, s: &Stmt) -> Res<Out> {
        match &s.kind {
            S::Let {
                var,
                name,
                ty,
                value,
            } => self
                .local(*var, name, ty.as_ref(), value)
                .map(|_| Out::Unit),
            S::Discard(e) => match self.expr(e, None)? {
                Out::Val(_) => Ok(Out::Unit),
                _ => Err(self.fail(
                    s.at,
                    1,
                    "nothing-to-discard",
                    "there is no value to discard",
                )),
            },
            S::Assign { target, op, value } => self.assign(target, *op, value).map(|_| Out::Unit),
            S::Return(value) => self.ret(s, value.as_ref()),
            S::Break | S::Continue if self.loops == 0 => Err(self.fail(
                s.at,
                s.len,
                "break-outside-loop",
                "this is not inside a loop",
            )),
            S::Break | S::Continue => Ok(Out::Never),
            S::For { name, list, body } => {
                let elem = self.elements(list)?;
                self.locals.insert(
                    name.at,
                    Local {
                        ty: elem,
                        how: How::For,
                    },
                );
                self.in_loop(body).map(|_| Out::Unit)
            }
            S::While { cond, body } => {
                let bool = self.prim("Bool");
                self.expect(cond, bool)?;
                self.in_loop(body).map(|_| Out::Unit)
            }
            S::Expr(e) => self.effect(e),
        }
    }

    /// An expression statement: its value, if any, may not be left unused.
    fn effect(&mut self, e: &Expr) -> Res<Out> {
        let out = match &e.kind {
            K::Hole(_) => self.hole(Use::Dropped),
            K::If {
                branches,
                otherwise,
            } => self.if_expr(e, branches, otherwise.as_ref(), Use::Dropped)?,
            K::Match { subject, .. } => self.match_expr(subject, Use::Dropped)?,
            _ => self.expr(e, None)?,
        };
        match out {
            Out::Val(_) => Err(self.fail_at(
                e,
                "unused-result",
                "the result is not used; write `_ =` to drop it",
            )),
            out => Ok(out),
        }
    }

    fn local(&mut self, var: bool, name: &Name, written: Option<&Type>, value: &Expr) -> Res {
        let ty = match written {
            Some(t) => {
                let ty = self.annotation(t)?;
                self.expect(value, ty)?
            }
            None => match self.expr(value, None)? {
                Out::Val(t) if !self.free(t) => t,
                Out::Unit => return Err(self.no_value(value, None)),
                _ => {
                    return Err(self.fail(
                        name.at,
                        name.text.len(),
                        "cannot-infer",
                        "the type needs an annotation",
                    ));
                }
            },
        };
        let how = if var { How::Var } else { How::Let };
        self.locals.insert(name.at, Local { ty, how });
        Ok(())
    }

    /// The element type of the list a `for` goes over.
    fn elements(&mut self, list: &Expr) -> Res<Ty> {
        let ty = self.expr_val(list, None)?;
        match (self.arg(ty, "List", 0), self.is_var(ty)) {
            (Some(elem), _) => Ok(elem),
            (None, true) => Ok(self.unknown()),
            (None, false) => Err(self.mismatch(list, "List", &self.types.show(ty), "")),
        }
    }

    fn in_loop(&mut self, body: &Block) -> Res {
        self.loops += 1;
        let done = self.block(body, Use::Dropped);
        self.loops -= 1;
        done.map(drop)
    }

    fn ret(&mut self, s: &Stmt, value: Option<&Expr>) -> Res<Out> {
        match (value, self.result) {
            (Some(v), Some(r)) => {
                self.expect(v, r)?;
                if self.tail == Some(s.at) {
                    return Err(self.fail(
                        s.at,
                        s.len,
                        "return-last",
                        "the last line is the result; drop `return`",
                    ));
                }
            }
            (Some(v), None) => {
                if let Out::Val(t) = self.expr(v, None)? {
                    return Err(self.mismatch(v, "no value", &self.types.show(t), ""));
                }
            }
            (None, Some(_)) => {
                return Err(self.fail(s.at, s.len, "missing-value", "this function gives a value"));
            }
            (None, None) => {}
        }
        Ok(Out::Never)
    }

    /// `if` with its `else if` branches; as a value, every branch fits the first one's type.
    pub(super) fn if_expr(
        &mut self,
        e: &Expr,
        branches: &[(Expr, Block)],
        otherwise: Option<&Block>,
        mut used: Use,
    ) -> Res<Out> {
        if matches!(used, Use::Value(_)) && otherwise.is_none() {
            return Err(self.fail_at(
                e,
                "if-needs-else",
                "an `if` that gives a value needs an `else`",
            ));
        }
        let bool = self.prim("Bool");
        let mut all_leave = true;
        for block in branches
            .iter()
            .map(|(c, b)| (Some(c), b))
            .chain(otherwise.map(|b| (None, b)))
        {
            if let (Some(cond), _) = block {
                self.expect(cond, bool)?;
            }
            let out = self.block(block.1, used)?;
            all_leave &= matches!(out, Out::Never);
            if let (Out::Val(t), Use::Value(acc)) = (out, used)
                && acc.is_none_or(|a| self.free(a))
            {
                used = Use::Value(Some(t));
            }
        }
        Ok(match used {
            Use::Value(Some(t)) if !all_leave => Out::Val(t),
            Use::Value(_) if !all_leave => Out::Val(self.unknown()),
            _ if all_leave && otherwise.is_some() => Out::Never,
            _ => Out::Unit,
        })
    }

    /// The type a written type names; a Map key is an Int or a String.
    fn annotation(&mut self, t: &Type) -> Res<Ty> {
        let node = match t {
            Type::Name { name, args } => match self.resolved.get(name.at) {
                Some(Binding::Def(i)) => {
                    return Ok(self.types.user(i).unwrap_or_else(|| self.unknown()));
                }
                Some(Binding::Builtin(b)) if args.len() == builtins::arity(b) => {
                    let tys = args
                        .iter()
                        .map(|a| self.annotation(a))
                        .collect::<Res<Vec<_>>>()?;
                    if b == "Map" {
                        self.key(&tys[0], &args[0])?;
                    }
                    Node::Name(b, tys.into())
                }
                _ => return Ok(self.unknown()),
            },
            Type::Fn { params, result, .. } => {
                let params = params
                    .iter()
                    .map(|p| self.annotation(p))
                    .collect::<Res<Vec<_>>>()?;
                let result = result.as_deref().map(|r| self.annotation(r)).transpose()?;
                Node::Fn(params.into(), result)
            }
        };
        Ok(self.types.intern(node))
    }

    fn key(&self, key: &Ty, written: &Type) -> Res {
        if self.is_var(*key) || self.is(*key, "Int") || self.is(*key, "String") {
            return Ok(());
        }
        let (at, len) = match written {
            Type::Name { name, .. } => (name.at, name.text.len()),
            Type::Fn { at, .. } => (*at, 2),
        };
        let found = self.types.show(*key);
        let d = self.fail(at, len, "type-mismatch", "a Map key is an Int or a String");
        Err(d.with_types(&"Int or String", &found))
    }
}
