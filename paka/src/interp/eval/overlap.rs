//! The run-time inout overlap detector (M6-22a, docs/decisions/M2-05.md section 3). It flags a
//! call when, at its start, an inout argument shares storage with another argument, or when a
//! read-only place argument is written between its evaluation and the call's return. Index values
//! are compared, so it finds what really overlaps, which the static check over-approximates.

use super::{Given, Idx};
use crate::core_form::{Arg, Local, Site};
use crate::interp::value::{Key, Value, position};
use paka_rt::ops::Int;

/// One step of a place's storage with its operands resolved; the steps after a slice fold into
/// the slice's range.
#[derive(PartialEq)]
pub(super) enum Span {
    Field(usize),
    Key(Key),
    Items(usize, usize),
}

/// A read-only place argument of a call whose operands are being evaluated.
struct Read {
    depth: usize,
    local: Local,
    spans: Vec<Span>,
    call: Site,
}

#[derive(Default)]
pub struct Detector {
    reads: Vec<Read>,
    /// The site of each flagged call, in run order.
    pub flagged: Vec<Site>,
}

impl Detector {
    pub(super) fn marks(&self) -> usize {
        self.reads.len()
    }

    /// Notes a read-only place argument of the call at `call`, unless a register holds it.
    pub(super) fn read(&mut self, depth: usize, local: Local, path: &[Idx], v: &Value, call: Site) {
        if !held_in_register(v) {
            let spans = spans(path);
            self.reads.push(Read {
                depth,
                local,
                spans,
                call,
            });
        }
    }

    /// The call at `site` starts with `given`, its reads past `base`: flags it when an inout
    /// overlaps another argument, then counts each inout as a write against the enclosing calls.
    /// Whether two of the places it takes overlap, which the interpreter cannot run.
    pub(super) fn start(
        &mut self,
        depth: usize,
        site: Site,
        args: &[Arg],
        given: &[Given],
        base: usize,
    ) -> bool {
        let taken: Vec<_> = (given.iter().zip(args))
            .filter_map(|(g, arg)| match g {
                Given::Borrow(l, path) => Some((matches!(arg, Arg::Inout(_)), *l, spans(path))),
                Given::Own(_) => None,
            })
            .collect();
        let inouts = || taken.iter().filter(|(inout, ..)| *inout);
        let own = &self.reads[base..];
        let read =
            inouts().any(|(_, l, s)| own.iter().any(|r| r.local == *l && overlap(s, &r.spans)));
        let twice = taken.iter().enumerate().any(|(n, (_, l, s))| {
            taken[n + 1..]
                .iter()
                .any(|(_, m, t)| m == l && overlap(s, t))
        });
        if read || twice {
            self.flagged.push(site);
        }
        self.reads.truncate(base);
        for (_, l, s) in inouts() {
            self.wrote(depth, *l, s);
        }
        twice
    }

    /// Flags each call in the frame at `depth` whose read-only argument the write overlaps.
    pub(super) fn wrote(&mut self, depth: usize, local: Local, spans: &[Span]) {
        let hit = (self.reads.iter())
            .filter(|r| r.depth == depth && r.local == local && overlap(spans, &r.spans))
            .map(|r| r.call);
        self.flagged.extend(hit);
    }
}

/// Int, Float, Bool and function values are read into a register (M2-05 section 1.2).
pub(super) fn held_in_register(v: &Value) -> bool {
    matches!(
        v,
        Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Fn(_)
    )
}

pub(super) fn spans(path: &[Idx]) -> Vec<Span> {
    let mut out = Vec::with_capacity(path.len());
    let mut sliced = false;
    for step in path {
        let span = match step {
            Idx::Field(n) => Span::Field(*n),
            Idx::Key(k, _) => Span::Key(k.clone()),
            Idx::Item(i, _) => Span::Items(at(*i), at(*i).saturating_add(1)),
            Idx::Slice(a, b, _) => Span::Items(at(*a), at(*b)),
        };
        match (sliced, out.last_mut(), span) {
            (true, Some(Span::Items(from, to)), Span::Items(a, b)) => {
                let base = *from;
                (*from, *to) = (base.saturating_add(a), base.saturating_add(b));
            }
            (_, _, span) => out.push(span),
        }
        sliced = matches!(step, Idx::Slice(..));
    }
    out
}

/// A position; a negative one, which traps when walked, stands past every other.
fn at(i: Int) -> usize {
    position(i).unwrap_or(usize::MAX)
}

/// Whether two places of one local share storage: a prefix does, and so do steps that differ
/// only at intersecting item ranges.
fn overlap(a: &[Span], b: &[Span]) -> bool {
    for (x, y) in a.iter().zip(b) {
        match (x, y) {
            (Span::Items(p, q), Span::Items(r, s)) => {
                if p.max(r) >= q.min(s) {
                    return false;
                }
                if q - p != 1 || s - r != 1 {
                    return true;
                }
            }
            _ if x != y => return false,
            _ => {}
        }
    }
    true
}
