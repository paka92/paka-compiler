//! Equality of values of any type (M4-38, docs/decisions/M2-09.md section 2), emitted inline: a
//! List item by item, a Map entry by entry in key order, a record field by field, a variant by
//! its tag and then its fields. Floats compare as numbers, so NaN differs from itself. A
//! recursive type is refused.

use super::values::{held, slot};
use super::{Gen, Refusal, args, ir_type};
use crate::core_form::{Ty, Type};
use crate::layout::Place;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlagsData, Value, types};
use cranelift_frontend::Switch;
use paka_rt::abi::dev::words::COMPARE;

impl Gen<'_> {
    /// Whether `x` and `y` of type `t` are equal, as a Bool.
    pub(super) fn equal(&mut self, t: Ty, x: Value, y: Value) -> Result<Value, Refusal> {
        self.equal_in(t, x, y, &mut vec![])
    }

    /// The first position of `v` of type `item` in the List at `xs`, or -1.
    pub(super) fn position(&mut self, item: Ty, xs: Value, v: Value) -> Result<Value, Refusal> {
        let flags = MemFlagsData::trusted();
        let len = self.b.ins().load(types::I64, flags, xs, 8);
        let [head, body, done] = [(); 3].map(|()| self.b.create_block());
        let i = self.b.append_block_param(head, types::I64);
        self.b.append_block_param(done, types::I64);
        let zero = self.int_const(0);
        self.b.ins().jump(head, &args(&[zero]));
        self.b.switch_to_block(head);
        let more = self.b.ins().icmp(IntCC::UnsignedLessThan, i, len);
        let none = self.int_const(-1);
        self.b.ins().brif(more, body, &[], done, &args(&[none]));
        self.b.switch_to_block(body);
        let at = self.item_at(item, xs, i);
        let x = self.get(item, at, 0);
        let eq = self.equal(item, x, v)?;
        let next = self.b.ins().iadd_imm_s(i, 1);
        self.b
            .ins()
            .brif(eq, done, &args(&[i]), head, &args(&[next]));
        self.b.switch_to_block(done);
        Ok(self.b.block_params(done)[0])
    }

    /// `Some(i)` when `i` is a position and `None` when it is -1, as an `Option<Int>` of type `opt`.
    pub(super) fn found(&mut self, opt: Ty, i: Value) -> Value {
        let flags = MemFlagsData::trusted();
        let size = self.size(opt);
        let at = slot(&mut self.b, size);
        let none = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, i, 0);
        let tag = self.b.ins().uextend(types::I64, none);
        self.b.ins().store(flags, tag, at, 0);
        let payload = self.l.get(opt).alternatives[0][0].offset;
        self.b.ins().store(flags, i, at, payload as i32);
        at
    }

    fn equal_in(
        &mut self,
        t: Ty,
        x: Value,
        y: Value,
        seen: &mut Vec<Ty>,
    ) -> Result<Value, Refusal> {
        ir_type(self.p, self.l, t)?;
        if seen.contains(&t) {
            return Err("equality of a recursive type is not compiled yet".into());
        }
        seen.push(t);
        let eq = match self.p.types[t] {
            Type::Float => self.b.ins().fcmp(FloatCC::Equal, x, y),
            Type::String => {
                let order = self.text(COMPARE, self.first, &[x, y]);
                self.b.ins().icmp_imm_s(IntCC::Equal, order, 0)
            }
            Type::List(item) => self.equal_items(item, x, y, seen)?,
            Type::Map(..) => self.equal_entries(t, x, y, seen)?,
            ref u if held(u) => self.equal_fields(t, x, y, seen)?,
            _ => self.b.ins().icmp(IntCC::Equal, x, y),
        };
        seen.pop();
        Ok(eq)
    }

    /// Two Lists: the same length, then each pair of items until one differs.
    fn equal_items(
        &mut self,
        item: Ty,
        x: Value,
        y: Value,
        seen: &mut Vec<Ty>,
    ) -> Result<Value, Refusal> {
        let flags = MemFlagsData::trusted();
        let n = self.b.ins().load(types::I64, flags, x, 8);
        let m = self.b.ins().load(types::I64, flags, y, 8);
        self.pairwise(n, m, |g, i| {
            let (a, b) = (g.item_at(item, x, i), g.item_at(item, y, i));
            let (a, b) = (g.get(item, a, 0), g.get(item, b, 0));
            g.equal_in(item, a, b, seen)
        })
    }

    /// Two Maps: the same length, then each pair of entries, in key order, until one differs.
    fn equal_entries(
        &mut self,
        t: Ty,
        x: Value,
        y: Value,
        seen: &mut Vec<Ty>,
    ) -> Result<Value, Refusal> {
        let Type::Map(k, v) = self.p.types[t] else {
            unreachable!("Map equality is of Maps");
        };
        let (n, m) = (self.map_len(x), self.map_len(y));
        let flags = MemFlagsData::trusted();
        let (x, y) = (
            self.b.ins().load(types::I64, flags, x, 0),
            self.b.ins().load(types::I64, flags, y, 0),
        );
        let offset = self.size(k);
        self.pairwise(n, m, |g, i| {
            let (a, b) = (g.entry_at(t, x, i), g.entry_at(t, y, i));
            let (ka, kb) = (g.get(k, a, 0), g.get(k, b, 0));
            let same = g.equal_in(k, ka, kb, seen)?;
            let (va, vb) = (g.get(v, a, offset), g.get(v, b, offset));
            let eq = g.equal_in(v, va, vb, seen)?;
            Ok(g.b.ins().band(same, eq))
        })
    }

    /// Whether lengths `n` and `m` agree and `eq` holds at each position below them, asked in
    /// order until it fails.
    fn pairwise(
        &mut self,
        n: Value,
        m: Value,
        mut eq: impl FnMut(&mut Self, Value) -> Result<Value, Refusal>,
    ) -> Result<Value, Refusal> {
        let [head, body, done] = [(); 3].map(|()| self.b.create_block());
        let i = self.b.append_block_param(head, types::I64);
        self.b.append_block_param(done, types::I8);
        let same = self.b.ins().icmp(IntCC::Equal, n, m);
        let zero = self.int_const(0);
        let differ = self.b.ins().iconst(types::I8, 0);
        self.b
            .ins()
            .brif(same, head, &args(&[zero]), done, &args(&[differ]));
        self.b.switch_to_block(head);
        let more = self.b.ins().icmp(IntCC::UnsignedLessThan, i, n);
        let all = self.b.ins().iconst(types::I8, 1);
        self.b.ins().brif(more, body, &[], done, &args(&[all]));
        self.b.switch_to_block(body);
        let eq = eq(self, i)?;
        let next = self.b.ins().iadd_imm_s(i, 1);
        let differ = self.b.ins().iconst(types::I8, 0);
        self.b
            .ins()
            .brif(eq, head, &args(&[next]), done, &args(&[differ]));
        self.b.switch_to_block(done);
        Ok(self.b.block_params(done)[0])
    }

    /// Two records, or two variants: the same tag, then each pair of fields until one differs.
    fn equal_fields(
        &mut self,
        t: Ty,
        x: Value,
        y: Value,
        seen: &mut Vec<Ty>,
    ) -> Result<Value, Refusal> {
        let layout = self.l.get(t);
        let (tagged, alternatives) = (layout.tagged, layout.alternatives.clone());
        let done = self.b.create_block();
        self.b.append_block_param(done, types::I8);
        if !tagged {
            self.fields_equal(&alternatives[0], x, y, done, seen)?;
        } else {
            let flags = MemFlagsData::trusted();
            let (a, b) = (
                self.b.ins().load(types::I64, flags, x, 0),
                self.b.ins().load(types::I64, flags, y, 0),
            );
            let same = self.b.ins().icmp(IntCC::Equal, a, b);
            let labels: Vec<Block> = alternatives.iter().map(|_| self.b.create_block()).collect();
            let (switch, differ) = (self.b.create_block(), self.b.ins().iconst(types::I8, 0));
            self.b.ins().brif(same, switch, &[], done, &args(&[differ]));
            self.b.switch_to_block(switch);
            let mut table = Switch::new();
            for (k, &label) in labels.iter().enumerate() {
                table.set_entry(k as u128, label);
            }
            table.emit(&mut self.b, a, labels[0]);
            for (places, label) in alternatives.iter().zip(labels) {
                self.b.switch_to_block(label);
                self.fields_equal(places, x, y, done, seen)?;
            }
        }
        self.b.switch_to_block(done);
        Ok(self.b.block_params(done)[0])
    }

    /// Jumps to `done` with false at the first pair of fields that differ, else with true.
    fn fields_equal(
        &mut self,
        places: &[Place],
        x: Value,
        y: Value,
        done: Block,
        seen: &mut Vec<Ty>,
    ) -> Result<(), Refusal> {
        for place in places {
            let (a, b) = (self.field(place, x), self.field(place, y));
            let eq = self.equal_in(place.ty, a, b, seen)?;
            let next = self.b.create_block();
            let differ = self.b.ins().iconst(types::I8, 0);
            self.b.ins().brif(eq, next, &[], done, &args(&[differ]));
            self.b.switch_to_block(next);
        }
        let all = self.b.ins().iconst(types::I8, 1);
        self.b.ins().jump(done, &args(&[all]));
        Ok(())
    }

    /// The field at `place` of the value at `at`, followed through its box.
    fn field(&mut self, place: &Place, at: Value) -> Value {
        if !place.boxed {
            return self.get(place.ty, at, place.offset);
        }
        let block =
            (self.b.ins()).load(types::I64, MemFlagsData::trusted(), at, place.offset as i32);
        self.get(place.ty, block, 0)
    }

    /// The address of item `i` of the List at `xs`.
    fn item_at(&mut self, item: Ty, xs: Value, i: Value) -> Value {
        let items = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), xs, 0);
        let size = self.size(item) as i64;
        let offset = self.b.ins().imul_imm_s(i, size);
        self.b.ins().iadd(items, offset)
    }
}
