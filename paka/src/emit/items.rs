//! The List rows that change a List in place, and `list_of` (M4-38, docs/decisions/M2-09.md
//! section 3): one runtime body, `rt.items`, told the item type's record and size. An item
//! handed to it is owned first, so the List takes a value no one else holds.

use super::values::{held, slot};
use super::{Emitted, Gen, Refusal, ir_type, ty};
use crate::core_form::{Arg, Site, Ty, Type};
use cranelift_codegen::ir::Value;
use paka_rt::abi::dev::Rt;
use paka_rt::abi::dev::items::*;
use std::mem::offset_of;

impl Gen<'_> {
    /// List row `name` on its `inout` List: the place's operands in its position, the other
    /// arguments, then the walk to its address with its checks.
    pub(super) fn items(&mut self, name: &str, given: &[Arg], site: Site) -> Emitted {
        let [Arg::Inout(place), rest @ ..] = given else {
            unreachable!("a List row takes its List inout first");
        };
        let operands = self.inout_operands(place)?;
        let mut x = vec![];
        for a in rest {
            let Arg::Value(a) = a else {
                unreachable!("a List row takes one inout argument");
            };
            x.push((ty(a), self.value(a)?));
        }
        let mut spilled = vec![];
        let (xs, list) = self.address(place, operands, &mut spilled);
        let Type::List(item) = self.p.types[list] else {
            unreachable!("a List row's place is a List");
        };
        ir_type(self.p, self.l, item)?;
        if name.starts_with("sort_by") {
            self.sort_by(item, xs, x[0].1, site);
            self.reload(spilled);
            return Ok(None);
        }
        let zero = self.int_const(0);
        let row = row(name);
        let (a, b) = match (row, &x[..]) {
            (APPEND | EXTEND, &[(t, v)]) => (self.given(t, v), zero),
            (INSERT, &[(t, v), (_, at)]) => (self.given(t, v), at),
            (REMOVE_AT, &[(_, i)]) => (i, zero),
            (SWAP, &[(_, i), (_, j)]) => (i, j),
            (SORT, []) => (self.int_const(pass(&self.p.types[item]) as i64), zero),
            _ => (zero, zero),
        };
        let out = match row {
            POP | REMOVE_AT => {
                let size = self.size(item);
                slot(&mut self.b, size)
            }
            _ => zero,
        };
        self.rt_items(row, site, item, [xs, a, b, out]);
        self.reload(spilled);
        Ok(match row {
            POP | REMOVE_AT => {
                let v = self.get(item, out, 0);
                Some(self.temp(item, v))
            }
            _ => None,
        })
    }

    /// `list_of(item, count)`: a new List of type `list`.
    pub(super) fn list_of(&mut self, list: Ty, x: &[Value], site: Site) -> Result<Value, Refusal> {
        let Type::List(item) = self.p.types[list] else {
            unreachable!("list_of makes a List");
        };
        ir_type(self.p, self.l, item)?;
        let a = self.given(item, x[0]);
        let (zero, size) = (self.int_const(0), self.size(list));
        let out = slot(&mut self.b, size);
        self.rt_items(LIST_OF, site, item, [zero, a, x[1], out]);
        Ok(self.temp(list, out))
    }

    /// The address of `v` of type `t` for the runtime to take: owned, and in a slot unless it
    /// is held.
    fn given(&mut self, t: Ty, v: Value) -> Value {
        let v = self.owned(t, v);
        if held(&self.p.types[t]) {
            return v;
        }
        let size = self.size(t);
        let at = slot(&mut self.b, size);
        self.put(t, at, 0, v);
        at
    }

    /// `sort_by(&xs, before)`: a status other than 0 from `before` is passed on as a call's.
    fn sort_by(&mut self, item: Ty, xs: Value, before: Value, site: Site) {
        let given = [
            self.ctx,
            self.int_const(self.size(item) as i64),
            self.int_const(pass(&self.p.types[item]) as i64),
            xs,
            before,
        ];
        let call = self.rt_invoke(offset_of!(Rt, sort_by), &given, true);
        let status = self.b.inst_results(call)[0];
        self.pass_on(status, site);
    }

    fn rt_items(&mut self, row: u64, site: Site, item: Ty, [xs, a, b, out]: [Value; 4]) {
        let given = [
            self.ctx,
            self.int_const(row as i64),
            self.int_const(self.site(site) as i64),
            self.record(item),
            self.int_const(self.size(item) as i64),
            xs,
            self.widen(a),
            self.widen(b),
            out,
        ];
        self.rt_try(offset_of!(Rt, items), &given);
    }
}

/// The `rt.items` row of a List row's name, its signature without the parameters.
fn row(name: &str) -> u64 {
    match name.split('(').next() {
        Some("append") => APPEND,
        Some("extend") => EXTEND,
        Some("pop") => POP,
        Some("insert") => INSERT,
        Some("remove_at") => REMOVE_AT,
        Some("swap") => SWAP,
        Some("reverse") => REVERSE,
        Some("sort") => SORT,
        _ => unreachable!("no List row {name}"),
    }
}

/// How the runtime reads an item of type `item`.
fn pass(item: &Type) -> u64 {
    match item {
        Type::Float => FLOAT,
        t if held(t) => HELD,
        _ => INT,
    }
}
