//! Moves, copies and drops (M4-33, docs/decisions/M4-05.md section 9; core.md section 2 "Copies").
//! A fresh value is a temporary its statement owns until something takes it: a binding, a field,
//! an item or a result. Taking a temporary moves it; taking anything else copies it. Each owner
//! is dropped where it ends: a statement, a scope, a `break` or `continue`, or a `return`. A trap
//! or a stop leaves the rest to the run's end.

use super::values::slot;
use super::{Gen, Refusal};
use crate::core_form::{Block, Expr, ExprKind, Local, Root, Stmt, Ty};
use crate::owners::Owner;
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, types};
use paka_rt::abi::dev::{Context, Rt};
use std::mem::offset_of;

type Dropped = Vec<(Ty, Owner<Value>)>;

impl Gen<'_> {
    /// `v`, fresh, now owned by the statement.
    pub(super) fn temp(&mut self, t: Ty, v: Value) -> Value {
        if self.l.owns(t) {
            self.owners.own(t, Owner::Temp(v));
        }
        v
    }

    /// `v` for a new owner: a temporary moves, anything else is copied.
    pub(super) fn owned(&mut self, t: Ty, v: Value) -> Value {
        match self.l.owns(t) {
            true if self.owners.disown(v) => v,
            true => self.copy_of(t, v, 0),
            // Its words may lie in a value about to be dropped.
            false if self.held(t) => {
                let size = self.size(t);
                let to = slot(&mut self.b, size);
                self.put(t, to, 0, v);
                to
            }
            false => v,
        }
    }

    /// A copy of `v` of type `t` in a new slot, owning new blocks; a `memory` trap at `site`.
    pub(super) fn copy_of(&mut self, t: Ty, v: Value, site: u64) -> Value {
        let size = self.size(t);
        let to = slot(&mut self.b, size);
        let record = self.record(t);
        let site = self.int_const(site as i64);
        self.rt_try(offset_of!(Rt, copy), &[self.ctx, site, record, v, to]);
        to
    }

    /// Binds local `l`, which then owns `v`.
    pub(super) fn bind_owned(&mut self, l: Local, v: Value) {
        let t = self.locals[l];
        let v = self.owned(t, v);
        self.bind(l, v);
        if self.l.owns(t) {
            self.owners.own(t, Owner::Local(l));
        }
    }

    /// Frees what the value of type `t` at `at` owns.
    pub(super) fn drop_at(&mut self, t: Ty, at: Value) {
        if self.l.owns(t) {
            let record = self.record(t);
            self.rt_call(offset_of!(Rt, drop), &[self.ctx, record, at]);
        }
    }

    fn drop_all(&mut self, dropped: Dropped) {
        for (t, owner) in dropped {
            let at = match owner {
                Owner::Local(l) => self.b.use_var(self.vars[l]),
                Owner::Temp(v) => v,
            };
            self.drop_at(t, at);
        }
    }

    /// Type `t`'s record, from its slot after the definitions' (M4-05 section 7).
    pub(super) fn record(&mut self, t: Ty) -> Value {
        let flags = MemFlagsData::trusted();
        let at = offset_of!(Context, slots) as i32;
        let slots = self.b.ins().load(types::I64, flags, self.ctx, at);
        let word = ((super::definitions(self.p) + t) * 8) as i32;
        self.b.ins().load(types::I64, flags, slots, word)
    }

    /// Runs `s`, then drops the temporaries it made.
    pub(super) fn statement(&mut self, s: &Stmt) -> Result<(), Refusal> {
        let mark = self.owners.mark();
        self.stmt(s)?;
        self.drop_temps(mark);
        Ok(())
    }

    pub(super) fn drop_temps(&mut self, mark: usize) {
        let temps = self.owners.temps_since(mark);
        self.drop_all(temps);
    }

    /// A block in its own scope, after binding each of `binds`: its value, owned by no one.
    pub(super) fn scoped(&mut self, binds: &[(Local, Value)], block: &Block) -> Emitted {
        self.owners.enter(false);
        for &(l, v) in binds {
            self.bind_owned(l, v);
        }
        for s in &block.stmts {
            self.statement(s)?;
        }
        let v = match &block.value {
            Some(e) => self.expr(e)?.map(|v| self.yielded(e, v)),
            None => None,
        };
        let dropped = self.owners.leave();
        self.drop_all(dropped);
        Ok(v)
    }

    /// A loop body in its own scope, which `break` and `continue` leave.
    pub(super) fn loop_body(&mut self, block: &Block) -> Result<(), Refusal> {
        self.owners.enter(true);
        for s in &block.stmts {
            self.statement(s)?;
        }
        let dropped = self.owners.leave();
        self.drop_all(dropped);
        Ok(())
    }

    /// Before a `break` or `continue`: drops the loop body's owners.
    pub(super) fn leave_loop(&mut self) {
        let dropped = self.owners.to_loop();
        self.drop_all(dropped);
    }

    /// The value of a block's last expression: its own local moves out, anything else is owned.
    fn yielded(&mut self, e: &Expr, v: Value) -> Value {
        let t = super::ty(e);
        match whole_local(e) {
            Some(l) if self.l.owns(t) && self.owners.disown_local(l) => v,
            _ => self.owned(t, v),
        }
    }

    /// Writes `e`'s value `v` as the result, drops every owner but the local it returns, and
    /// returns status 0.
    pub(super) fn give(&mut self, e: Option<&Expr>, v: Option<Value>) {
        let kept = e.and_then(whole_local).filter(|&l| self.owners.holds(l));
        let result = e.zip(v).map(|(e, v)| {
            let t = super::ty(e);
            (t, if kept.is_some() { v } else { self.owned(t, v) })
        });
        let dropped = self.owners.to_fn(kept);
        self.drop_all(dropped);
        if let (Some(out), Some((t, v))) = (self.out, result) {
            self.put(t, out, 0, v);
        }
        self.ret(0);
    }

    /// Fills the slot at `at` of type `t` with zeros, so it can be dropped before it is built.
    pub(super) fn zeroed(&mut self, t: Ty, at: Value) {
        let zero = self.int_const(0);
        for word in (0..self.size(t) as i32).step_by(8) {
            self.b.ins().store(MemFlagsData::trusted(), zero, at, word);
        }
    }
}

type Emitted = Result<Option<Value>, Refusal>;

/// The local `e` reads whole, if it does.
fn whole_local(e: &Expr) -> Option<Local> {
    match &e.kind {
        ExprKind::Place(p) if p.steps.is_empty() => match p.root {
            Root::Local(l) => Some(l),
            Root::Value(_) => None,
        },
        _ => None,
    }
}
