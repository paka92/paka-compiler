//! Views (docs/decisions/M5-01.md, M5-02.md, M5-04.md): each element line of a view's body or
//! child block adds its node to the child list being built; the root is the one node the body's
//! lines give.

use super::Cx;
use crate::ast::{self, ExprKind as K, StmtKind as S};
use crate::builtins::ELEMENTS;
use crate::core_form::{Block, Callee, Child, Expr, ExprKind, Stmt, Type};
use crate::resolve::Binding;
use paka_rt::ops::Int;

/// How the element lines of one child block are numbered.
#[derive(Clone, Copy, Default)]
pub(super) struct Lines {
    next: usize,
    looped: bool,
}

impl<'a> Cx<'_, 'a> {
    /// A view's body: its root.
    pub(super) fn root(&mut self, body: &ast::Block<'a>) -> Block {
        let lines = self.numbered(body);
        Block {
            stmts: vec![],
            value: Some(Box::new(self.drawn(ExprKind::Root(lines)))),
        }
    }

    /// `b` as element lines numbered from 0.
    fn numbered(&mut self, b: &ast::Block<'a>) -> Block {
        let outer = self.lines.replace(Lines::default());
        let block = self.block(b, false);
        self.lines = outer;
        block
    }

    /// `let`s as usual; an `if` or `match` holds more lines and a `for` adds its lines in a loop.
    pub(super) fn element_lines(&mut self, list: &[ast::Stmt<'a>]) -> Block {
        let line = |cx: &mut Self, s: &ast::Stmt<'a>| match &s.kind {
            S::Expr(e) => Stmt::Expr(cx.line(e)),
            S::For { .. } => {
                let outer = cx.looped(true);
                let s = cx.stmt(s);
                cx.looped(outer);
                s
            }
            _ => cx.stmt(s),
        };
        Block {
            stmts: list.iter().map(|s| line(self, s)).collect(),
            value: None,
        }
    }

    fn looped(&mut self, looped: bool) -> bool {
        let lines = self.lines.as_mut().expect("in a view");
        std::mem::replace(&mut lines.looped, looped)
    }

    fn line(&mut self, e: &ast::Expr<'a>) -> Expr {
        match &e.kind {
            K::If {
                branches,
                otherwise,
            } => return self.if_expr(e, branches, otherwise.as_ref()),
            K::Match { subject, arms } => return self.match_expr(e, subject, arms),
            _ => {}
        }
        let lines = self.lines.as_mut().expect("in a view");
        let (line, looped) = (lines.next, lines.looped);
        lines.next += 1;
        let (node, key) = self.node(e);
        let child = Child {
            line,
            looped,
            key,
            node,
        };
        Expr {
            ty: None,
            kind: ExprKind::Child(Box::new(child)),
        }
    }

    /// The node of an element line, and its key.
    fn node(&mut self, e: &ast::Expr<'a>) -> (Expr, Option<Expr>) {
        let (call, block) = match &e.kind {
            K::Hole(name) => {
                let site = self.site_of(e.at, Some(name.to_string()));
                return (self.drawn(ExprKind::Placeholder(site)), None);
            }
            K::Children(call, b) => (&**call, Some(b)),
            _ => (e, None),
        };
        let K::Call(callee, args) = &call.kind else {
            unreachable!("an element or a view call");
        };
        let (args, key) = match args.split_last() {
            Some((last, rest)) if last.label.as_ref().is_some_and(|l| l.text == "key") => {
                (rest, Some(&last.value))
            }
            _ => (&args[..], None),
        };
        let kind = match self.resolved.get(callee.at) {
            Some(Binding::Def(d)) => self.call(Callee::Fn(self.ids[d]), args, callee.at),
            Some(Binding::Builtin(name)) => self.element(callee, name, args, block),
            _ => unreachable!("an element or a view call"),
        };
        let node = self.drawn(kind);
        (node, key.map(|k| self.expr(k)))
    }

    /// The element's arguments in table order, each one left out given its default.
    fn element(
        &mut self,
        callee: &ast::Expr<'a>,
        name: &str,
        args: &[ast::Arg<'a>],
        block: Option<&ast::Block<'a>>,
    ) -> ExprKind {
        let row = ELEMENTS.iter().position(|el| el.name == name);
        let row = row.expect("an element");
        let mut written = args.iter().peekable();
        let mut values = vec![];
        for &(param, _, default) in ELEMENTS[row].args {
            let label = param.strip_suffix(':');
            let value = match written.next_if(|a| a.label.as_ref().map(|l| l.text) == label) {
                Some(a) => self.expr(&a.value),
                None => Expr {
                    ty: Some(self.core.ty(Type::Int)),
                    kind: ExprKind::Int(default.and_then(Int::parse).expect("an Int default")),
                },
            };
            values.push(value);
        }
        let site = self.site(callee.at);
        let children = block.map(|b| self.numbered(b));
        ExprKind::Element(row, values, children, site)
    }

    /// A node of the view's type.
    fn drawn(&self, kind: ExprKind) -> Expr {
        Expr {
            ty: self.result,
            kind,
        }
    }
}
