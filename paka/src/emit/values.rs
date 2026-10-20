//! Values held in memory (M4-31, M4-32, M4-36): Strings, Lists, records, enums, `Option`,
//! `Result` and `Effect`.
//! Such a value is the address of its words (docs/decisions/M4-05.md section 8): a local's own
//! stack slot, a temporary slot, a List's item, or a parameter's caller memory.

use super::{Emitted, Gen, Label, Refusal, args, words::Op};
use crate::core_form::{Arm, Compound, Expr, Local, Place, Root, Step, Tag, Ty, Type};
use crate::layout;
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, types,
};
use cranelift_frontend::{FunctionBuilder, Switch};
use paka_rt::abi::dev::Rt;
use std::mem::offset_of;

/// A place's root value and type, then its steps' operands.
pub(super) type Operands = (Value, Ty, Vec<Op>);

/// Whether a value of type `t` is held in memory.
pub(super) fn held(t: &Type) -> bool {
    matches!(
        t,
        Type::String
            | Type::List(_)
            | Type::Record(_)
            | Type::Enum(_)
            | Type::Option(_)
            | Type::Result(..)
            | Type::Effect(_)
            | Type::Map(..)
    )
}

/// The address of a new stack slot of `bytes`, at least one word.
pub(super) fn slot(b: &mut FunctionBuilder, bytes: usize) -> Value {
    let words = StackSlotData::new(StackSlotKind::ExplicitSlot, bytes.max(8) as u32, 3);
    let slot = b.create_sized_stack_slot(words);
    b.ins().stack_addr(types::I64, slot, 0)
}

impl Gen<'_> {
    pub(super) fn held(&self, t: Ty) -> bool {
        held(&self.p.types[t])
    }

    pub(super) fn size(&self, t: Ty) -> usize {
        self.l.get(t).size
    }

    /// Copies `bytes` from `from` to `to` a word at a time, so a source at or after its target
    /// in the same value, as a field is, copies right.
    fn copy(&mut self, to: Value, from: Value, bytes: usize) {
        let flags = MemFlagsData::trusted();
        for at in (0..bytes as i32).step_by(8) {
            let word = self.b.ins().load(types::I64, flags, from, at);
            self.b.ins().store(flags, word, to, at);
        }
    }

    /// The value of type `t` at `at + offset`: a held value is its address.
    pub(super) fn get(&mut self, t: Ty, at: Value, offset: usize) -> Value {
        let flags = MemFlagsData::trusted();
        match self.p.types[t] {
            Type::Float => self.b.ins().load(types::F64, flags, at, offset as i32),
            Type::Bool => {
                let word = self.b.ins().load(types::I64, flags, at, offset as i32);
                self.b.ins().ireduce(types::I8, word)
            }
            ref t if held(t) && offset == 0 => at,
            ref t if held(t) => self.b.ins().iadd_imm_s(at, offset as i64),
            _ => self.b.ins().load(types::I64, flags, at, offset as i32),
        }
    }

    /// The field at `place` of the value at `at`, followed through its box.
    pub(super) fn field(&mut self, place: &layout::Place, at: Value) -> Value {
        if !place.boxed {
            return self.get(place.ty, at, place.offset);
        }
        let block =
            (self.b.ins()).load(types::I64, MemFlagsData::trusted(), at, place.offset as i32);
        self.get(place.ty, block, 0)
    }

    /// Writes `v` of type `t` at `at + offset`: a held value's words are copied.
    pub(super) fn put(&mut self, t: Ty, at: Value, offset: usize, v: Value) {
        if self.held(t) {
            let to = self.b.ins().iadd_imm_s(at, offset as i64);
            return self.copy(to, v, self.size(t));
        }
        let v = self.widen(v);
        self.b
            .ins()
            .store(MemFlagsData::trusted(), v, at, offset as i32);
    }

    /// Binds local `l` to `v`: a held local takes its words into its slot, and an `inout`
    /// parameter into its caller's place.
    pub(super) fn bind(&mut self, l: Local, v: Value) {
        let var = self.vars[l];
        if self.by_pointer(l) {
            let at = self.b.use_var(var);
            return self.put(self.locals[l], at, 0, v);
        }
        match self.held(self.locals[l]) {
            true => {
                let at = self.b.use_var(var);
                self.copy(at, v, self.size(self.locals[l]));
            }
            false => self.b.def_var(var, v),
        }
    }

    /// Whether local `l` is an `inout` parameter its variable only points to: one not held in
    /// memory, whose variable is already an address.
    fn by_pointer(&self, l: Local) -> bool {
        self.params.get(l).is_some_and(|q| q.inout) && !self.held(self.locals[l])
    }

    fn local(&mut self, l: Local) -> Value {
        let v = self.b.use_var(self.vars[l]);
        match self.by_pointer(l) {
            true => self.get(self.locals[l], v, 0),
            false => v,
        }
    }

    /// The place's root value and type, then its steps' operands, in written order.
    fn operands(&mut self, place: &Place) -> Result<Operands, Refusal> {
        let (root, ty) = match &place.root {
            Root::Local(l) => (self.local(*l), self.locals[*l]),
            Root::Value(e) => (self.value(e)?, e.ty.expect("a place has a type")),
        };
        let mut ops = vec![];
        for step in &place.steps {
            ops.push(match step {
                Step::Field(i) => Op::Field(*i),
                Step::Item(i, site) => Op::Item(self.value(i)?, *site),
                Step::Slice(a, b, site) => Op::Slice(self.value(a)?, self.value(b)?, *site),
                Step::Key(k, site) => Op::Key(self.value(k)?, *site, false),
            });
        }
        Ok((root, ty, ops))
    }

    /// The place's value, which a List slice copies into a new List.
    pub(super) fn read(&mut self, place: &Place) -> Result<Value, Refusal> {
        let (v, ty) = self.borrow(place)?;
        Ok(match place.steps.last() {
            Some(Step::Slice(.., site)) if self.p.types[ty] != Type::String => {
                let copy = self.copy_of(ty, v, self.site(*site));
                self.temp(ty, copy)
            }
            _ => v,
        })
    }

    /// The place's value and type, read where it lies: a List slice is a view into its List.
    pub(super) fn borrow(&mut self, place: &Place) -> Result<(Value, Ty), Refusal> {
        let (root, ty, ops) = self.operands(place)?;
        if ops.is_empty() {
            return Ok((root, ty));
        }
        let (at, ty, offset) = self.walk(root, ty, &ops);
        let v = self.get(ty, at, offset);
        // A first field lies where its value does; its own name keeps it from passing for the
        // value's owner.
        let v = match v == root {
            true => self.b.ins().iadd_imm_s(v, 0),
            false => v,
        };
        Ok((v, ty))
    }

    /// An `inout` place's operands, evaluated in the argument's position; a whole local has
    /// none.
    pub(super) fn inout_operands(&mut self, place: &Place) -> Result<Option<Operands>, Refusal> {
        match place.steps.is_empty() {
            true => Ok(None),
            false => self.operands(place).map(Some),
        }
    }

    /// The address and type of an `inout` place, walked with its checks. A local held in a
    /// variable is stored in a slot for the call; `spilled` takes it, to read it back after.
    pub(super) fn address(
        &mut self,
        place: &Place,
        operands: Option<Operands>,
        spilled: &mut Vec<(Local, Value)>,
    ) -> (Value, Ty) {
        if let Some((root, ty, ops)) = operands {
            let (at, ty, offset) = self.walk(root, ty, &ops);
            return (self.b.ins().iadd_imm_s(at, offset as i64), ty);
        }
        let Root::Local(l) = place.root else {
            unreachable!("an inout place is rooted at a local");
        };
        let (at, ty) = (self.b.use_var(self.vars[l]), self.locals[l]);
        if self.held(ty) || self.by_pointer(l) {
            return (at, ty);
        }
        let to = slot(&mut self.b, 8);
        self.put(ty, to, 0, at);
        spilled.push((l, to));
        (to, ty)
    }

    /// Reads back the locals a call's `inout` arguments spilled.
    pub(super) fn reload(&mut self, spilled: Vec<(Local, Value)>) {
        for (l, at) in spilled {
            let v = self.get(self.locals[l], at, 0);
            self.b.def_var(self.vars[l], v);
        }
    }

    /// `place = e` or `place op= e`: the steps' operands, then `e`, then the walk with its
    /// checks, then the write. The new value is owned before the old one is dropped.
    pub(super) fn assign(
        &mut self,
        place: &Place,
        op: Option<&Compound>,
        e: &Expr,
    ) -> Result<(), Refusal> {
        let (root, ty, mut ops) = self.operands(place)?;
        if let (None, Some(Op::Key(.., add))) = (op, ops.last_mut()) {
            *add = true;
        }
        let v = self.value(e)?;
        let (at, ty, offset) = self.walk(root, ty, &ops);
        let whole = ops.is_empty();
        let old = if whole {
            root
        } else {
            self.get(ty, at, offset)
        };
        let v = match op {
            Some(op) => self.compound(op, old, v)?,
            None => v,
        };
        let v = self.owned(ty, v);
        self.drop_at(ty, old);
        match (whole, &place.root) {
            (true, &Root::Local(l)) => self.bind(l, v),
            _ => self.put(ty, at, offset, v),
        }
        Ok(())
    }

    /// A record or variant of type `t`, built in a new slot; each field is stored as it is
    /// evaluated, so a later field cannot change an earlier one's copy. The slot is zeroed and
    /// owned first, so an exit while building drops what it holds so far.
    pub(super) fn construct(&mut self, t: Ty, alternative: usize, fields: &[Expr]) -> Emitted {
        let size = self.size(t);
        let at = slot(&mut self.b, size);
        if self.l.owns(t) {
            self.zeroed(t, at);
            self.temp(t, at);
        }
        if self.l.get(t).tagged {
            let tag = self.int_const(alternative as i64);
            self.b.ins().store(MemFlagsData::trusted(), tag, at, 0);
        }
        let places = self.l.get(t).alternatives[alternative].clone();
        for (field, place) in fields.iter().zip(places) {
            let v = self.value(field)?;
            let v = self.owned(place.ty, v);
            match place.boxed {
                true => self.boxed(place.ty, at, place.offset, v),
                false => self.put(place.ty, at, place.offset, v),
            }
        }
        Ok(Some(at))
    }

    /// Moves `v` of type `t` into a new block, whose address is written at `at + offset`.
    fn boxed(&mut self, t: Ty, at: Value, offset: usize, v: Value) {
        let out = slot(&mut self.b, 8);
        let bytes = self.int_const(self.size(t) as i64);
        self.rt_try(offset_of!(Rt, alloc), &[self.ctx, bytes, out]);
        let flags = MemFlagsData::trusted();
        let block = self.b.ins().load(types::I64, flags, out, 0);
        self.put(t, block, 0, v);
        self.b.ins().store(flags, block, at, offset as i32);
    }

    /// A `match`: one branch per arm, chosen by the subject's tag; an arm binds its one
    /// variant's fields, then runs its block.
    pub(super) fn matched(&mut self, e: &Expr, subject: &Expr, arms: &[Arm]) -> Emitted {
        let ty = subject.ty.expect("a subject has a type");
        let at = self.value(subject)?;
        let tag = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), at, 0);
        let labels: Vec<Label> = arms.iter().map(|_| self.b.create_block()).collect();
        let mut switch = Switch::new();
        for (arm, &label) in arms.iter().zip(&labels) {
            for t in &arm.tags {
                switch.set_entry(alternative(t)? as u128, label);
            }
        }
        switch.emit(&mut self.b, tag, labels[0]);
        let mut bodies = vec![];
        for (arm, &label) in arms.iter().zip(&labels) {
            self.b.switch_to_block(label);
            let mut binds = vec![];
            if let [t] = &arm.tags[..] {
                let places = self.l.get(ty).alternatives[alternative(t)?].clone();
                for (bind, place) in arm.binds.iter().zip(places) {
                    if let Some(l) = bind {
                        binds.push((*l, self.field(&place, at)));
                    }
                }
            }
            let body = self.b.create_block();
            self.b.ins().jump(body, &args(&[]));
            bodies.push((body, Some(&arm.body), binds));
        }
        self.join(e, bodies)
    }

    /// `unwrap_or(opt, fallback)`: the payload when `opt` is `Some`, read either way and
    /// dropped by a select. The result borrows from one of the two.
    pub(super) fn unwrap_or(&mut self, opt: Ty, at: Value, fallback: Value) -> Value {
        let payload = self.l.get(opt).alternatives[0][0];
        let tag = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), at, 0);
        let v = self.get(payload.ty, at, payload.offset);
        self.b.ins().select(tag, fallback, v)
    }

    /// `ok_or(opt, error)`: `Some` and `Ok` share tag 0, `None` and `Err` tag 1. The result's
    /// payload borrows from `opt` or `error`.
    pub(super) fn ok_or(&mut self, (opt, at): (Ty, Value), result: Ty, error: Value) -> Value {
        let size = self.size(result);
        let to = slot(&mut self.b, size);
        let tag = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), at, 0);
        self.b.ins().store(MemFlagsData::trusted(), tag, to, 0);
        let [some, none, merge] = [(); 3].map(|()| self.b.create_block());
        self.b.ins().brif(tag, none, &[], some, &[]);
        self.b.switch_to_block(some);
        let (from, ok) = (
            self.l.get(opt).alternatives[0][0],
            self.l.get(result).alternatives[0][0],
        );
        let v = self.get(from.ty, at, from.offset);
        self.put(ok.ty, to, ok.offset, v);
        self.b.ins().jump(merge, &[]);
        self.b.switch_to_block(none);
        let err = self.l.get(result).alternatives[1][0];
        self.put(err.ty, to, err.offset, error);
        self.b.ins().jump(merge, &[]);
        self.b.switch_to_block(merge);
        to
    }
}

/// The variant's position: its tag word, and its alternative in the layout.
pub(super) fn alternative(tag: &Tag) -> Result<usize, Refusal> {
    match tag {
        Tag::User(_, i) => Ok(*i),
        Tag::Some | Tag::Ok => Ok(0),
        Tag::None | Tag::Err => Ok(1),
        Tag::Effect(e) => Ok(*e as usize),
    }
}
