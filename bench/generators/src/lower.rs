//! Core form to Cranelift IR, for the node kinds the benchmark definitions use.
//!
//! A definition takes the slot table's address, a result place and its arguments, and returns 0
//! or a trap number (docs/decisions/M4-01.md section 4).

use crate::code::{RUNTIME, Trap};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    AbiParam, Block as Label, BlockArg, Function, InstBuilder, MemFlagsData, SigRef, Signature,
    StackSlot, StackSlotData, StackSlotKind, Type as IrType, UserFuncName, Value, types,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use paka::builtins::ROWS;
use paka::core_form::{
    Arith, Bin, Block, Callee, Cmp, Compound, Expr, ExprKind, FloatArith, FnId, Place, Program,
    Root, Stmt, Tag, Ty, Type, Un,
};
use paka_rt::ops::Int;

/// The IR of definition `id`.
pub fn function(
    isa: &dyn TargetIsa,
    p: &Program,
    id: FnId,
    cx: &mut FunctionBuilderContext,
) -> Function {
    let mut func =
        Function::with_name_signature(UserFuncName::user(0, id as u32), signature(isa, p, id));
    let mut b = FunctionBuilder::new(&mut func, cx);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let f = &p.fns[id];
    let vars: Vec<_> = f
        .locals
        .iter()
        .map(|&t| b.declare_var(ir_type(p, t)))
        .collect();
    let params = b.block_params(entry).to_vec();
    for (var, &v) in vars.iter().zip(&params[2..]) {
        b.def_var(*var, v);
    }
    let scratch = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let mut g = Gen {
        b,
        p,
        isa,
        vars,
        slots: params[0],
        out: params[1],
        scratch,
        loops: Vec::new(),
        traps: Default::default(),
        pass: None,
        sigs: vec![None; p.fns.len()],
        float_sigs: [None; 2],
    };
    if let Some(v) = g.block(&f.body) {
        g.b.ins().store(MemFlagsData::trusted(), v, g.out, 0);
    }
    g.ret(0);
    g.finish();
    func
}

/// Slot table address, result place, then the parameters; the status comes back.
fn signature(isa: &dyn TargetIsa, p: &Program, id: FnId) -> Signature {
    let mut sig = Signature::new(isa.default_call_conv());
    let ptr = AbiParam::new(isa.pointer_type());
    sig.params.extend([ptr, ptr]);
    sig.params.extend(
        p.fns[id]
            .params
            .iter()
            .map(|q| AbiParam::new(ir_type(p, q.ty))),
    );
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

fn ir_type(p: &Program, t: Ty) -> IrType {
    match p.types[t] {
        Type::Int | Type::Enum(_) => types::I64,
        Type::Float => types::F64,
        Type::Bool => types::I8,
        ref other => panic!("{other:?} is outside the benchmark"),
    }
}

fn args(values: &[Value]) -> Vec<BlockArg> {
    values.iter().map(|&v| BlockArg::Value(v)).collect()
}

struct Gen<'a> {
    b: FunctionBuilder<'a>,
    p: &'a Program,
    isa: &'a dyn TargetIsa,
    vars: Vec<Variable>,
    slots: Value,
    out: Value,
    /// Where a called definition writes its result.
    scratch: StackSlot,
    /// Each enclosing loop's (continue, break) targets.
    loops: Vec<(Label, Label)>,
    /// Filled at the end: the current block cannot be left half built.
    traps: [Option<Label>; 5],
    /// Returns a callee's nonzero status.
    pass: Option<Label>,
    sigs: Vec<Option<SigRef>>,
    float_sigs: [Option<SigRef>; 2],
}

impl Gen<'_> {
    fn finish(mut self) {
        for (code, label) in self.traps.into_iter().enumerate() {
            if let Some(label) = label {
                self.b.switch_to_block(label);
                self.ret(code as i64);
            }
        }
        if let Some(label) = self.pass {
            self.b.switch_to_block(label);
            let status = self.b.block_params(label)[0];
            self.b.ins().return_(&[status]);
        }
        self.b.seal_all_blocks();
        self.b.finalize(self.isa.frontend_config());
    }

    fn ret(&mut self, status: i64) {
        let status = self.b.ins().iconst(types::I32, status);
        self.b.ins().return_(&[status]);
    }

    /// Continues in a new block that nothing reaches, after a jump or a return.
    fn dead(&mut self) {
        let label = self.b.create_block();
        self.b.switch_to_block(label);
    }

    fn trap_if(&mut self, cond: Value, trap: Trap) {
        let label = *self.traps[trap as usize].get_or_insert_with(|| self.b.create_block());
        let ok = self.b.create_block();
        self.b.ins().brif(cond, label, &[], ok, &[]);
        self.b.switch_to_block(ok);
    }

    fn block(&mut self, block: &Block) -> Option<Value> {
        for s in &block.stmts {
            self.stmt(s);
        }
        self.expr(block.value.as_deref()?)
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let(l, e) => {
                let v = self.value(e);
                self.b.def_var(self.vars[*l], v);
            }
            Stmt::Assign(place, op, e) => {
                let var = self.vars[local(place)];
                let mut v = self.value(e);
                if let Some(op) = op {
                    let old = self.b.use_var(var);
                    v = match op {
                        Compound::Int(a, _) => self.int(a, old, v),
                        Compound::Float(a) => self.float(a, old, v),
                        Compound::Join(_) => panic!("text is outside the benchmark"),
                    };
                }
                self.b.def_var(var, v);
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    let v = self.value(e);
                    self.b.ins().store(MemFlagsData::trusted(), v, self.out, 0);
                }
                self.ret(0);
                self.dead();
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = *self.loops.last().expect("a loop holds it");
                self.b
                    .ins()
                    .jump(if matches!(s, Stmt::Break) { exit } else { next }, &[]);
                self.dead();
            }
            Stmt::While(cond, body) => {
                let [head, inner, exit] = [(); 3].map(|()| self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let c = self.value(cond);
                self.b.ins().brif(c, inner, &[], exit, &[]);
                self.b.switch_to_block(inner);
                self.loops.push((head, exit));
                self.block(body);
                self.loops.pop();
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(exit);
            }
            Stmt::Expr(e) => {
                self.expr(e);
            }
            Stmt::For(..) => panic!("for is outside the benchmark"),
        }
    }

    fn value(&mut self, e: &Expr) -> Value {
        self.expr(e).expect("the node gives a value")
    }

    fn zero(&mut self, t: IrType) -> Value {
        match t {
            types::F64 => self.b.ins().f64const(0.0),
            _ => self.b.ins().iconst(t, 0),
        }
    }

    fn is_float(&self, e: &Expr) -> bool {
        e.ty.is_some_and(|t| self.p.types[t] == Type::Float)
    }

    fn expr(&mut self, e: &Expr) -> Option<Value> {
        Some(match &e.kind {
            ExprKind::Int(n) => self.b.ins().iconst(types::I64, raw(*n)),
            ExprKind::Float(x) => self.b.ins().f64const(f64::from_bits(x.bits())),
            ExprKind::Bool(v) => self.b.ins().iconst(types::I8, i64::from(*v)),
            ExprKind::Place(place) => self.b.use_var(self.vars[local(place)]),
            ExprKind::Variant(tag, fields) if fields.is_empty() => {
                self.b.ins().iconst(types::I64, variant(tag))
            }
            ExprKind::Unary(op, x) => {
                let x = self.value(x);
                match op {
                    Un::Neg(_) => {
                        let zero = self.b.ins().iconst(types::I64, 0);
                        self.int(&Arith::Sub, zero, x)
                    }
                    Un::NegFloat => self.b.ins().fneg(x),
                    Un::Not => self.b.ins().icmp_imm_s(IntCC::Equal, x, 0),
                }
            }
            ExprKind::Binary(op @ (Bin::And | Bin::Or), x, y) => {
                self.short(matches!(op, Bin::And), x, y)
            }
            ExprKind::Binary(op, x, y) => {
                let float = self.is_float(x);
                let (x, y) = (self.value(x), self.value(y));
                match op {
                    Bin::Int(a, _) => self.int(a, x, y),
                    Bin::Float(a) => self.float(a, x, y),
                    Bin::Eq(_) | Bin::Ne(_) => {
                        let eq = matches!(op, Bin::Eq(_));
                        match float {
                            true => self.b.ins().fcmp(
                                if eq {
                                    FloatCC::Equal
                                } else {
                                    FloatCC::NotEqual
                                },
                                x,
                                y,
                            ),
                            false => self.b.ins().icmp(
                                if eq { IntCC::Equal } else { IntCC::NotEqual },
                                x,
                                y,
                            ),
                        }
                    }
                    Bin::Order(c) if float => self.b.ins().fcmp(float_cc(c), x, y),
                    Bin::Order(c) => self.b.ins().icmp(int_cc(c), x, y),
                    Bin::Join(_) | Bin::And | Bin::Or => unreachable!("handled above or outside"),
                }
            }
            ExprKind::Call(Callee::Fn(id), given, _) => {
                let given: Vec<_> = given.iter().map(|a| self.arg(a)).collect();
                self.call(*id, &given)
            }
            ExprKind::Call(Callee::Builtin(row), given, _) => {
                let given: Vec<_> = given.iter().map(|a| self.arg(a)).collect();
                self.builtin(ROWS[*row].1, &given)
            }
            ExprKind::If(c, then, otherwise) => {
                let c = self.value(c);
                let [yes, no] = [(); 2].map(|()| self.b.create_block());
                self.b.ins().brif(c, yes, &[], no, &[]);
                return self.join(e, [(yes, Some(then)), (no, otherwise.as_ref())]);
            }
            ExprKind::Match(scrutinee, arms) => {
                let v = self.value(scrutinee);
                let labels: Vec<_> = arms.iter().map(|_| self.b.create_block()).collect();
                for (arm, &label) in arms.iter().zip(&labels).take(arms.len() - 1) {
                    let mut hit = self.b.ins().iconst(types::I8, 0);
                    for t in &arm.tags {
                        let one = self.b.ins().icmp_imm_s(IntCC::Equal, v, variant(t));
                        hit = self.b.ins().bor(hit, one);
                    }
                    let next = self.b.create_block();
                    self.b.ins().brif(hit, label, &[], next, &[]);
                    self.b.switch_to_block(next);
                }
                self.b
                    .ins()
                    .jump(*labels.last().expect("a match holds an arm"), &[]);
                let branches: Vec<_> = labels
                    .into_iter()
                    .zip(arms.iter().map(|a| Some(&a.body)))
                    .collect();
                return self.join(e, branches);
            }
            other => panic!("{other:?} is outside the benchmark"),
        })
    }

    /// Runs each branch from its label, then meets in one block that takes the value, if any.
    fn join<'b>(
        &mut self,
        e: &Expr,
        branches: impl IntoIterator<Item = (Label, Option<&'b Block>)>,
    ) -> Option<Value> {
        let ty = e.ty.map(|t| ir_type(self.p, t));
        let merge = self.b.create_block();
        for (label, body) in branches {
            self.b.switch_to_block(label);
            let v = body.and_then(|body| self.block(body));
            let v = ty.map(|t| v.unwrap_or_else(|| self.zero(t)));
            self.b.ins().jump(merge, &args(v.as_slice()));
        }
        self.b.switch_to_block(merge);
        ty.map(|t| self.b.append_block_param(merge, t))
    }

    fn short(&mut self, and: bool, x: &Expr, y: &Expr) -> Value {
        let x = self.value(x);
        let [rhs, merge] = [(); 2].map(|()| self.b.create_block());
        let v = self.b.append_block_param(merge, types::I8);
        let done = args(&[x]);
        match and {
            true => self.b.ins().brif(x, rhs, &[], merge, &done),
            false => self.b.ins().brif(x, merge, &done, rhs, &[]),
        };
        self.b.switch_to_block(rhs);
        let y = self.value(y);
        self.b.ins().jump(merge, &args(&[y]));
        self.b.switch_to_block(merge);
        v
    }

    fn arg(&mut self, a: &paka::core_form::Arg) -> Value {
        match a {
            paka::core_form::Arg::Value(e) => self.value(e),
            paka::core_form::Arg::Inout(_) => panic!("inout is outside the benchmark"),
        }
    }

    fn int(&mut self, a: &Arith, x: Value, y: Value) -> Value {
        let (v, overflow) = match a {
            Arith::Add => self.b.ins().sadd_overflow(x, y),
            Arith::Sub => self.b.ins().ssub_overflow(x, y),
            Arith::Mul => self.b.ins().smul_overflow(x, y),
            Arith::Div | Arith::Rem => {
                let zero = self.b.ins().icmp_imm_s(IntCC::Equal, y, 0);
                self.trap_if(zero, Trap::DivideByZero);
                if matches!(a, Arith::Rem) {
                    return self.b.ins().srem(x, y);
                }
                let min = self.b.ins().icmp_imm_s(IntCC::Equal, x, i64::MIN);
                let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, y, -1);
                let both = self.b.ins().band(min, minus_one);
                self.trap_if(both, Trap::Overflow);
                return self.b.ins().sdiv(x, y);
            }
        };
        self.trap_if(overflow, Trap::Overflow);
        v
    }

    fn float(&mut self, a: &FloatArith, x: Value, y: Value) -> Value {
        let ins = self.b.ins();
        match a {
            FloatArith::Add => ins.fadd(x, y),
            FloatArith::Sub => ins.fsub(x, y),
            FloatArith::Mul => ins.fmul(x, y),
            FloatArith::Div => ins.fdiv(x, y),
        }
    }

    /// A call through the definition's slot; a nonzero status returns at once.
    fn call(&mut self, id: FnId, given: &[Value]) -> Value {
        let ptr = self.isa.pointer_type();
        let sig = match self.sigs[id] {
            Some(sig) => sig,
            None => *self.sigs[id].insert(self.b.import_signature(signature(self.isa, self.p, id))),
        };
        let at = self
            .b
            .ins()
            .load(ptr, MemFlagsData::trusted(), self.slots, id as i32 * 8);
        let out = self.b.ins().stack_addr(ptr, self.scratch, 0);
        let all: Vec<_> = [self.slots, out]
            .into_iter()
            .chain(given.iter().copied())
            .collect();
        let inst = self.b.ins().call_indirect(sig, at, &all);
        let status = self.b.inst_results(inst)[0];
        let pass = *self.pass.get_or_insert_with(|| {
            let label = self.b.create_block();
            self.b.append_block_param(label, types::I32);
            label
        });
        let ok = self.b.create_block();
        self.b.ins().brif(status, pass, &args(&[status]), ok, &[]);
        self.b.switch_to_block(ok);
        let result = self.p.fns[id]
            .result
            .expect("a benchmark definition gives a value");
        self.b
            .ins()
            .stack_load(ptr, ir_type(self.p, result), self.scratch, 0)
    }

    fn builtin(&mut self, row: &str, x: &[Value]) -> Value {
        match row {
            "min(a: Int, b: Int) -> Int" | "max(a: Int, b: Int) -> Int" => {
                let cc = if row.starts_with("min") {
                    IntCC::SignedLessThan
                } else {
                    IntCC::SignedGreaterThan
                };
                let pick = self.b.ins().icmp(cc, x[0], x[1]);
                self.b.ins().select(pick, x[0], x[1])
            }
            "abs(x: Int) -> Int" => {
                let min = self.b.ins().icmp_imm_s(IntCC::Equal, x[0], i64::MIN);
                self.trap_if(min, Trap::Overflow);
                let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, x[0], 0);
                let flipped = self.b.ins().ineg(x[0]);
                self.b.ins().select(negative, flipped, x[0])
            }
            "abs(x: Float) -> Float" => self.b.ins().fabs(x[0]),
            "sqrt(x: Float) -> Float" => self.b.ins().sqrt(x[0]),
            "to_float(n: Int) -> Float" => self.b.ins().fcvt_from_sint(types::F64, x[0]),
            "to_int(x: Float) -> Int" => {
                const TWO_63: f64 = 9223372036854775808.0;
                let (lo, hi) = (
                    self.b.ins().f64const(-TWO_63),
                    self.b.ins().f64const(TWO_63),
                );
                let above = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, x[0], lo);
                let below = self.b.ins().fcmp(FloatCC::LessThan, x[0], hi);
                let inside = self.b.ins().band(above, below);
                let outside = self.b.ins().icmp_imm_s(IntCC::Equal, inside, 0);
                self.trap_if(outside, Trap::BadConversion);
                self.b.ins().fcvt_to_sint(types::I64, x[0])
            }
            "wrap_add(a: Int, b: Int) -> Int" => self.b.ins().iadd(x[0], x[1]),
            "wrap_mul(a: Int, b: Int) -> Int" => self.b.ins().imul(x[0], x[1]),
            "bit_and(a: Int, b: Int) -> Int" => self.b.ins().band(x[0], x[1]),
            _ => self.runtime(row, x),
        }
    }

    /// A stand-in runtime function, reached through the slots after the definitions.
    fn runtime(&mut self, row: &str, x: &[Value]) -> Value {
        let at = RUNTIME.iter().position(|r| *r == row);
        let at = at.unwrap_or_else(|| panic!("{row} is outside the benchmark"));
        let sig = *self.float_sigs[x.len() - 1].get_or_insert_with(|| {
            let mut sig = Signature::new(self.isa.default_call_conv());
            sig.params
                .extend(x.iter().map(|_| AbiParam::new(types::F64)));
            sig.returns.push(AbiParam::new(types::F64));
            self.b.import_signature(sig)
        });
        let slot = (self.p.fns.len() + at) as i32 * 8;
        let addr = self.b.ins().load(
            self.isa.pointer_type(),
            MemFlagsData::trusted(),
            self.slots,
            slot,
        );
        let inst = self.b.ins().call_indirect(sig, addr, x);
        self.b.inst_results(inst)[0]
    }
}

/// A literal leaves Paka here, into the machine code.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}

fn local(place: &Place) -> usize {
    match place {
        Place {
            root: Root::Local(l),
            steps,
        } if steps.is_empty() => *l,
        _ => panic!("only plain locals are in the benchmark"),
    }
}

fn variant(tag: &Tag) -> i64 {
    match tag {
        Tag::User(_, v) => *v as i64,
        other => panic!("{other:?} is outside the benchmark"),
    }
}

fn int_cc(c: &Cmp) -> IntCC {
    match c {
        Cmp::Lt => IntCC::SignedLessThan,
        Cmp::Le => IntCC::SignedLessThanOrEqual,
        Cmp::Gt => IntCC::SignedGreaterThan,
        Cmp::Ge => IntCC::SignedGreaterThanOrEqual,
    }
}

fn float_cc(c: &Cmp) -> FloatCC {
    match c {
        Cmp::Lt => FloatCC::LessThan,
        Cmp::Le => FloatCC::LessThanOrEqual,
        Cmp::Gt => FloatCC::GreaterThan,
        Cmp::Ge => FloatCC::GreaterThanOrEqual,
    }
}
