//! Blocks and statements, and the `if` that is both.

use super::branch::Joined;
use super::text::{called, render};
use super::{Cx, How, Local, Out, Res, Use, span};
use crate::ast::{Arm, ArmBody, Block, Expr, ExprKind as K, Name, Pos, Stmt, StmtKind as S, Type};
use crate::builtins;
use crate::resolve::Binding;
use crate::sig::{Node, Ty};
use std::collections::BTreeSet;

/// The `return`s that end a body: the last line, and the last lines of the branches and arms it
/// ends with.
pub(super) fn tails(body: &Block) -> BTreeSet<Pos> {
    let (mut found, mut stack) = (BTreeSet::new(), Vec::from_iter(body.stmts.last()));
    while let Some(s) = stack.pop() {
        match &s.kind {
            S::Return(_) => {
                found.insert(s.at);
            }
            S::Expr(Expr {
                kind:
                    K::If {
                        branches,
                        otherwise,
                    },
                ..
            }) => {
                let blocks = branches.iter().map(|(_, b)| b).chain(otherwise);
                stack.extend(blocks.filter_map(|b| b.stmts.last()));
            }
            S::Expr(Expr {
                kind: K::Match { arms, .. },
                ..
            }) => {
                stack.extend(arms.iter().filter_map(last_of));
            }
            _ => {}
        }
    }
    found
}

fn last_of<'x, 'a>(arm: &'x Arm<'a>) -> Option<&'x Stmt<'a>> {
    match &arm.body {
        ArmBody::Block(b) => b.stmts.last(),
        ArmBody::Line(s) => Some(s),
    }
}

/// The value that ends each branch of an `if` or arm of a `match`, in order, when it has one;
/// none for another expression.
pub(super) fn ends<'x>(e: &'x Expr<'x>) -> Vec<Option<&'x Expr<'x>>> {
    let lasts: Vec<_> = match &e.kind {
        K::If {
            branches,
            otherwise,
        } => (branches.iter().map(|(_, b)| b).chain(otherwise))
            .map(|b| b.stmts.last())
            .collect(),
        K::Match { arms, .. } => arms.iter().map(last_of).collect(),
        _ => return Vec::new(),
    };
    let value = |s: Option<&'x Stmt<'x>>| match &s?.kind {
        S::Expr(x) => Some(x),
        _ => None,
    };
    lasts.into_iter().map(value).collect()
}

pub(super) fn values<'x>(e: &'x Expr<'x>) -> Vec<&'x Expr<'x>> {
    ends(e).into_iter().flatten().collect()
}

impl Cx<'_, '_> {
    /// `used` says how the last line counts: as the block's value or as nothing.
    pub(super) fn block(&mut self, b: &Block, used: Use) -> Res<Out> {
        if let Use::Elements = used {
            return self.children(b).map(|()| Out::Unit);
        }
        let mut ended = false;
        for (i, s) in b.stmts.iter().enumerate() {
            if ended {
                return Err(self.fail(span_of(s), "unreachable-code", "nothing runs after this"));
            }
            ended = match (&s.kind, used) {
                (S::Expr(e), Use::Value(want)) if i + 1 == b.stmts.len() => {
                    return self.tail(e, want);
                }
                _ => matches!(self.stmt(s)?, Out::Never),
            };
        }
        match used {
            Use::Value(want) if !ended => {
                let of = want.map_or(String::new(), |w| {
                    format!(" of type {}", self.types.show(w))
                });
                let message = format!("this block must end in a value{of}");
                Err(self.fail((b.close, 1), "missing-value", message))
            }
            _ => Ok(if ended { Out::Never } else { Out::Unit }),
        }
    }

    /// `e` as a value: of type `want`, or of its own type; a hole stays open for the branches.
    pub(super) fn tail(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        match (want, &e.kind) {
            (Some(w), _) => self.expect_out(e, w),
            (None, K::Hole(_)) => {
                let out = Out::Val(self.letter('T'));
                self.note(e, &out);
                Ok(out)
            }
            (None, _) => {
                let out = self.expr(e, None)?;
                self.val(out, e).map(|_| out)
            }
        }
    }

    pub(super) fn stmt(&mut self, s: &Stmt) -> Res<Out> {
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
                Out::Val(t) if self.free(t) => Err(self.open_value(e, None)),
                Out::Val(_) => Ok(Out::Unit),
                _ => {
                    let message = format!("{} has no value: drop _ =", called(e));
                    Err(self.fail((s.at, 1), "nothing-to-discard", message))
                }
            },
            S::Assign { target, op, value } => self.assign(target, *op, value).map(|_| Out::Unit),
            S::Return(value) => self.ret(s, value.as_ref()),
            S::Break | S::Continue if self.loops == 0 => Err(self.fail(
                span_of(s),
                "break-outside-loop",
                "this is not inside a loop",
            )),
            S::Break | S::Continue => Ok(Out::Never),
            S::For { name, list, body } => {
                let elem = self.elements(list)?;
                if let Some(walk) = self.position_walk(list) {
                    self.walks.insert(name.at, walk);
                }
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
            K::Hole(_) => Out::Unit,
            K::If {
                branches,
                otherwise,
            } => self.if_expr(e, branches, otherwise.as_ref(), Use::Dropped)?,
            K::Match { .. } => self.match_expr(e, Use::Dropped)?,
            _ => self.expr(e, None)?,
        };
        match out {
            Out::Val(ty) => Err(self.unused(e, ty)),
            out => Ok(out),
        }
    }

    fn unused(&self, e: &Expr, ty: Ty) -> crate::diagnostic::Diagnostic {
        let (name, call) = (called(e), render(e));
        let message = match &e.kind {
            _ if self.is(ty, "Option") || self.is(ty, "Result") => format!(
                "the value of {name} is not used; it is {}: match it, pass it up with {call}?, or write `_ = {call}`",
                self.types.show(ty)
            ),
            K::Try(x, _) => format!("the value of {} is not used; write `_ = {call}`", called(x)),
            _ => format!("the value of {name} is not used; write _ = ..."),
        };
        self.fail_at(e, "unused-result", message)
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
                out => {
                    let ty = match out {
                        Out::Val(t) => t,
                        _ => self.letter('T'),
                    };
                    let keyword = if var { "var" } else { "let" };
                    return Err(self.open_value(value, Some((keyword, name.text, ty))));
                }
            },
        };
        let how = if var { How::Var } else { How::Let };
        self.locals.insert(name.at, Local { ty, how });
        Ok(())
    }

    pub(super) fn is_open(&self, e: &Expr) -> bool {
        self.info.get(&span(e)).is_some_and(|&t| self.free(t))
    }

    /// The part of `e` that lacks a type of its own.
    pub(super) fn needs<'x>(&self, e: &'x Expr<'x>) -> &'x Expr<'x> {
        self.needs_in([e], e)
    }

    /// The part of `roots` that lacks a type of its own: a hole if any, else the first part, the
    /// deepest, that is open and holds no open part; an open list or map literal is one part, its
    /// items searched only for a hole (M2-02 section 5). `fallback` when there is none.
    pub(super) fn needs_in<'x>(
        &self,
        roots: impl IntoIterator<Item = &'x Expr<'x>>,
        fallback: &'x Expr<'x>,
    ) -> &'x Expr<'x> {
        let mut stack: Vec<_> = roots.into_iter().collect();
        stack.reverse();
        let mut deepest = None;
        while let Some(e) = stack.pop() {
            if matches!(e.kind, K::Hole(_)) {
                return e;
            }
            let inner = self.open_parts(e);
            if inner.is_empty() || matches!(e.kind, K::List(_) | K::Map(_)) {
                deepest.get_or_insert(e);
            }
            stack.extend(inner.into_iter().rev());
        }
        deepest.unwrap_or(fallback)
    }

    /// The parts of `e` that are open: arguments, items, branches and operands.
    fn open_parts<'x>(&self, e: &'x Expr<'x>) -> Vec<&'x Expr<'x>> {
        let parts: Vec<&Expr> = match &e.kind {
            K::Call(_, args) => args.iter().map(|a| &a.value).collect(),
            K::List(items) => items.iter().collect(),
            K::Map(pairs) => pairs.iter().flat_map(|(k, v)| [k, v]).collect(),
            K::Unary(_, x) | K::Field(x, _) | K::Try(x, _) => vec![x],
            K::Binary(_, _, x, y) | K::Index(x, y) | K::Range(x, y) => vec![x, y],
            _ => values(e),
        };
        parts.into_iter().filter(|x| self.is_open(x)).collect()
    }

    /// The element type of the list a `for` goes over.
    pub(super) fn elements(&mut self, list: &Expr) -> Res<Ty> {
        let ty = self.expr_val(list, None)?;
        if self.free(ty) {
            return Err(self.open_value(list, None));
        }
        match (self.arg(ty, "List", 0), self.is_var(ty)) {
            (Some(elem), _) => Ok(elem),
            (None, true) => Ok(self.unknown()),
            (None, false) => {
                let fix = self.walk_fix(ty, list);
                Err(self.mismatch(span(list), "List", &self.types.show(ty), &fix))
            }
        }
    }

    /// The fix for a `for` over a String or a Map (M2-14 section 1.12).
    fn walk_fix(&self, ty: Ty, list: &Expr) -> String {
        match (self.is(ty, "String"), self.is(ty, "Map")) {
            (true, _) => format!("write chars({})", render(list)),
            (_, true) => format!("write keys({})", render(list)),
            _ => String::new(),
        }
    }

    fn in_loop(&mut self, body: &Block) -> Res {
        self.loops += 1;
        let done = self.block(body, Use::Dropped);
        self.loops -= 1;
        done.map(drop)
    }

    fn ret(&mut self, s: &Stmt, value: Option<&Expr>) -> Res<Out> {
        if self.tails.contains(&s.at) {
            let message = "return on the last line; drop return: the last line is the value";
            return Err(self.fail((s.at, "return".len()), "return-last", message));
        }
        match (value, self.result) {
            (Some(v), Some(r)) => {
                self.expect(v, r)?;
            }
            (Some(v), None) => {
                let name = self.program.defs[self.def].name().text;
                if matches!(v.kind, K::Hole(_)) {
                    let message = format!("{name} has no result; drop the value");
                    return Err(self.fail_at(v, "type-mismatch", message));
                }
                if let Out::Val(t) = self.expr(v, None)? {
                    let found = self.types.show(t);
                    let message =
                        format!("{name} has no result; drop the value or write -> {found}");
                    return Err(self
                        .fail_at(v, "type-mismatch", message)
                        .with_types(&"no value", &found));
                }
            }
            (None, Some(r)) => {
                let message = format!("return needs a value of type {}", self.types.show(r));
                return Err(self.fail(span_of(s), "missing-value", message));
            }
            (None, None) => {}
        }
        Ok(Out::Never)
    }

    /// `if` with its `else if` branches; as a value, the branches share one type.
    pub(super) fn if_expr(
        &mut self,
        e: &Expr,
        branches: &[(Expr, Block)],
        otherwise: Option<&Block>,
        used: Use,
    ) -> Res<Out> {
        if matches!(used, Use::Value(_)) && otherwise.is_none() {
            return Err(self.fail(
                (e.at, "if".len()),
                "if-needs-else",
                "an if used as a value needs an else",
            ));
        }
        let bool = self.prim("Bool");
        let joined = self.branches(e, &ends(e), used, |cx, i, u, again| match branches.get(i) {
            Some((cond, block)) => {
                if !again {
                    cx.expect(cond, bool)?;
                }
                cx.block(block, u)
            }
            None => cx.block(otherwise.expect("an else"), u),
        })?;
        Ok(match (used, joined) {
            (Use::Value(_), Joined { ty, leaves: false }) => {
                Out::Val(ty.unwrap_or_else(|| self.unknown()))
            }
            (_, Joined { leaves: true, .. }) if otherwise.is_some() => Out::Never,
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
        let (at, len) = match written {
            Type::Name { name, .. } => (name.at, name.text.len()),
            Type::Fn { at, .. } => (*at, 2),
        };
        self.map_key((at, len), *key)
    }

    /// A Map key is an Int or a String; `at` is where the key type is written or its first key.
    pub(super) fn map_key(&self, at: super::Span, key: Ty) -> Res {
        if self.is_var(key) || self.is(key, "Int") || self.is(key, "String") {
            return Ok(());
        }
        let found = self.types.show(key);
        let message = "a Map key is an Int or a String";
        let d = self.fail(at, "type-mismatch", message);
        Err(d.with_types(&"Int or String", &found))
    }
}

fn span_of(s: &Stmt) -> super::Span {
    (s.at, s.len)
}
