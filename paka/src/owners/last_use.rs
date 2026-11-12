//! The last uses a local moves out at (M6-33), in the narrow form only, with no liveness
//! analysis: a local read once on the right of `x = e` that writes it whole, and a `let` local
//! read once by the last statement of its block that names it. Either read runs whenever its
//! statement does, never in a branch or a loop of it, so the scopes the back end walks see the
//! move on every path through the statement. A loop never repeats such a read: the right of
//! `x = e` writes `x` again before the next one, and a `let` local is declared anew each time.

use crate::core_form::{
    Arg, Bin, Block, Callee, Expr, ExprKind, Local, Place, Root, Step, Stmt, Walk,
};
use crate::owners::whole_local;
use std::collections::BTreeSet;

/// The reads in `body`, by address, that move their local.
pub(super) fn reads(body: &Block) -> BTreeSet<*const Expr> {
    let mut found = BTreeSet::new();
    visit_block(body, &mut |node| {
        if let Node::Block(b) = node {
            found.extend(in_block(b));
        }
    });
    found
}

/// The reads that move in `b`'s own statements.
fn in_block(b: &Block) -> Vec<*const Expr> {
    let mut found = vec![];
    for (i, s) in b.stmts.iter().enumerate() {
        match s {
            Stmt::Let(l, _) => found.extend(let_last_use(b, i + 1, *l)),
            Stmt::Assign(Place { root, steps }, None, e) if steps.is_empty() => {
                let Root::Local(x) = root else { continue };
                if !leaves_loop(e) {
                    found.extend(read_once(e, *x));
                }
            }
            _ => {}
        }
    }
    found
}

/// The read of `let` local `l` that moves: the one naming it in the last statement from `from`
/// on, or the block's value, that names it at all.
fn let_last_use(b: &Block, from: usize, l: Local) -> Option<*const Expr> {
    let value = b
        .value
        .as_deref()
        .filter(|e| mentions(|f| visit_expr(e, f), l) > 0);
    let e = match value {
        Some(e) => e,
        None => {
            let named = |s: &&Stmt| mentions(|f| visit_stmt(s, f), l) > 0;
            let last = b.stmts[from..].iter().rfind(named)?;
            match last {
                Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => e,
                Stmt::Assign(_, _, e) if mentions(|f| visit_stmt(last, f), l) == 1 => e,
                _ => return None,
            }
        }
    };
    read_once(e, l)
}

/// The read of `l` in `e`, when it is the only place `e` names `l` and `e` always makes it.
fn read_once(e: &Expr, l: Local) -> Option<*const Expr> {
    if mentions(|f| visit_expr(e, f), l) != 1 {
        return None;
    }
    let mut reads = vec![];
    always(e, &mut reads);
    let read = reads.into_iter().find(|r| whole_local(r) == Some(l))?;
    Some(read as *const Expr)
}

/// The whole-local reads `e` makes whenever it runs: not in a block, nor right of `&&` or `||`.
fn always<'a>(e: &'a Expr, reads: &mut Vec<&'a Expr>) {
    let mut each = |es: &'a [Expr]| es.iter().for_each(|e| always(e, reads));
    match &e.kind {
        ExprKind::Place(p) => match &p.root {
            Root::Local(_) if p.steps.is_empty() => reads.push(e),
            Root::Local(_) => {}
            Root::Value(x) => always(x, reads),
        },
        ExprKind::Interp(es, _)
        | ExprKind::Record(_, es)
        | ExprKind::List(es)
        | ExprKind::Variant(_, es) => each(es),
        ExprKind::Map(entries) => entries.iter().for_each(|(k, v)| {
            always(k, reads);
            always(v, reads);
        }),
        ExprKind::Binary(Bin::And | Bin::Or, x, _)
        | ExprKind::Unary(_, x)
        | ExprKind::If(x, ..)
        | ExprKind::Match(x, _) => always(x, reads),
        ExprKind::Binary(_, x, y) => {
            always(x, reads);
            always(y, reads);
        }
        ExprKind::Call(callee, args, _) => {
            if let Callee::Value(f) = callee {
                always(f, reads);
            }
            for a in args {
                if let Arg::Value(e) = a {
                    always(e, reads);
                }
            }
        }
        _ => {}
    }
}

/// How many places under what `visit` walks are rooted at `l`.
fn mentions<'a>(visit: impl FnOnce(&mut dyn FnMut(Node<'a>)), l: Local) -> usize {
    let mut n = 0;
    visit(&mut |node| {
        if let Node::Place(p) = node
            && p.root == Root::Local(l)
        {
            n += 1;
        }
    });
    n
}

/// Whether `e` holds a `break` or `continue`.
fn leaves_loop(e: &Expr) -> bool {
    let mut found = false;
    visit_expr(e, &mut |node| {
        found |= matches!(node, Node::Stmt(Stmt::Break | Stmt::Continue));
    });
    found
}

enum Node<'a> {
    Block(&'a Block),
    Stmt(&'a Stmt),
    Place(&'a Place),
}

fn visit_block<'a>(b: &'a Block, f: &mut dyn FnMut(Node<'a>)) {
    f(Node::Block(b));
    b.stmts.iter().for_each(|s| visit_stmt(s, f));
    if let Some(e) = &b.value {
        visit_expr(e, f);
    }
}

fn visit_stmt<'a>(s: &'a Stmt, f: &mut dyn FnMut(Node<'a>)) {
    f(Node::Stmt(s));
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => visit_expr(e, f),
        Stmt::Assign(p, _, e) => {
            visit_place(p, f);
            visit_expr(e, f);
        }
        Stmt::For(_, Walk::List(e), b) | Stmt::While(e, b) => {
            visit_expr(e, f);
            visit_block(b, f);
        }
        Stmt::For(_, Walk::Range(x, y), b) => {
            visit_expr(x, f);
            visit_expr(y, f);
            visit_block(b, f);
        }
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn visit_place<'a>(p: &'a Place, f: &mut dyn FnMut(Node<'a>)) {
    f(Node::Place(p));
    if let Root::Value(e) = &p.root {
        visit_expr(e, f);
    }
    for step in &p.steps {
        match step {
            Step::Field(_) => {}
            Step::Item(e, _) | Step::Key(e, _) => visit_expr(e, f),
            Step::Slice(x, y, _) => {
                visit_expr(x, f);
                visit_expr(y, f);
            }
        }
    }
}

fn visit_expr<'a>(e: &'a Expr, f: &mut dyn FnMut(Node<'a>)) {
    match &e.kind {
        ExprKind::Interp(es, _)
        | ExprKind::Record(_, es)
        | ExprKind::List(es)
        | ExprKind::Variant(_, es) => es.iter().for_each(|e| visit_expr(e, f)),
        ExprKind::Map(entries) => entries.iter().for_each(|(k, v)| {
            visit_expr(k, f);
            visit_expr(v, f);
        }),
        ExprKind::Place(p) => visit_place(p, f),
        ExprKind::Unary(_, x) => visit_expr(x, f),
        ExprKind::Binary(_, x, y) => {
            visit_expr(x, f);
            visit_expr(y, f);
        }
        ExprKind::Call(callee, args, _) => {
            if let Callee::Value(c) = callee {
                visit_expr(c, f);
            }
            for a in args {
                match a {
                    Arg::Value(e) => visit_expr(e, f),
                    Arg::Inout(p) => visit_place(p, f),
                }
            }
        }
        ExprKind::If(c, then, otherwise) => {
            visit_expr(c, f);
            visit_block(then, f);
            otherwise.iter().for_each(|b| visit_block(b, f));
        }
        ExprKind::Match(subject, arms) => {
            visit_expr(subject, f);
            arms.iter().for_each(|a| visit_block(&a.body, f));
        }
        ExprKind::Root(b) => visit_block(b, f),
        ExprKind::Element(_, es, b, _) => {
            es.iter().for_each(|e| visit_expr(e, f));
            b.iter().for_each(|b| visit_block(b, f));
        }
        ExprKind::Child(c) => {
            c.key.iter().for_each(|k| visit_expr(k, f));
            visit_expr(&c.node, f);
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Hole(_)
        | ExprKind::FnValue(_)
        | ExprKind::Placeholder(_) => {}
    }
}
