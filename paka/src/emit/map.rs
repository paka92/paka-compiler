//! Maps (M4-38b, docs/decisions/M2-14.md sections 1.9 and 1.11): a Map is the address of its one
//! word, like any held value; the runtime's `rt.map` finds, adds and removes entries.

use super::values::slot;
use super::{Gen, Refusal, ir_type};
use crate::core_form::{Expr, Site, Ty, Type};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, types};
use paka_rt::abi::dev::Rt;
use paka_rt::abi::dev::map::*;
use std::mem::offset_of;

/// The bytes before a Map block's entries: its length and capacity.
const HEAD: i64 = 16;

impl Gen<'_> {
    /// A Map literal: an empty Map owned first, then each entry added as it is evaluated; a later
    /// equal key replaces the value.
    pub(super) fn map(&mut self, t: Ty, entries: &[(Expr, Expr)]) -> Result<Value, Refusal> {
        ir_type(self.p, self.l, t)?;
        let Type::Map(_, v) = self.p.types[t] else {
            unreachable!("a Map literal is a Map");
        };
        let out = slot(&mut self.b, 8);
        let zero = self.int_const(0);
        self.b.ins().store(MemFlagsData::trusted(), zero, out, 0);
        self.temp(t, out);
        for (key, value) in entries {
            let (key, value) = (self.value(key)?, self.value(value)?);
            // A literal has no site of its own; only a full heap can trap here.
            let at = self.rt_map(PUT, 0, t, out, key);
            let old = self.get(v, at, 0);
            let value = self.owned(v, value);
            self.drop_at(v, old);
            self.put(v, at, 0, value);
        }
        Ok(out)
    }

    /// The address of the value under `key` in the Map at `at` of type `t`: `add` makes a missing
    /// entry, else a missing key traps `out_of_bounds` at `site`.
    pub(super) fn entry(&mut self, at: Value, t: Ty, key: Value, site: Site, add: bool) -> Value {
        let site = self.site(site);
        self.rt_map(if add { PUT } else { AT }, site, t, at, key)
    }

    /// The number of entries of the Map at `at`.
    pub(super) fn map_len(&mut self, at: Value) -> Value {
        let flags = MemFlagsData::trusted();
        let word = self.b.ins().load(types::I64, flags, at, 0);
        let [some, done] = [(); 2].map(|()| self.b.create_block());
        let n = self.b.append_block_param(done, types::I64);
        let zero = self.int_const(0);
        self.b
            .ins()
            .brif(word, some, &[], done, &super::args(&[zero]));
        self.b.switch_to_block(some);
        let len = self.b.ins().load(types::I64, flags, word, 0);
        self.b.ins().jump(done, &super::args(&[len]));
        self.b.switch_to_block(done);
        n
    }

    /// The address of entry `i`'s key in the Map block `block` of type `t`; its value follows.
    pub(super) fn entry_at(&mut self, t: Ty, block: Value, i: Value) -> Value {
        let Type::Map(k, v) = self.p.types[t] else {
            unreachable!("an entry is a Map's");
        };
        let size = (self.size(k) + self.size(v)) as i64;
        let skip = self.b.ins().imul_imm_s(i, size);
        let at = self.b.ins().iadd(block, skip);
        self.b.ins().iadd_imm_s(at, HEAD)
    }

    /// `get(m, key)`: `Some` of a copy of the value, else `None`, as an Option of type `opt`.
    pub(super) fn get_key(
        &mut self,
        (t, m): (Ty, Value),
        key: Value,
        opt: Ty,
        site: Site,
    ) -> Value {
        let Type::Map(_, v) = self.p.types[t] else {
            unreachable!("get takes a Map");
        };
        let flags = MemFlagsData::trusted();
        let at = self.rt_map(FIND, 0, t, m, key);
        let size = self.size(opt);
        let out = slot(&mut self.b, size);
        let payload = self.l.get(opt).alternatives[0][0].offset;
        let [some, none, done] = [(); 3].map(|()| self.b.create_block());
        self.b.ins().brif(at, some, &[], none, &[]);
        self.b.switch_to_block(some);
        let value = self.get(v, at, 0);
        let value = match self.l.owns(v) {
            true => self.copy_of(v, value, self.site(site)),
            false => value,
        };
        self.put(v, out, payload, value);
        let tag = self.int_const(0);
        self.b.ins().store(flags, tag, out, 0);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(none);
        let tag = self.int_const(1);
        self.b.ins().store(flags, tag, out, 0);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
        self.temp(opt, out)
    }

    /// `has(m, key)`.
    pub(super) fn has_key(&mut self, t: Ty, m: Value, key: Value) -> Value {
        let at = self.rt_map(FIND, 0, t, m, key);
        self.b.ins().icmp_imm_s(IntCC::NotEqual, at, 0)
    }

    /// `keys(m)` or `values(m)`: a new List of type `list`.
    pub(super) fn listed(
        &mut self,
        (t, m): (Ty, Value),
        keys: bool,
        list: Ty,
        site: Site,
    ) -> Value {
        let out = slot(&mut self.b, 24);
        let site = self.site(site);
        let zero = self.int_const(0);
        self.rt_map_into(if keys { KEYS } else { VALUES }, site, t, m, zero, out);
        self.temp(list, out)
    }

    /// `remove(&m, key)` on the Map at `m` of type `t`.
    pub(super) fn remove_key(&mut self, t: Ty, m: Value, key: Value) {
        let zero = self.int_const(0);
        self.rt_map_into(REMOVE, 0, t, m, key, zero);
    }

    /// Map row `row` that gives an address.
    fn rt_map(&mut self, row: u64, site: u64, t: Ty, m: Value, key: Value) -> Value {
        let out = slot(&mut self.b, 8);
        self.rt_map_into(row, site, t, m, key, out);
        self.b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), out, 0)
    }

    fn rt_map_into(&mut self, row: u64, site: u64, t: Ty, m: Value, key: Value, out: Value) {
        let Type::Map(k, v) = self.p.types[t] else {
            unreachable!("a Map row takes a Map");
        };
        let string = matches!(self.p.types[k], Type::String);
        let shape = self.size(v) as u64 | if string { STRING_KEY } else { 0 };
        let given = [
            self.ctx,
            self.int_const(row as i64),
            self.int_const(site as i64),
            self.record(v),
            self.int_const(shape as i64),
            m,
            key,
            out,
        ];
        self.rt_try(offset_of!(Rt, map), &given);
    }
}
