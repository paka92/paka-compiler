//! Ownership (docs/decisions/M2-05.md): an `&` place against the other operands of its call
//! (Rule A), a read against what its siblings change (Rule B), over every form with operands
//! (Rule C), and the place a `for` walks (§1.7). Every check reads one statement; the types come
//! from the pass before, and a read of unknown type is left alone.
//!
//! The walk keeps its own stack, so a deep chain cannot overflow it. A write looks up the stack
//! once, so the cost grows with the writes times their depth, not with the operands.

use super::text::{assign_op, render};
use super::{Cx, Span, earliest, span};
use crate::ast::{
    Arg, ArmBody, Block, Expr, ExprKind as K, Name, Pos, Stmt, StmtKind as S, StrPart,
};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;

const OVERLAP: &str = "inout-overlap";
const READ: &str = "changed-while-read";
const WALKED: &str = "changed-while-walked";

#[derive(Clone, Copy)]
/// One step below a root: a field, or an index or slice with its base and its index.
enum Step<'x> {
    Field(&'x str),
    Index(&'x Expr<'x>, &'x Expr<'x>),
}

/// A local name followed by steps (§1.2): the root by the position of its declaration.
pub(super) struct Place<'x> {
    root: Pos,
    steps: Vec<Step<'x>>,
    e: &'x Expr<'x>,
}

/// Two places overlap unless their roots differ or they first part at two different fields.
fn overlap(a: &Place, b: &Place) -> bool {
    let apart = |pair: (&Step, &Step)| matches!(pair, (Step::Field(x), Step::Field(y)) if x != y);
    a.root == b.root && !a.steps.iter().zip(&b.steps).any(apart)
}

/// Where two overlapping places first differ as written: the collection and two different
/// indexes into it.
fn parting<'x>(a: &Place<'x>, b: &Place<'x>) -> Option<(&'x Expr<'x>, &'x Expr<'x>, &'x Expr<'x>)> {
    a.steps.iter().zip(&b.steps).find_map(|pair| match pair {
        (&Step::Index(base, x), &Step::Index(_, y)) => {
            let (tx, ty) = (render(x), render(y));
            (tx != ty || tx.contains("..")).then_some((base, x, y))
        }
        _ => None,
    })
}

/// The name of a `let` that binds `p`, as a message proposes it.
fn let_name(p: &Place) -> String {
    let name = match p.steps.last() {
        None => "old",
        Some(Step::Field(field)) => field,
        Some(Step::Index(_, i)) if matches!(i.kind, K::Range(..)) => "part",
        Some(Step::Index(_, i)) if matches!(i.kind, K::Int("0")) => "first",
        Some(Step::Index(base, _)) => {
            let word = match &base.kind {
                K::Name(n) => n,
                K::Field(_, n) => n.text,
                _ => "",
            };
            word.strip_suffix('s')
                .filter(|w| !w.is_empty())
                .unwrap_or("item")
        }
    };
    name.to_string()
}

/// The operands of a form that evaluates several in order (Rule C).
fn operands<'x>(e: &'x Expr<'x>) -> Option<Vec<&'x Expr<'x>>> {
    Some(match &e.kind {
        K::Call(_, args) | K::Record(_, args) => args.iter().map(|a| &a.value).collect(),
        K::Binary(_, _, l, r) => vec![l, r],
        K::List(items) => items.iter().collect(),
        K::Map(pairs) => pairs.iter().flat_map(|(k, v)| [k, v]).collect(),
        K::Str(parts) => (parts.iter())
            .filter_map(|p| match p {
                StrPart::Expr(x) => Some(x),
                StrPart::Text(_) => None,
            })
            .collect(),
        _ => return None,
    })
}

/// How a message names the form that reads an operand.
fn reader(e: &Expr) -> String {
    match &e.kind {
        K::Call(f, _) => render(f),
        K::Record(name, _) => name.to_string(),
        K::Binary(op, ..) => op.text().to_string(),
        K::List(_) => "this list".to_string(),
        K::Map(_) => "this map".to_string(),
        _ => "this text".to_string(),
    }
}

#[derive(Clone, Copy)]
enum Part<'x> {
    Expr(&'x Expr<'x>),
    Stmt(&'x Stmt<'x>),
    Block(&'x Block<'x>),
}

/// A part on the walk's stack, its children and the next one to visit.
struct Frame<'x> {
    part: Part<'x>,
    kids: Vec<Part<'x>>,
    next: usize,
}

/// The children of a part; a form's operands come first, in order, so a child's index is its
/// operand's index.
fn kids<'x>(part: Part<'x>) -> Vec<Part<'x>> {
    let e = Part::Expr;
    match part {
        Part::Block(b) => b.stmts.iter().map(Part::Stmt).collect(),
        Part::Stmt(s) => match &s.kind {
            S::Let { value: x, .. } | S::Discard(x) | S::Return(Some(x)) | S::Expr(x) => vec![e(x)],
            S::Assign { target, value, .. } => vec![e(target), e(value)],
            S::For { list, body, .. } => vec![e(list), Part::Block(body)],
            S::While { cond, body } => vec![e(cond), Part::Block(body)],
            S::Return(None) | S::Break | S::Continue => Vec::new(),
        },
        Part::Expr(x) => {
            if let Some(ops) = operands(x) {
                let callee = match &x.kind {
                    K::Call(f, _) => Some(e(f)),
                    _ => None,
                };
                return ops.into_iter().map(e).chain(callee).collect();
            }
            match &x.kind {
                K::Unary(_, a) | K::Field(a, _) | K::Try(a, _) | K::Inout(a) => vec![e(a)],
                K::Range(a, b) | K::Index(a, b) => vec![e(a), e(b)],
                K::If {
                    branches,
                    otherwise,
                } => (branches.iter())
                    .flat_map(|(c, b)| [e(c), Part::Block(b)])
                    .chain(otherwise.iter().map(Part::Block))
                    .collect(),
                K::Match { subject, arms } => std::iter::once(e(subject))
                    .chain(arms.iter().map(|a| match &a.body {
                        ArmBody::Block(b) => Part::Block(b),
                        ArmBody::Line(s) => Part::Stmt(s),
                    }))
                    .collect(),
                _ => Vec::new(),
            }
        }
    }
}

impl Cx<'_, '_> {
    /// The earliest ownership error of a body.
    pub(super) fn ownership(&self, body: &Block) -> Option<Diagnostic> {
        let mut found = None;
        let mut frames: Vec<Frame> = Vec::new();
        let mut next = Some(Part::Block(body));
        loop {
            if let Some(part) = next.take() {
                found = earliest(found, self.enter(part, &frames));
                let kids = kids(part);
                frames.push(Frame {
                    part,
                    kids,
                    next: 0,
                });
            }
            let Some(top) = frames.last_mut() else {
                return found;
            };
            match top.kids.get(top.next) {
                Some(&kid) => {
                    top.next += 1;
                    next = Some(kid);
                }
                None => drop(frames.pop()),
            }
        }
    }

    /// The place `e` names when its root is a local name.
    pub(super) fn rooted<'x>(&self, e: &'x Expr<'x>) -> Option<Place<'x>> {
        let mut steps = Vec::new();
        let mut cur = e;
        loop {
            cur = match &cur.kind {
                K::Field(x, name) => {
                    steps.push(Step::Field(name.text));
                    x
                }
                K::Index(x, i) => {
                    steps.push(Step::Index(x, i));
                    x
                }
                K::Name(_) => break,
                _ => return None,
            };
        }
        let Some(Binding::Local(root)) = self.resolved.get(cur.at) else {
            return None;
        };
        steps.reverse();
        Some(Place { root, steps, e })
    }

    /// A read of a place, unless its type is Int, Float or Bool (a fresh value) or unknown.
    fn read<'x>(&self, e: &'x Expr<'x>) -> Option<Place<'x>> {
        let &ty = self.info.get(&span(e))?;
        let fresh = self.is_var(ty) || ["Int", "Float", "Bool"].iter().any(|n| self.is(ty, n));
        if fresh { None } else { self.rooted(e) }
    }

    /// The checks a part starts: Rule A on a call, and every write against the parts around it.
    fn enter(&self, part: Part, frames: &[Frame]) -> Option<Diagnostic> {
        match part {
            Part::Expr(e) => {
                let K::Call(f, args) = &e.kind else {
                    return None;
                };
                let writes = args.iter().filter_map(|a| match &a.value.kind {
                    K::Inout(x) => Some((self.rooted(x)?, span(&a.value))),
                    _ => None,
                });
                writes.fold(self.inout_overlap(f, args), |found, (place, at)| {
                    earliest(found, self.wrote(frames, &place, at, &render(e)))
                })
            }
            Part::Stmt(s) => {
                let S::Assign { target, op, value } = &s.kind else {
                    return None;
                };
                let place = self.rooted(target)?;
                let by = format!("{} {} {}", render(target), assign_op(*op), render(value));
                self.wrote(frames, &place, span(target), &by)
            }
            Part::Block(_) => None,
        }
    }

    /// The reads and walks around a write that it changes; `by` names the write.
    fn wrote(&self, frames: &[Frame], place: &Place, at: Span, by: &str) -> Option<Diagnostic> {
        (frames.iter().rev()).fold(None, |found, frame| {
            let inside = frame.next - 1;
            let d = match frame.part {
                Part::Expr(e) => self.changed_while_read(e, inside, place, by),
                Part::Stmt(Stmt {
                    kind: S::For { name, list, .. },
                    ..
                }) if inside == 1 => self.changed_while_walked(name, list, place, at),
                _ => None,
            };
            earliest(found, d)
        })
    }

    /// Rule B: an operand of `e` other than the one at `inside` that reads what is written.
    fn changed_while_read(
        &self,
        e: &Expr,
        inside: usize,
        place: &Place,
        by: &str,
    ) -> Option<Diagnostic> {
        let ops = operands(e)?;
        let (op, read) = (ops.into_iter().enumerate())
            .filter(|&(i, _)| i != inside)
            .find_map(|(_, op)| Some((op, self.read(op).filter(|r| overlap(r, place))?)))?;
        let (r, f, name) = (render(op), reader(e), let_name(&read));
        let message = format!(
            "{r} is read by {f} and changed by {by} in the same call; bind it first: let {name} = {r}"
        );
        Some(self.fail_at(op, READ, message))
    }

    fn changed_while_walked(
        &self,
        name: &Name,
        list: &Expr,
        place: &Place,
        at: Span,
    ) -> Option<Diagnostic> {
        let walked = self.rooted(list).filter(|w| overlap(w, place))?;
        let (w, x) = (render(walked.e), name.text);
        let message = format!(
            "{w} is walked by this for; walk a copy (let items = {w}, then for {x} in items) or by \
             position (for i in 0..<len({w}))"
        );
        Some(self.fail(at, WALKED, message))
    }

    /// Rule A: the first operand that overlaps an earlier one where either is an `&` place.
    fn inout_overlap(&self, f: &Expr, args: &[Arg]) -> Option<Diagnostic> {
        if !args.iter().any(|a| matches!(a.value.kind, K::Inout(_))) {
            return None;
        }
        let places: Vec<_> = (args.iter())
            .map(|a| match &a.value.kind {
                K::Inout(x) => self.rooted(x).map(|p| (p, true)),
                _ => self.read(&a.value).map(|p| (p, false)),
            })
            .collect();
        for (j, later) in places.iter().enumerate() {
            let Some(later) = later else { continue };
            let clash = places[..j]
                .iter()
                .flatten()
                .find(|earlier| (earlier.1 || later.1) && overlap(&earlier.0, &later.0));
            if let Some(earlier) = clash {
                let message = self.overlap_text(f, earlier, later);
                return Some(self.fail_at(&args[j].value, OVERLAP, message));
            }
        }
        None
    }

    /// The `inout-overlap` text for two operands of `f`, each a place and whether it has `&`.
    fn overlap_text(&self, f: &Expr, a: &(Place, bool), b: &(Place, bool)) -> String {
        let twice = a.1 && b.1;
        let (amp, other) = if a.1 { (&a.0, &b.0) } else { (&b.0, &a.0) };
        let Some((base, i, j)) = parting(amp, other) else {
            if twice {
                let short = if amp.steps.len() <= other.steps.len() {
                    amp
                } else {
                    other
                };
                return format!("{} is passed with & twice", render(short.e));
            }
            let (read, name) = (render(other.e), let_name(other));
            return format!(
                "{} is passed with & and read in one call; bind it first: let {name} = {read}",
                render(amp.e)
            );
        };
        let map = (self.info.get(&span(base))).is_some_and(|&t| self.is(t, "Map"));
        let (of, items) = (render(base), if map { "entries" } else { "items" });
        if twice {
            let (f, i, j) = (render(f), render(i), render(j));
            format!("two {items} of {of} in one call; write {f}(&{of}, {i}, {j})")
        } else {
            format!("two {items} of {of} in one call; bind one first")
        }
    }
}
