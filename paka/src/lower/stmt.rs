//! Blocks and statements.

use super::Cx;
use crate::ast::{self, AssignOp, BinOp, ExprKind as K, StmtKind as S};
use crate::core_form::{Block, ExprKind, Place, Stmt, Type, Walk};

impl<'a> Cx<'_, 'a> {
    /// `valued` when the block's last expression, if it gives a value, is the block's value.
    pub(super) fn block(&mut self, b: &ast::Block<'a>, valued: bool) -> Block {
        self.stmts(&b.stmts, valued)
    }

    pub(super) fn stmts(&mut self, list: &[ast::Stmt<'a>], valued: bool) -> Block {
        let Some((last, init)) = list.split_last() else {
            return Block {
                stmts: vec![],
                value: None,
            };
        };
        let mut stmts: Vec<_> = init.iter().map(|s| self.stmt(s)).collect();
        let mut value = None;
        match &last.kind {
            S::Expr(e) if valued && self.entry(e).is_some() => {
                value = Some(Box::new(self.expr(e)));
            }
            _ => stmts.push(self.stmt(last)),
        }
        Block { stmts, value }
    }

    fn stmt(&mut self, s: &ast::Stmt<'a>) -> Stmt {
        match &s.kind {
            S::Let { name, value, .. } => {
                let value = self.expr(value);
                let ty = value.ty.expect("a let holds a value");
                Stmt::Let(self.declare(name.at, ty), value)
            }
            S::Discard(e) | S::Expr(e) => Stmt::Expr(self.expr(e)),
            S::Assign { target, op, value } => self.assign(target, *op, value),
            S::Return(e) => Stmt::Return(e.as_ref().map(|e| self.expr(e))),
            S::Break => Stmt::Break,
            S::Continue => Stmt::Continue,
            S::For { name, list, body } => {
                let (walk, item) = self.walk(list);
                let local = self.declare(name.at, item);
                Stmt::For(local, walk, self.block(body, false))
            }
            S::While { cond, body } => {
                let cond = self.expr(cond);
                Stmt::While(cond, self.block(body, false))
            }
        }
    }

    fn assign(&mut self, target: &ast::Expr<'a>, op: AssignOp, value: &ast::Expr<'a>) -> Stmt {
        let (place, ty) = self.place(target);
        let value = self.expr(value);
        let compound = bin_op(op).map(|op| self.math(op, ty, self.after(target)).into());
        Stmt::Assign(place, compound, value)
    }

    /// The place `e` names, and its type.
    pub(super) fn place(&mut self, e: &ast::Expr<'a>) -> (Place, Option<usize>) {
        let e = self.expr(e);
        let ExprKind::Place(place) = e.kind else {
            unreachable!("a checked place");
        };
        (place, e.ty)
    }

    /// What a `for` walks, and the type of its name.
    fn walk(&mut self, list: &ast::Expr<'a>) -> (Walk, usize) {
        if let K::Range(a, b) = &list.kind {
            let (a, b) = (self.expr(a), self.expr(b));
            return (Walk::Range(a, b), self.core.ty(Type::Int));
        }
        let list = self.expr(list);
        let item = match list.ty.map(|t| &self.core.types[t]) {
            Some(Type::List(item)) => *item,
            _ => unreachable!("a checked list"),
        };
        (Walk::List(list), item)
    }
}

/// The operator of `op=`, none for `=`.
fn bin_op(op: AssignOp) -> Option<BinOp> {
    match op {
        AssignOp::Set => None,
        AssignOp::Add => Some(BinOp::Add),
        AssignOp::Sub => Some(BinOp::Sub),
        AssignOp::Mul => Some(BinOp::Mul),
        AssignOp::Div => Some(BinOp::Div),
        AssignOp::Rem => Some(BinOp::Rem),
    }
}
