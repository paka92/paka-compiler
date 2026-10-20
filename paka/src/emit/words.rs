//! Strings and Lists (M4-32): literals, the String rows of `rt.text`, and List items and slices
//! with their bounds checks inline. Until M4-33 places copies and frees, no block is changed in
//! place: a write through a List item copies the List first, so values sharing a block never
//! see each other's writes.

use super::values::slot;
use super::{Gen, Refusal, ir_type, ty};
use crate::core_form::{Bin, Block, Expr, ExprKind, Local, Site, Ty, Type};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, types};
use paka_rt::abi::dev::words::{COMPARE, JOIN, SHOW};
use paka_rt::abi::dev::{self as rt, Rt};
use std::mem::offset_of;

/// A place step, its operands already evaluated.
#[derive(Clone, Copy)]
pub(super) enum Op {
    Field(usize),
    Item(Value, Site),
    Slice(Value, Value, Site),
}

/// The words of a String and a List.
const PTR: i32 = 0;
const LEN: i32 = 8;
const CHARS: i32 = 16;
const CAP: i32 = 24;

impl Gen<'_> {
    /// Follows `ops` from the value at `root` of type `ty`: the address, type and offset reached.
    /// A write copies each List it steps into, and stores the copy back first.
    pub(super) fn walk(
        &mut self,
        root: Value,
        mut ty: Ty,
        ops: &[Op],
        write: bool,
    ) -> (Value, Ty, usize) {
        let (mut at, mut offset) = (root, 0);
        for &op in ops {
            if let Op::Field(i) = op {
                let field = self.l.get(ty).alternatives[0][i];
                (offset, ty) = (offset + field.offset, field.ty);
                continue;
            }
            let here = self.b.ins().iadd_imm_s(at, offset as i64);
            offset = 0;
            let item = match self.p.types[ty] {
                Type::List(item) => Some(item),
                _ => None,
            };
            at = match (op, item) {
                (Op::Item(i, site), Some(item)) => {
                    ty = item;
                    self.item(here, item, i, site, write)
                }
                (Op::Slice(a, b, site), Some(item)) => self.slice(here, item, a, b, site),
                (Op::Item(i, site), None) => self.text(rt::words::INDEX, site, &[here, i]),
                (Op::Slice(a, b, site), None) => self.text(rt::words::SLICE, site, &[here, a, b]),
                (Op::Field(_), _) => unreachable!("taken above"),
            };
        }
        (at, ty, offset)
    }

    /// The address of item `i` of the List at `at`, after the bounds check.
    fn item(&mut self, at: Value, item: Ty, i: Value, site: Site, write: bool) -> Value {
        let flags = MemFlagsData::trusted();
        let len = self.b.ins().load(types::I64, flags, at, LEN);
        let outside = (self.b.ins()).icmp(IntCC::UnsignedGreaterThanOrEqual, i, len);
        self.check(outside, rt::OUT_OF_BOUNDS, site, &[i, len]);
        let size = self.size(item);
        if write {
            let from = self.b.ins().load(types::I64, flags, at, PTR);
            self.new_list(site, from, len, size, at);
        }
        let ptr = self.b.ins().load(types::I64, flags, at, PTR);
        let skip = self.b.ins().imul_imm_s(i, size as i64);
        self.b.ins().iadd(ptr, skip)
    }

    /// Items `a` up to `b` excluded of the List at `at`, after the bounds check, as a new List.
    fn slice(&mut self, at: Value, item: Ty, a: Value, b: Value, site: Site) -> Value {
        let flags = MemFlagsData::trusted();
        let len = self.b.ins().load(types::I64, flags, at, LEN);
        let reversed = self.b.ins().icmp(IntCC::UnsignedGreaterThan, a, b);
        let past = self.b.ins().icmp(IntCC::UnsignedGreaterThan, b, len);
        let outside = self.b.ins().bor(reversed, past);
        self.check(outside, rt::OUT_OF_BOUNDS, site, &[a, b, len]);
        let size = self.size(item);
        let ptr = self.b.ins().load(types::I64, flags, at, PTR);
        let skip = self.b.ins().imul_imm_s(a, size as i64);
        let from = self.b.ins().iadd(ptr, skip);
        let count = self.b.ins().isub(b, a);
        let out = slot(&mut self.b, 24);
        self.new_list(site, from, count, size, out);
        out
    }

    /// `rt.list`: `count` items of `size` bytes copied from `from`, or none when it is 0, as a
    /// new List written to `out`.
    fn new_list(&mut self, site: Site, from: Value, count: Value, size: usize, out: Value) {
        let at = self.int_const(self.site(site) as i64);
        let size = self.int_const(size as i64);
        self.rt_try(
            offset_of!(Rt, list),
            &[self.ctx, at, from, count, size, out],
        );
    }

    /// A List literal: the block first, then each item stored as it is evaluated.
    pub(super) fn list(&mut self, t: Ty, items: &[Expr]) -> Result<Value, Refusal> {
        ir_type(self.p, self.l, t)?;
        let Type::List(item) = self.p.types[t] else {
            unreachable!("a List literal is a List");
        };
        let size = self.size(item);
        let out = slot(&mut self.b, 24);
        let (none, count) = (self.int_const(0), self.int_const(items.len() as i64));
        // A literal has no site of its own; only a full heap can trap here.
        self.new_list(0, none, count, size, out);
        for (i, e) in items.iter().enumerate() {
            let v = self.value(e)?;
            let ptr = (self.b.ins()).load(types::I64, MemFlagsData::trusted(), out, PTR);
            self.put(item, ptr, i * size, v);
        }
        Ok(out)
    }

    /// `for l in xs`: each item of the List as it was when the loop began.
    pub(super) fn each(&mut self, l: Local, xs: &Expr, body: &Block) -> Result<(), Refusal> {
        let Type::List(item) = self.p.types[ty(xs)] else {
            unreachable!("a List walk is over a List");
        };
        let size = self.size(item);
        let at = self.value(xs)?;
        let flags = MemFlagsData::trusted();
        let ptr = self.b.ins().load(types::I64, flags, at, PTR);
        let len = self.b.ins().load(types::I64, flags, at, LEN);
        let i = self.b.declare_var(types::I64);
        let zero = self.int_const(0);
        self.b.def_var(i, zero);
        let [head, inner, next, exit] = [(); 4].map(|()| self.b.create_block());
        self.b.ins().jump(head, &[]);
        self.b.switch_to_block(head);
        self.poll();
        let n = self.b.use_var(i);
        let more = self.b.ins().icmp(IntCC::UnsignedLessThan, n, len);
        self.b.ins().brif(more, inner, &[], exit, &[]);
        self.b.switch_to_block(inner);
        let skip = self.b.ins().imul_imm_s(n, size as i64);
        let address = self.b.ins().iadd(ptr, skip);
        let v = self.get(item, address, 0);
        self.bind(l, v);
        self.body(next, exit, body)?;
        self.b.ins().jump(next, &[]);
        self.b.switch_to_block(next);
        let n = self.b.use_var(i);
        let step = self.b.ins().iadd_imm_s(n, 1);
        self.b.def_var(i, step);
        self.b.ins().jump(head, &[]);
        self.b.switch_to_block(exit);
        Ok(())
    }

    /// A String literal: a new block of its bytes rounded up to whole words, stored a word at a
    /// time; the empty String takes no block.
    pub(super) fn literal(&mut self, s: &str) -> Value {
        let flags = MemFlagsData::trusted();
        let out = slot(&mut self.b, 32);
        let cap = s.len().next_multiple_of(8);
        let ptr = match cap {
            0 => self.int_const(0),
            _ => {
                let to = slot(&mut self.b, 8);
                let bytes = self.int_const(cap as i64);
                self.rt_try(offset_of!(Rt, alloc), &[self.ctx, bytes, to]);
                let ptr = self.b.ins().load(types::I64, flags, to, 0);
                for (i, chunk) in s.as_bytes().chunks(8).enumerate() {
                    let mut word = [0; 8];
                    word[..chunk.len()].copy_from_slice(chunk);
                    let word = self.int_const(i64::from_le_bytes(word));
                    self.b.ins().store(flags, word, ptr, (i * 8) as i32);
                }
                ptr
            }
        };
        let [bytes, chars, cap] =
            [s.len(), s.chars().count(), cap].map(|n| self.int_const(n as i64));
        for (w, at) in [ptr, bytes, chars, cap]
            .into_iter()
            .zip([PTR, LEN, CHARS, CAP])
        {
            self.b.ins().store(flags, w, out, at);
        }
        out
    }

    /// `rt.text` row `row` on up to three words; COMPARE and CODE_AT give an Int, the others a
    /// String.
    pub(super) fn text(&mut self, row: u64, site: Site, operands: &[Value]) -> Value {
        let int = matches!(row, COMPARE | rt::words::CODE_AT);
        let out = slot(&mut self.b, if int { 8 } else { 32 });
        let mut given = vec![
            self.ctx,
            self.int_const(row as i64),
            self.int_const(self.site(site) as i64),
        ];
        given.extend(operands.iter().map(|&v| self.widen(v)));
        while given.len() < 6 {
            given.push(self.int_const(0));
        }
        given.push(out);
        self.rt_try(offset_of!(Rt, text), &given);
        match int {
            true => (self.b.ins()).load(types::I64, MemFlagsData::trusted(), out, 0),
            false => out,
        }
    }

    /// Interpolation: the parts joined in order, an Int in decimal and a Bool as `true` or
    /// `false`; a `memory` trap at the opening quote past the cap.
    pub(super) fn interpolate(&mut self, parts: &[Expr], site: Site) -> Result<Value, Refusal> {
        let mut text = None;
        for part in parts {
            let v = self.steady(part)?;
            let v = match self.p.types[ty(part)] {
                Type::Int => self.text(SHOW, site, &[v]),
                Type::Bool => {
                    let one = self.int_const(1);
                    self.text(SHOW, site, &[v, one])
                }
                _ => v,
            };
            text = Some(match text {
                None => v,
                Some(t) => self.text(JOIN, site, &[t, v]),
            });
        }
        Ok(text.unwrap_or_else(|| self.literal("")))
    }

    /// `x + y`, `x == y` and the order of two Strings.
    pub(super) fn text_binary(&mut self, op: &Bin, x: &Expr, y: &Expr) -> Result<Value, Refusal> {
        let x = self.steady(x)?;
        let y = self.value(y)?;
        Ok(match op {
            Bin::Join(site) => self.text(JOIN, *site, &[x, y]),
            _ => {
                let order = self.text(COMPARE, 0, &[x, y]);
                let zero = self.int_const(0);
                self.compare(op, false, order, zero)
            }
        })
    }

    /// A String's code point count or a List's length.
    pub(super) fn length(&mut self, at: Value, string: bool) -> Value {
        let word = if string { CHARS } else { LEN };
        (self.b.ins()).load(types::I64, MemFlagsData::trusted(), at, word)
    }

    /// The value of `e`; a String read from a place is copied, so evaluating a later operand
    /// cannot change it.
    fn steady(&mut self, e: &Expr) -> Result<Value, Refusal> {
        let v = self.value(e)?;
        if !matches!(e.kind, ExprKind::Place(_)) || self.p.types[ty(e)] != Type::String {
            return Ok(v);
        }
        let to = slot(&mut self.b, 32);
        self.put(ty(e), to, 0, v);
        Ok(to)
    }
}
