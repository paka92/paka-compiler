//! The dev back end (M4-26, docs/decisions/M4-05.md): a definition's core form to x86-64 machine
//! code through Cranelift. Every check is an explicit compare and a branch to `rt.trap`, so no
//! emitted instruction faults by itself (M4-04).

use crate::builtins::ROWS;
use crate::core_form::{
    Arg, Arith, Bin, Block, Callee, Cmp, Compound, Expr, ExprKind, FloatArith, Fn, Place, Program,
    Root, Site, Stmt, Ty, Type, Un, Walk,
};
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::Ieee64;
use cranelift_codegen::ir::{
    AbiParam, Block as Label, BlockArg, Function, InstBuilder, MemFlagsData, Signature, StackSlot,
    StackSlotData, StackSlotKind, Type as IrType, UserFuncName, Value, types,
};
use cranelift_codegen::isa::{self, CallConv, OwnedTargetIsa, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use paka_rt::abi::dev::{self as rt, Context, Rt};
use paka_rt::ops::Int;
use std::mem::offset_of;

/// Why a definition was not compiled.
pub type Refusal = String;

/// One target and one set of contexts, reused for every definition.
pub struct Emitter {
    isa: OwnedTargetIsa,
    builder: FunctionBuilderContext,
    context: cranelift_codegen::Context,
}

impl Emitter {
    /// An emitter for this machine, which must be x86-64 with SSE4.1 (`floor`, `ceil`, `trunc`).
    pub fn new() -> Result<Emitter, Refusal> {
        if !has_sse41() {
            return Err("dev mode needs an x86-64 processor with SSE4.1".into());
        }
        let mut target =
            isa::lookup_by_name("x86_64-unknown-linux-gnu").map_err(|e| e.to_string())?;
        for flag in ["has_sse3", "has_ssse3", "has_sse41"] {
            target.enable(flag).map_err(|e| e.to_string())?;
        }
        let flags = settings::Flags::new(settings::builder());
        Ok(Emitter {
            isa: target.finish(flags).map_err(|e| e.to_string())?,
            builder: FunctionBuilderContext::new(),
            context: cranelift_codegen::Context::new(),
        })
    }

    /// Machine code for definition `def`: a function, or example `def - fns.len()`.
    pub fn code(&mut self, p: &Program, def: usize) -> Result<Vec<u8>, Refusal> {
        self.context.func = self.ir(p, def)?;
        let code = match self
            .context
            .compile(&*self.isa, &mut ControlPlane::default())
        {
            Err(e) => Err(format!("{:?}", e.inner)),
            Ok(c) if !c.buffer.relocs().is_empty() => Err("the code needs relocating".into()),
            Ok(c) => Ok(c.code_buffer().to_vec()),
        };
        self.context.clear();
        code
    }

    /// The Cranelift IR of definition `def`.
    pub(crate) fn ir(&mut self, p: &Program, def: usize) -> Result<Function, Refusal> {
        let f = p
            .fns
            .get(def)
            .unwrap_or_else(|| &p.examples[def - p.fns.len()]);
        let name = UserFuncName::user(0, def as u32);
        let mut func = Function::with_name_signature(name, signature(p, f)?);
        let built = Gen::build(&*self.isa, &mut func, &mut self.builder, p, def, f);
        if built.is_err() {
            // A refusal leaves the builder's context half used.
            self.builder = FunctionBuilderContext::new();
        }
        built.map(|()| func)
    }
}

fn has_sse41() -> bool {
    #[cfg(target_arch = "x86_64")]
    return std::arch::is_x86_feature_detected!("sse4.1");
    #[cfg(not(target_arch = "x86_64"))]
    false
}

/// `fn(ctx, args..., out) -> status` (M4-05 section 5); `out` only when there is a result.
fn signature(p: &Program, f: &Fn) -> Result<Signature, Refusal> {
    let mut sig = Signature::new(CallConv::SystemV);
    sig.params.push(AbiParam::new(types::I64));
    for q in &f.params {
        sig.params.push(AbiParam::new(word(ir_type(p, q.ty)?)));
    }
    if f.result.is_some() {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.returns.push(AbiParam::new(types::I32));
    Ok(sig)
}

/// A Bool is I8 inside a function and a whole word across one.
fn word(t: IrType) -> IrType {
    if t == types::I8 { types::I64 } else { t }
}

fn ir_type(p: &Program, t: Ty) -> Result<IrType, Refusal> {
    match p.types[t] {
        Type::Int => Ok(types::I64),
        Type::Float => Ok(types::F64),
        Type::Bool => Ok(types::I8),
        ref other => Err(format!("a value of type {other:?} is not compiled yet")),
    }
}

fn args(values: &[Value]) -> Vec<BlockArg> {
    values.iter().map(|&v| BlockArg::Value(v)).collect()
}

type Emitted = Result<Option<Value>, Refusal>;

struct Gen<'a> {
    b: FunctionBuilder<'a>,
    p: &'a Program,
    def: usize,
    ctx: Value,
    out: Option<Value>,
    vars: Vec<Variable>,
    /// Each enclosing loop's (continue, break) targets.
    loops: Vec<(Label, Label)>,
    /// Returns status 2; filled at the end.
    stop: Option<Label>,
    /// Where a callee writes its result.
    result_slot: Option<StackSlot>,
}

impl<'a> Gen<'a> {
    fn build(
        isa: &dyn TargetIsa,
        func: &'a mut Function,
        cx: &'a mut FunctionBuilderContext,
        p: &'a Program,
        def: usize,
        f: &Fn,
    ) -> Result<(), Refusal> {
        let mut b = FunctionBuilder::new(func, cx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params = b.block_params(entry).to_vec();
        let mut vars = vec![];
        for (i, &t) in f.locals.iter().enumerate() {
            let t = ir_type(p, t)?;
            let var = b.declare_var(t);
            if i < f.params.len() {
                let v = params[i + 1];
                let v = if t == types::I8 {
                    b.ins().ireduce(t, v)
                } else {
                    v
                };
                b.def_var(var, v);
            }
            vars.push(var);
        }
        let mut g = Gen {
            b,
            p,
            def,
            ctx: params[0],
            out: f.result.map(|_| params[params.len() - 1]),
            vars,
            loops: vec![],
            stop: None,
            result_slot: None,
        };
        g.poll();
        g.stack_check();
        let v = g.block(&f.body)?;
        g.ret_value(v);
        if g.stop.is_some() {
            let stop = g.stop();
            g.b.switch_to_block(stop);
            g.ret(2);
        }
        g.b.seal_all_blocks();
        g.b.finalize(isa.frontend_config());
        Ok(())
    }

    fn ret(&mut self, status: i64) {
        let status = self.b.ins().iconst(types::I32, status);
        self.b.ins().return_(&[status]);
    }

    /// Writes the result, if any, and returns status 0.
    fn ret_value(&mut self, v: Option<Value>) {
        if let (Some(out), Some(v)) = (self.out, v) {
            let v = self.widen(v);
            self.b.ins().store(MemFlagsData::trusted(), v, out, 0);
        }
        self.ret(0);
    }

    /// A Bool as the whole word it is across a call.
    fn widen(&mut self, v: Value) -> Value {
        match self.b.func.dfg.value_type(v) {
            types::I8 => self.b.ins().uextend(types::I64, v),
            _ => v,
        }
    }

    /// Continues in a new block that nothing reaches, after a jump or a return.
    fn dead(&mut self) {
        let label = self.b.create_block();
        self.b.switch_to_block(label);
    }

    /// Returns status 2 when the interrupt flag is set (M4-05 section 6).
    fn poll(&mut self) {
        let at = (self.b.ins()).iadd_imm_s(self.ctx, offset_of!(Context, interrupt) as i64);
        let flag = (self.b.ins()).atomic_load(types::I64, MemFlagsData::trusted(), at);
        let stop = self.stop();
        let go = self.b.create_block();
        self.b.ins().brif(flag, stop, &[], go, &[]);
        self.b.switch_to_block(go);
    }

    fn stop(&mut self) -> Label {
        let stop = *self.stop.get_or_insert_with(|| self.b.create_block());
        self.b.set_cold_block(stop);
        stop
    }

    /// Traps with `stack` when the stack pointer is below `ctx.stack_limit` (M4-05 section 6).
    fn stack_check(&mut self) {
        let flags = MemFlagsData::trusted();
        let at = offset_of!(Context, stack_limit) as i32;
        let limit = self.b.ins().load(types::I64, flags, self.ctx, at);
        let sp = self.b.ins().get_stack_pointer(types::I64);
        let low = self.b.ins().icmp(IntCC::UnsignedLessThan, sp, limit);
        self.branch_to_trap(low, rt::STACK, 0, &[]);
    }

    /// Traps with `kind` when `bad` is set.
    fn check(&mut self, bad: Value, kind: u64, site: Site, operands: &[Value]) {
        let at = self.site(site);
        self.branch_to_trap(bad, kind, at, operands);
    }

    fn branch_to_trap(&mut self, bad: Value, kind: u64, at: u64, operands: &[Value]) {
        let [fail, ok] = [(); 2].map(|()| self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.ins().brif(bad, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        self.trap(kind, at, operands);
        self.b.switch_to_block(ok);
    }

    /// `definition << 32 | site`, as the trap record and frames hold it.
    fn site(&self, site: Site) -> u64 {
        (self.def << 32 | site) as u64
    }

    /// Calls `rt.trap(ctx, kind, at, a, b, c)` and returns status 1.
    fn trap(&mut self, kind: u64, at: u64, operands: &[Value]) {
        let mut given = vec![
            self.ctx,
            self.int_const(kind as i64),
            self.int_const(at as i64),
        ];
        given.extend(operands);
        while given.len() < 6 {
            given.push(self.int_const(0));
        }
        self.rt_call(offset_of!(Rt, trap), &given);
        self.ret(1);
    }

    /// Calls the runtime routine at `offset` in `ctx.rt`; every one takes words and returns none.
    fn rt_call(&mut self, offset: usize, given: &[Value]) {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params = vec![AbiParam::new(types::I64); given.len()];
        let sig = self.b.import_signature(sig);
        let flags = MemFlagsData::trusted();
        let table =
            (self.b.ins()).load(types::I64, flags, self.ctx, offset_of!(Context, rt) as i32);
        let f = self.b.ins().load(types::I64, flags, table, offset as i32);
        self.b.ins().call_indirect(sig, f, given);
    }

    /// Calls definition `id` through its slot (M4-05 section 7); a nonzero status returns at
    /// once, a trap after pushing `site` as a frame.
    fn call(&mut self, id: usize, given: &[Arg], site: Site) -> Emitted {
        let f = &self.p.fns[id];
        let mut values = vec![self.ctx];
        for a in given {
            let Arg::Value(e) = a else {
                return Err("an inout argument is not compiled yet".into());
            };
            let v = self.value(e)?;
            values.push(self.widen(v));
        }
        let result = f.result.map(|t| ir_type(self.p, t)).transpose()?;
        let slot = result.map(|_| self.result_slot());
        if let Some(slot) = slot {
            values.push(self.b.ins().stack_addr(types::I64, slot, 0));
        }
        let sig = self.b.import_signature(signature(self.p, f)?);
        let flags = MemFlagsData::trusted();
        let at = offset_of!(Context, slots) as i32;
        let slots = self.b.ins().load(types::I64, flags, self.ctx, at);
        let code = self.b.ins().load(types::I64, flags, slots, (id * 8) as i32);
        let call = self.b.ins().call_indirect(sig, code, &values);
        let status = self.b.inst_results(call)[0];
        self.pass_on(status, site);
        let (Some(t), Some(slot)) = (result, slot) else {
            return Ok(None);
        };
        let v = self.b.ins().stack_load(types::I64, word(t), slot, 0);
        Ok(Some(match t {
            types::I8 => self.b.ins().ireduce(t, v),
            _ => v,
        }))
    }

    /// Returns a callee's nonzero status: 2 as it is, 1 after pushing the call site.
    fn pass_on(&mut self, status: Value, site: Site) {
        let [fail, push, ok] = [(); 3].map(|()| self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.set_cold_block(push);
        self.b.ins().brif(status, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        let stopped = self.b.ins().icmp_imm_s(IntCC::Equal, status, 2);
        let stop = self.stop();
        self.b.ins().brif(stopped, stop, &[], push, &[]);
        self.b.switch_to_block(push);
        let at = self.int_const(self.site(site) as i64);
        self.rt_call(offset_of!(Rt, push_frame), &[self.ctx, at]);
        self.ret(1);
        self.b.switch_to_block(ok);
    }

    fn result_slot(&mut self) -> StackSlot {
        *self.result_slot.get_or_insert_with(|| {
            let data = StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3);
            self.b.create_sized_stack_slot(data)
        })
    }

    fn int_const(&mut self, n: i64) -> Value {
        self.b.ins().iconst(types::I64, n)
    }

    fn float_const(&mut self, bits: u64) -> Value {
        self.b.ins().f64const(Ieee64::with_bits(bits))
    }

    fn zero(&mut self, t: IrType) -> Value {
        match t {
            types::F64 => self.float_const(0),
            _ => self.b.ins().iconst(t, 0),
        }
    }

    fn block(&mut self, block: &Block) -> Emitted {
        for s in &block.stmts {
            self.stmt(s)?;
        }
        match &block.value {
            Some(e) => self.expr(e),
            None => Ok(None),
        }
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), Refusal> {
        match s {
            Stmt::Let(l, e) => {
                let v = self.value(e)?;
                self.b.def_var(self.vars[*l], v);
            }
            Stmt::Assign(place, op, e) => {
                let var = self.vars[local(place)?];
                let mut v = self.value(e)?;
                if let Some(op) = op {
                    let old = self.b.use_var(var);
                    v = match op {
                        Compound::Int(a, site) => self.int(a, *site, old, v),
                        Compound::Float(a) => self.float(a, old, v),
                        Compound::Join(_) => return Err("text is not compiled yet".into()),
                    };
                }
                self.b.def_var(var, v);
            }
            Stmt::Return(e) => {
                let v = e.as_ref().map(|e| self.value(e)).transpose()?;
                self.ret_value(v);
                self.dead();
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = *self.loops.last().expect("a loop holds it");
                let to = if matches!(s, Stmt::Break) { exit } else { next };
                self.b.ins().jump(to, &[]);
                self.dead();
            }
            Stmt::While(cond, body) => {
                let [head, inner, exit] = [(); 3].map(|()| self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                self.poll();
                let c = self.value(cond)?;
                self.b.ins().brif(c, inner, &[], exit, &[]);
                self.b.switch_to_block(inner);
                self.body(head, exit, body)?;
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(exit);
            }
            Stmt::For(l, Walk::Range(from, to), body) => {
                let (from, to) = (self.value(from)?, self.value(to)?);
                let i = self.b.declare_var(types::I64);
                self.b.def_var(i, from);
                let [head, inner, next, exit] = [(); 4].map(|()| self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                self.poll();
                let at = self.b.use_var(i);
                let more = self.b.ins().icmp(IntCC::SignedLessThan, at, to);
                self.b.ins().brif(more, inner, &[], exit, &[]);
                self.b.switch_to_block(inner);
                self.b.def_var(self.vars[*l], at);
                self.body(next, exit, body)?;
                self.b.ins().jump(next, &[]);
                self.b.switch_to_block(next);
                // `at < to`, so this never passes the Int range.
                let at = self.b.use_var(i);
                let step = self.b.ins().iadd_imm_s(at, 1);
                self.b.def_var(i, step);
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(exit);
            }
            Stmt::For(_, Walk::List(_), _) => return Err("a List is not compiled yet".into()),
            Stmt::Expr(e) => {
                self.expr(e)?;
            }
        }
        Ok(())
    }

    fn body(&mut self, next: Label, exit: Label, body: &Block) -> Result<(), Refusal> {
        self.loops.push((next, exit));
        self.block(body)?;
        self.loops.pop();
        Ok(())
    }

    fn value(&mut self, e: &Expr) -> Result<Value, Refusal> {
        Ok(self.expr(e)?.expect("the node gives a value"))
    }

    fn is_float(&self, e: &Expr) -> bool {
        e.ty.is_some_and(|t| self.p.types[t] == Type::Float)
    }

    fn expr(&mut self, e: &Expr) -> Emitted {
        let v = match &e.kind {
            ExprKind::Int(n) => self.int_const(raw(*n)),
            ExprKind::Float(x) => self.float_const(x.bits()),
            ExprKind::Bool(v) => self.b.ins().iconst(types::I8, i64::from(*v)),
            ExprKind::Place(place) => self.b.use_var(self.vars[local(place)?]),
            ExprKind::Hole(site) => {
                self.trap(rt::HOLE, self.site(*site), &[]);
                return self.unreached(e);
            }
            ExprKind::Unary(op, x) => {
                let x = self.value(x)?;
                match op {
                    Un::Neg(site) => {
                        let zero = self.int_const(0);
                        let (v, overflow) = self.b.ins().ssub_overflow(zero, x);
                        self.check(overflow, rt::OVERFLOW, *site, &[x]);
                        v
                    }
                    Un::NegFloat => self.b.ins().fneg(x),
                    Un::Not => self.b.ins().icmp_imm_s(IntCC::Equal, x, 0),
                }
            }
            ExprKind::Binary(op @ (Bin::And | Bin::Or), x, y) => {
                self.short(matches!(op, Bin::And), x, y)?
            }
            ExprKind::Binary(op, x, y) => {
                let float = self.is_float(x);
                let (x, y) = (self.value(x)?, self.value(y)?);
                match op {
                    Bin::Int(a, site) => self.int(a, *site, x, y),
                    Bin::Float(a) => self.float(a, x, y),
                    Bin::Join(_) => return Err("text is not compiled yet".into()),
                    _ => self.compare(op, float, x, y),
                }
            }
            ExprKind::Call(Callee::Builtin(row), given, site) => {
                let mut x = vec![];
                for a in given {
                    match a {
                        Arg::Value(e) => x.push(self.value(e)?),
                        Arg::Inout(_) => return Err("an inout argument is not compiled yet".into()),
                    }
                }
                return self.builtin(ROWS[*row].1, &x, *site, e);
            }
            ExprKind::Call(Callee::Fn(id), given, site) => return self.call(*id, given, *site),
            ExprKind::If(c, then, otherwise) => {
                let c = self.value(c)?;
                let [yes, no] = [(); 2].map(|()| self.b.create_block());
                self.b.ins().brif(c, yes, &[], no, &[]);
                return self.join(e, [(yes, Some(then)), (no, otherwise.as_ref())]);
            }
            kind => return Err(format!("{} is not compiled yet", what(kind))),
        };
        Ok(Some(v))
    }

    /// After a trap: dead code, and a stand-in value of the node's type.
    fn unreached(&mut self, e: &Expr) -> Emitted {
        self.dead();
        let t = e.ty.map(|t| ir_type(self.p, t)).transpose()?;
        Ok(t.map(|t| self.zero(t)))
    }

    /// Runs each branch from its label, then meets in one block that takes the value, if any.
    fn join<'b>(
        &mut self,
        e: &Expr,
        branches: impl IntoIterator<Item = (Label, Option<&'b Block>)>,
    ) -> Emitted {
        let t = e.ty.map(|t| ir_type(self.p, t)).transpose()?;
        let merge = self.b.create_block();
        for (label, body) in branches {
            self.b.switch_to_block(label);
            let v = match body {
                Some(body) => self.block(body)?,
                None => None,
            };
            let v = t.map(|t| v.unwrap_or_else(|| self.zero(t)));
            self.b.ins().jump(merge, &args(v.as_slice()));
        }
        self.b.switch_to_block(merge);
        Ok(t.map(|t| self.b.append_block_param(merge, t)))
    }

    fn short(&mut self, and: bool, x: &Expr, y: &Expr) -> Result<Value, Refusal> {
        let x = self.value(x)?;
        let [rhs, merge] = [(); 2].map(|()| self.b.create_block());
        let v = self.b.append_block_param(merge, types::I8);
        let done = args(&[x]);
        match and {
            true => self.b.ins().brif(x, rhs, &[], merge, &done),
            false => self.b.ins().brif(x, merge, &done, rhs, &[]),
        };
        self.b.switch_to_block(rhs);
        let y = self.value(y)?;
        self.b.ins().jump(merge, &args(&[y]));
        self.b.switch_to_block(merge);
        Ok(v)
    }

    fn compare(&mut self, op: &Bin, float: bool, x: Value, y: Value) -> Value {
        let (int_cc, float_cc) = match op {
            Bin::Eq(_) => (IntCC::Equal, FloatCC::Equal),
            Bin::Ne(_) => (IntCC::NotEqual, FloatCC::NotEqual),
            Bin::Order(Cmp::Lt) => (IntCC::SignedLessThan, FloatCC::LessThan),
            Bin::Order(Cmp::Le) => (IntCC::SignedLessThanOrEqual, FloatCC::LessThanOrEqual),
            Bin::Order(Cmp::Gt) => (IntCC::SignedGreaterThan, FloatCC::GreaterThan),
            Bin::Order(Cmp::Ge) => (IntCC::SignedGreaterThanOrEqual, FloatCC::GreaterThanOrEqual),
            _ => unreachable!("an arithmetic operator is handled before"),
        };
        match float {
            true => self.b.ins().fcmp(float_cc, x, y),
            false => self.b.ins().icmp(int_cc, x, y),
        }
    }

    fn int(&mut self, a: &Arith, site: Site, x: Value, y: Value) -> Value {
        let (v, overflow) = match a {
            Arith::Add => self.b.ins().sadd_overflow(x, y),
            Arith::Sub => self.b.ins().ssub_overflow(x, y),
            Arith::Mul => self.b.ins().smul_overflow(x, y),
            Arith::Div | Arith::Rem => return self.divide(matches!(a, Arith::Div), site, x, y),
        };
        self.check(overflow, rt::OVERFLOW, site, &[x, y]);
        v
    }

    /// `/` or `%`. The divisors that fault, 0 and -1 under the smallest Int, trap before the
    /// divide, and a select swaps them for 1, so the instruction cannot fault even if reached.
    fn divide(&mut self, div: bool, site: Site, x: Value, y: Value) -> Value {
        let zero = self.b.ins().icmp_imm_s(IntCC::Equal, y, 0);
        self.check(zero, rt::DIVIDE_BY_ZERO, site, &[x, y]);
        let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, y, -1);
        let risky = match div {
            true => {
                let min = self.b.ins().icmp_imm_s(IntCC::Equal, x, i64::MIN);
                let both = self.b.ins().band(min, minus_one);
                self.check(both, rt::OVERFLOW, site, &[x, y]);
                both
            }
            // `x % -1` is 0, as is `x % 1`.
            false => minus_one,
        };
        let risky = self.b.ins().bor(zero, risky);
        let one = self.int_const(1);
        let d = self.b.ins().select(risky, one, y);
        match div {
            true => self.b.ins().sdiv(x, d),
            false => self.b.ins().srem(x, d),
        }
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

    fn builtin(&mut self, row: &str, x: &[Value], site: Site, e: &Expr) -> Emitted {
        let v = match row {
            "min(a: Int, b: Int) -> Int" | "max(a: Int, b: Int) -> Int" => {
                let cc = match row.starts_with("min") {
                    true => IntCC::SignedLessThan,
                    false => IntCC::SignedGreaterThan,
                };
                let pick = self.b.ins().icmp(cc, x[0], x[1]);
                self.b.ins().select(pick, x[0], x[1])
            }
            // NaN wins, and -0.0 is below 0.0 (M2-03), as in WebAssembly.
            "min(a: Float, b: Float) -> Float" => self.b.ins().fmin(x[0], x[1]),
            "max(a: Float, b: Float) -> Float" => self.b.ins().fmax(x[0], x[1]),
            "abs(x: Int) -> Int" => {
                let min = self.b.ins().icmp_imm_s(IntCC::Equal, x[0], i64::MIN);
                self.check(min, rt::OVERFLOW, site, &[x[0]]);
                let negative = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, x[0], 0);
                let flipped = self.b.ins().ineg(x[0]);
                self.b.ins().select(negative, flipped, x[0])
            }
            "abs(x: Float) -> Float" => self.b.ins().fabs(x[0]),
            "to_float(n: Int) -> Float" => self.b.ins().fcvt_from_sint(types::F64, x[0]),
            "to_int(x: Float) -> Int" => self.convert(x[0], site),
            "sqrt(x: Float) -> Float" => self.b.ins().sqrt(x[0]),
            "floor(x: Float) -> Float" => self.b.ins().floor(x[0]),
            "ceil(x: Float) -> Float" => self.b.ins().ceil(x[0]),
            "trunc(x: Float) -> Float" => self.b.ins().trunc(x[0]),
            "round(x: Float) -> Float" => self.round(x[0]),
            "wrap_add(a: Int, b: Int) -> Int" => self.b.ins().iadd(x[0], x[1]),
            "wrap_sub(a: Int, b: Int) -> Int" => self.b.ins().isub(x[0], x[1]),
            "wrap_mul(a: Int, b: Int) -> Int" => self.b.ins().imul(x[0], x[1]),
            "bit_and(a: Int, b: Int) -> Int" => self.b.ins().band(x[0], x[1]),
            "bit_or(a: Int, b: Int) -> Int" => self.b.ins().bor(x[0], x[1]),
            "bit_xor(a: Int, b: Int) -> Int" => self.b.ins().bxor(x[0], x[1]),
            "bit_not(a: Int) -> Int" => self.b.ins().bnot(x[0]),
            "shift_left(a: Int, count: Int) -> Int" | "shift_right(a: Int, count: Int) -> Int" => {
                let outside = (self.b.ins()).icmp_imm_s(IntCC::UnsignedGreaterThan, x[1], 63);
                self.check(outside, rt::OUT_OF_BOUNDS, site, &[x[1]]);
                match row.starts_with("shift_left") {
                    true => self.b.ins().ishl(x[0], x[1]),
                    false => self.b.ins().sshr(x[0], x[1]),
                }
            }
            "assert(cond: Bool)" => {
                let failed = self.b.ins().icmp_imm_s(IntCC::Equal, x[0], 0);
                self.check(failed, rt::ASSERT, site, &[]);
                return Ok(None);
            }
            "unreachable() -> T" => {
                self.trap(rt::ASSERT, self.site(site), &[]);
                return self.unreached(e);
            }
            _ => return Err(format!("`{row}` is not compiled yet")),
        };
        Ok(Some(v))
    }

    /// `trunc` as an Int after a range check, so the conversion never meets NaN or a value
    /// outside the Int range.
    fn convert(&mut self, x: Value, site: Site) -> Value {
        let low = self.float_const(MINUS_TWO_63);
        let high = self.float_const(TWO_63);
        let above = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, x, low);
        let below = self.b.ins().fcmp(FloatCC::LessThan, x, high);
        let inside = self.b.ins().band(above, below);
        let outside = self.b.ins().icmp_imm_s(IntCC::Equal, inside, 0);
        let bits = self.b.ins().bitcast(types::I64, MemFlagsData::new(), x);
        self.check(outside, rt::BAD_CONVERSION, site, &[bits]);
        self.b.ins().fcvt_to_sint_sat(types::I64, x)
    }

    /// Half away from zero, as `paka_rt::ops::Float::round`: a Float of 2^52 or more is whole.
    fn round(&mut self, x: Value) -> Value {
        let size = self.b.ins().fabs(x);
        let limit = self.float_const(TWO_52);
        let whole = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, size, limit);
        let t = self.b.ins().trunc(x);
        let fraction = self.b.ins().fsub(x, t);
        let fraction = self.b.ins().fabs(fraction);
        let half = self.float_const(HALF);
        let up = self
            .b
            .ins()
            .fcmp(FloatCC::GreaterThanOrEqual, fraction, half);
        let one = self.float_const(ONE);
        let one = self.b.ins().fcopysign(one, x);
        let away = self.b.ins().fadd(t, one);
        let rounded = self.b.ins().select(up, away, t);
        self.b.ins().select(whole, x, rounded)
    }
}

/// Float bits: -2^63, 2^63, 2^52, 0.5 and 1.
const MINUS_TWO_63: u64 = 0xC3E0_0000_0000_0000;
const TWO_63: u64 = 0x43E0_0000_0000_0000;
const TWO_52: u64 = 0x4330_0000_0000_0000;
const HALF: u64 = 0x3FE0_0000_0000_0000;
const ONE: u64 = 0x3FF0_0000_0000_0000;

/// A literal leaves Paka here, into the machine code.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}

fn local(place: &Place) -> Result<usize, Refusal> {
    match place {
        Place {
            root: Root::Local(l),
            steps,
        } if steps.is_empty() => Ok(*l),
        _ => Err("a place with steps is not compiled yet".into()),
    }
}

fn what(kind: &ExprKind) -> &'static str {
    match kind {
        ExprKind::Str(_) | ExprKind::Interp(..) => "text",
        ExprKind::Record(..) => "a record",
        ExprKind::List(_) => "a List",
        ExprKind::Map(_) => "a Map",
        ExprKind::Variant(..) => "a variant",
        ExprKind::FnValue(_) => "a function value",
        ExprKind::Call(..) => "a call",
        ExprKind::Match(..) => "match",
        _ => "this expression",
    }
}

#[cfg(test)]
mod tests;
