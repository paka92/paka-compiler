//! Views (M5-24): each node is written into the run's tree buffer through `rt.view` as it is made,
//! children before the list that holds them closes (paka_rt::abi::dev::view). A view's value is a
//! word of no meaning: the node it stands for is the last one written.

use super::values::slot;
use super::{Emitted, Gen, Refusal, ty};
use crate::builtins::ELEMENTS;
use crate::core_form::{Block, Child, Expr, ExprKind, Site, Type};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Value};
use paka_rt::abi::dev::view::*;
use paka_rt::abi::dev::{self as rt, Rt};
use std::mem::offset_of;

impl Gen<'_> {
    pub(super) fn view(&mut self, e: &Expr) -> Emitted {
        match &e.kind {
            ExprKind::Root(lines) => {
                self.view_op(OPEN, &[]);
                self.scoped(&[], lines)?;
                self.view_op(ROOT, &[]);
            }
            ExprKind::Element(row, args, lines, site) => {
                self.element(*row, args, lines.as_ref(), *site)?
            }
            ExprKind::Placeholder(site) => {
                let name = self.p.sites[*site].hole.as_deref().unwrap_or_default();
                let name = self.literal(name);
                self.view_op(HOLE, &[name]);
            }
            ExprKind::Child(c) => {
                self.child(c)?;
                return Ok(None);
            }
            _ => unreachable!("a view node"),
        }
        Ok(Some(self.int_const(0)))
    }

    /// Its arguments first; a Row or Col traps `assert` on a negative gap before its children.
    fn element(
        &mut self,
        row: usize,
        args: &[Expr],
        lines: Option<&Block>,
        site: Site,
    ) -> Result<(), Refusal> {
        let mut x = vec![];
        for a in args {
            x.push(self.borrowed(a)?);
        }
        match ELEMENTS[row].name {
            "Text" => self.view_op(TEXT, &x),
            name @ ("Button" | "Input") => {
                let t = ty(&args[1]);
                let message = match self.held(t) {
                    true => x[1],
                    false => {
                        let at = slot(&mut self.b, 8);
                        self.put(t, at, 0, x[1]);
                        at
                    }
                };
                let record = self.record(t);
                let [size, t] = [self.size(t), t].map(|n| self.int_const(n as i64));
                let op = if name == "Button" { BUTTON } else { INPUT };
                self.view_op(op, &[x[0], message, record, size, t]);
            }
            name => {
                let gap = x[0];
                let bad = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, gap, 0);
                self.check(bad, rt::ASSERT, site, &[]);
                self.view_op(OPEN, &[]);
                self.scoped(&[], lines.expect("a child block"))?;
                let gap = match self.plant {
                    8 => self.b.ins().iadd_imm_s(gap, 1),
                    _ => gap,
                };
                self.view_op(if name == "Row" { ROW } else { COL }, &[gap]);
            }
        }
        Ok(())
    }

    /// The node, then its key.
    fn child(&mut self, c: &Child) -> Result<(), Refusal> {
        self.expr(&c.node)?;
        let (kind, key) = match &c.key {
            Some(k) if self.p.types[ty(k)] == Type::String => (STRING_KEY, self.borrowed(k)?),
            Some(k) => (INT_KEY, self.value(k)?),
            None => (NO_KEY, self.int_const(0)),
        };
        let [line, looped, kind] =
            [c.line as u64, u64::from(c.looped), kind].map(|n| self.int_const(n as i64));
        self.view_op(CHILD, &[line, looped, kind, key]);
        Ok(())
    }

    /// `rt.view(ctx, op, operands...)`, unused operands 0.
    fn view_op(&mut self, op: u64, operands: &[Value]) {
        let mut given = vec![self.ctx, self.int_const(op as i64)];
        given.extend(operands);
        while given.len() < 7 {
            given.push(self.int_const(0));
        }
        self.rt_try(offset_of!(Rt, view), &given);
    }
}
