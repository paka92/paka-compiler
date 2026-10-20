//! Values held in memory (M4-31, M4-32): Strings, Lists, records, enums, `Option` and `Result`.
//! Such a value is the address of its words (docs/decisions/M4-05.md section 8): a local's own
//! stack slot, a temporary slot, a List's item, or a parameter's caller memory.

use super::{Emitted, Gen, Label, Refusal, args, ir_type, words::Op};
use crate::core_form::{Arm, Compound, Expr, Local, Place, Root, Step, Tag, Ty, Type};
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, types,
};
use cranelift_frontend::{FunctionBuilder, Switch};

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
    )
}

/// The address of a new stack slot of `bytes`, at least one word.
pub(super) fn slot(b: &mut FunctionBuilder, bytes: usize) -> Value {
    let words = StackSlotData::new(StackSlotKind::ExplicitSlot, bytes.max(8) as u32, 3);
    let slot = b.create_sized_stack_slot(words);
    b.ins().stack_addr(types::I64, slot, 0)
}

impl Gen<'_> {
    fn held(&self, t: Ty) -> bool {
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
            ref t if held(t) => self.b.ins().iadd_imm_s(at, offset as i64),
            _ => self.b.ins().load(types::I64, flags, at, offset as i32),
        }
    }

    /// Writes `v` of type `t` at `at + offset`: a held value is copied.
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

    /// Binds local `l` to `v`: a held local copies it into its slot.
    pub(super) fn bind(&mut self, l: Local, v: Value) {
        let var = self.vars[l];
        match self.held(self.locals[l]) {
            true => {
                let at = self.b.use_var(var);
                self.copy(at, v, self.size(self.locals[l]));
            }
            false => self.b.def_var(var, v),
        }
    }

    /// The place's root value and type, then its steps' operands, in written order.
    fn operands(&mut self, place: &Place) -> Result<(Value, Ty, Vec<Op>), Refusal> {
        let (root, ty) = match &place.root {
            Root::Local(l) => (self.b.use_var(self.vars[*l]), self.locals[*l]),
            Root::Value(e) => (self.value(e)?, e.ty.expect("a place has a type")),
        };
        let mut ops = vec![];
        for step in &place.steps {
            ops.push(match step {
                Step::Field(i) => Op::Field(*i),
                Step::Item(i, site) => Op::Item(self.value(i)?, *site),
                Step::Slice(a, b, site) => Op::Slice(self.value(a)?, self.value(b)?, *site),
                Step::Key(..) => return Err("a Map step is not compiled yet".into()),
            });
        }
        Ok((root, ty, ops))
    }

    pub(super) fn read(&mut self, place: &Place) -> Result<Value, Refusal> {
        let (root, ty, ops) = self.operands(place)?;
        if ops.is_empty() {
            return Ok(root);
        }
        let (at, ty, offset) = self.walk(root, ty, &ops, false);
        Ok(self.get(ty, at, offset))
    }

    /// `place = e` or `place op= e`: the steps' operands, then `e`, then the walk with its
    /// checks, then the write.
    pub(super) fn assign(
        &mut self,
        place: &Place,
        op: Option<&Compound>,
        e: &Expr,
    ) -> Result<(), Refusal> {
        let (root, ty, ops) = self.operands(place)?;
        let mut v = self.value(e)?;
        let (at, ty, offset) = self.walk(root, ty, &ops, true);
        let whole = ops.is_empty();
        if let Some(op) = op {
            let old = if whole {
                root
            } else {
                self.get(ty, at, offset)
            };
            v = self.compound(op, old, v)?;
        }
        match (whole, &place.root) {
            (true, &Root::Local(l)) => self.bind(l, v),
            _ => self.put(ty, at, offset, v),
        }
        Ok(())
    }

    /// A record or variant of type `t`, built in a new slot; each field is stored as it is
    /// evaluated, so a later field cannot change an earlier one's copy.
    pub(super) fn construct(&mut self, t: Ty, alternative: usize, fields: &[Expr]) -> Emitted {
        ir_type(self.p, self.l, t)?;
        let size = self.size(t);
        let at = slot(&mut self.b, size);
        if self.l.get(t).tagged {
            let tag = self.int_const(alternative as i64);
            self.b.ins().store(MemFlagsData::trusted(), tag, at, 0);
        }
        let places = self.l.get(t).alternatives[alternative].clone();
        for (field, place) in fields.iter().zip(places) {
            let v = self.value(field)?;
            self.put(place.ty, at, place.offset, v);
        }
        Ok(Some(at))
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
            if let [t] = &arm.tags[..] {
                let places = self.l.get(ty).alternatives[alternative(t)?].clone();
                for (bind, place) in arm.binds.iter().zip(places) {
                    if let Some(l) = bind {
                        let v = self.get(place.ty, at, place.offset);
                        self.bind(*l, v);
                    }
                }
            }
            let body = self.b.create_block();
            self.b.ins().jump(body, &args(&[]));
            bodies.push((body, Some(&arm.body)));
        }
        self.join(e, bodies)
    }

    /// `unwrap_or(opt, fallback)`: the payload when `opt` is `Some`, read either way and
    /// dropped by a select.
    pub(super) fn unwrap_or(&mut self, opt: Ty, at: Value, fallback: Value) -> Value {
        let payload = self.l.get(opt).alternatives[0][0];
        let tag = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), at, 0);
        let v = self.get(payload.ty, at, payload.offset);
        self.b.ins().select(tag, fallback, v)
    }

    /// `ok_or(opt, error)`: `Some` and `Ok` share tag 0, `None` and `Err` tag 1.
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
        Tag::Effect(_) => Err("an Effect is not compiled yet".into()),
    }
}
