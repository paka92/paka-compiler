//! Drop lists written by hand: each exit drops what is open at it, last owned first.

use super::{Owner, Owners, last_use, whole_local};
use crate::core_form::{Arg, Block, Callee, Expr, ExprKind, Local, Root, Stmt, Walk};
use crate::source::Source;

use Owner::{Local as L, Temp as T};

/// Types are not looked at; every owner here is of type 0.
fn names(dropped: Vec<(usize, Owner<u32>)>) -> Vec<Owner<u32>> {
    dropped.into_iter().map(|(_, o)| o).collect()
}

fn owners() -> Owners<u32> {
    let mut o = Owners::default();
    o.enter(false);
    o.own(0, L(0));
    o
}

#[test]
fn a_scope_drops_its_owners_last_first() {
    let mut o = owners();
    o.enter(false);
    o.own(0, L(1));
    o.own(0, T(7));
    assert_eq!(names(o.leave()), [T(7), L(1)]);
    assert_eq!(names(o.leave()), [L(0)]);
}

#[test]
fn a_moved_temporary_is_not_dropped() {
    let mut o = owners();
    o.own(0, T(1));
    o.own(0, T(2));
    assert!(o.disown(1));
    assert!(!o.disown(1));
    assert_eq!(names(o.leave()), [T(2), L(0)]);
}

#[test]
fn a_statement_frees_its_temporaries_and_keeps_its_locals() {
    let mut o = owners();
    let mark = o.mark();
    o.own(0, T(1));
    o.own(0, L(1));
    o.own(0, T(2));
    assert_eq!(names(o.temps_since(mark)), [T(2), T(1)]);
    assert_eq!(names(o.leave()), [L(1), L(0)]);
}

#[test]
fn a_break_drops_through_the_loop_body_only() {
    let mut o = owners();
    o.enter(true);
    o.own(0, L(1));
    o.enter(false);
    o.own(0, T(3));
    assert_eq!(names(o.to_loop()), [T(3), L(1)]);
    // Nothing is taken: the code after the exit still closes the scopes.
    assert_eq!(names(o.leave()), [T(3)]);
}

#[test]
fn a_return_drops_everything_but_the_local_it_returns() {
    let mut o = owners();
    o.enter(true);
    o.own(0, L(1));
    o.own(0, T(4));
    assert_eq!(names(o.to_fn(Some(1))), [T(4), L(0)]);
    assert_eq!(names(o.to_fn(None)), [T(4), L(1), L(0)]);
}

#[test]
fn a_block_moves_out_only_its_own_local() {
    let mut o = owners();
    o.enter(false);
    o.own(0, L(1));
    assert!(!o.disown_local(0));
    assert!(o.disown_local(1));
    assert_eq!(names(o.leave()), []);
}

/// The body of `f` in `text`.
fn body(text: &str) -> Block {
    let source = Source::file("main.paka", text.into());
    let program = crate::check("main.paka", &source)
        .unwrap_or_else(|d| panic!("{d}"))
        .0;
    program
        .fns
        .into_iter()
        .find(|f| f.name == "f")
        .expect("an `f`")
        .body
}

/// The whole-local reads under `b`, in source order.
fn reads_in<'a>(b: &'a Block, found: &mut Vec<&'a Expr>) {
    for s in &b.stmts {
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Assign(_, _, e) | Stmt::Return(Some(e)) => {
                expr_reads(e, found)
            }
            Stmt::For(_, Walk::List(e), body) | Stmt::While(e, body) => {
                expr_reads(e, found);
                reads_in(body, found);
            }
            Stmt::For(_, Walk::Range(x, y), body) => {
                expr_reads(x, found);
                expr_reads(y, found);
                reads_in(body, found);
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
    b.value.iter().for_each(|e| expr_reads(e, found));
}

fn expr_reads<'a>(e: &'a Expr, found: &mut Vec<&'a Expr>) {
    match &e.kind {
        ExprKind::Place(p) if p.steps.is_empty() && matches!(p.root, Root::Local(_)) => {
            found.push(e)
        }
        ExprKind::Interp(es, _) | ExprKind::Variant(_, es) => {
            es.iter().for_each(|e| expr_reads(e, found))
        }
        ExprKind::Unary(_, x) => expr_reads(x, found),
        ExprKind::Binary(_, x, y) => {
            expr_reads(x, found);
            expr_reads(y, found);
        }
        ExprKind::Call(callee, args, _) => {
            if let Callee::Value(f) = callee {
                expr_reads(f, found);
            }
            for a in args {
                if let Arg::Value(e) = a {
                    expr_reads(e, found);
                }
            }
        }
        ExprKind::If(c, then, otherwise) => {
            expr_reads(c, found);
            reads_in(then, found);
            otherwise.iter().for_each(|b| reads_in(b, found));
        }
        ExprKind::Match(x, arms) => {
            expr_reads(x, found);
            arms.iter().for_each(|a| reads_in(&a.body, found));
        }
        ExprKind::Root(b) => reads_in(b, found),
        _ => {}
    }
}

/// The locals `f` in `text` moves out, one per moving read, in source order.
fn moved(text: &str) -> Vec<Local> {
    let body = body(text);
    let moving = last_use::reads(&body);
    let mut reads = vec![];
    reads_in(&body, &mut reads);
    let moves = reads
        .into_iter()
        .filter(|&e| moving.contains(&(e as *const Expr)));
    moves.filter_map(whole_local).collect()
}

#[test]
fn a_let_moves_at_its_last_read() {
    assert_eq!(
        moved("fn f(s: String) -> String {\n  let t = s + \"!\"\n  t\n}\n"),
        [1]
    );
    let later = "fn f(s: String) -> Int {\n  let t = s + \"!\"\n  let n = 1\n  let m = len(t) + n\n  m\n}\n";
    assert_eq!(moved(later), [1, 2, 3]);
}

#[test]
fn a_let_read_twice_or_in_a_branch_is_copied() {
    let twice = "fn f(s: String) -> String {\n  let t = s + \"!\"\n  t + t\n}\n";
    assert_eq!(moved(twice), []);
    let branch = "fn f(s: String, b: Bool) -> String {\n  let t = s + \"!\"\n  if b { t } else { \"\" }\n}\n";
    assert_eq!(moved(branch), []);
}

#[test]
fn a_read_in_a_loop_of_a_local_from_outside_it_is_copied() {
    let text = "fn f(s: String, n: Int) -> String {
  let t = s + \"-\"
  var out = \"\"
  for i in 0..<n {
    let piece = t + \".\"
    out = out + piece
  }
  out
}
";
    // `out` moves on the right of its own write and as the value; `piece` at its last read.
    let [t, out] = [2, 3];
    let moves = moved(text);
    assert!(!moves.contains(&t), "{moves:?}");
    assert_eq!(moves.iter().filter(|&&l| l == out).count(), 2, "{moves:?}");
    assert_eq!(moves.len(), 3, "{moves:?}");
}

#[test]
fn a_write_that_reads_its_local_twice_copies() {
    let twice = "fn f(s: String) -> String {\n  var x = s\n  x = x + x\n  \"\"\n}\n";
    assert_eq!(moved(twice), []);
}

#[test]
fn a_write_moves_its_local_before_a_try_that_may_return() {
    let text = "enum E {\n  N(v: Int),\n  Add(l: E, r: E)\n}

fn f(ws: List<String>) -> Result<E, String> {
  var e = N(0)
  for w in ws {
    e = Add(e, N(ok_or(parse_int(w), w)?))
  }
  Ok(e)
}
";
    let moves = moved(text);
    assert_eq!(moves.len(), 2, "{moves:?}");
    assert_eq!(moves[0], moves[1]);
}

/// One function body's owners: local 0 is a parameter, local 1 a `let` that moves at `read`.
fn moving(body: &Block, read: &Expr) -> Owners<u32> {
    let mut o = Owners::new(body);
    o.enter(false);
    o.own(0, L(1));
    assert_eq!(o.moves(read), Some(1));
    o
}

#[test]
fn a_moved_local_is_not_dropped_on_any_exit() {
    let b = body("fn f(s: String) -> String {\n  let t = s + \"!\"\n  t\n}\n");
    let read = b.value.as_deref().unwrap();
    let mut o = Owners::new(&b);
    o.enter(false);
    o.own(0, L(1));
    // Before the read every exit drops it.
    assert_eq!(names(o.to_fn(None)), [L(1)]);
    o = moving(&b, read);
    assert!(!o.holds(1));
    assert_eq!(o.moves(read), None);
    o.enter(true);
    assert_eq!(names(o.to_loop()), []);
    assert_eq!(names(o.to_fn(None)), []);
    assert_eq!(names(o.leave()), []);
    assert_eq!(names(o.leave()), []);
}

#[test]
fn a_local_written_whole_after_its_move_owns_again() {
    let b = body("fn f(s: String) -> String {\n  let t = s + \"!\"\n  t\n}\n");
    let mut o = moving(&b, b.value.as_deref().unwrap());
    o.own(0, T(5));
    assert!(o.disown(5));
    // The write finds nothing to drop and the local owns the new value.
    assert!(o.refill(1));
    assert!(!o.refill(1));
    assert_eq!(names(o.to_fn(None)), [L(1)]);
    assert_eq!(names(o.leave()), [L(1)]);
}

#[test]
fn a_read_that_is_not_a_last_use_does_not_move() {
    let b = body("fn f(s: String) -> String {\n  let t = s + \"!\"\n  t + t\n}\n");
    let mut reads = vec![];
    reads_in(&b, &mut reads);
    let mut o = Owners::new(&b);
    o.enter(false);
    o.own(0, L(1));
    assert!(reads.iter().all(|&e| o.moves(e).is_none()));
    assert_eq!(names(o.leave()), [L(1)]);
}
