//! The dev back end (M4-26, docs/decisions/M4-05.md): a definition's core form to x86-64 machine
//! code through Cranelift. Every check is an explicit compare and a branch to `rt.trap`, so no
//! emitted instruction faults by itself (M4-04).

use crate::builtins::ROWS;
use crate::core_form::{
    Arg, Arith, Bin, Block, Callee, Cmp, Compound, EnumId, Expr, ExprKind, Field, FloatArith, Fn,
    FnRef, Local, Param, Place, Program, Root, Site, Stmt, Tag, Ty, Type, Un, Walk,
};
use crate::layout::Layouts;
use crate::owners::Owners;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::Ieee64;
use cranelift_codegen::ir::{
    AbiParam, Block as Label, BlockArg, Function, Inst, InstBuilder, MemFlagsData, SigRef,
    Signature, StackSlot, StackSlotData, StackSlotKind, Type as IrType, UserFuncName, Value, types,
};
use cranelift_codegen::isa::{self, CallConv, OwnedTargetIsa, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use paka_rt::abi::dev::words::*;
use paka_rt::abi::dev::{self as rt, Context, Rt};
use paka_rt::ops::Int;
use std::collections::{BTreeMap, BTreeSet};
use std::mem::offset_of;

/// Why a definition was not compiled.
pub type Refusal = String;

/// The most stack slot bytes a function may take, so the runner's margin always holds (M4-05
/// section 6).
const FRAME_LIMIT: u32 = 256 << 10;

/// One target and one set of contexts, reused for every definition.
pub struct Emitter {
    isa: OwnedTargetIsa,
    builder: FunctionBuilderContext,
    context: cranelift_codegen::Context,
    /// A planted fault, 1 to 5, for the fuzz loop's self-test (M4-41); 0 in real use.
    pub plant: u8,
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
            plant: 0,
        })
    }

    /// Machine code for definition `def`: a function, or example `def - fns.len()`.
    pub fn code(&mut self, p: &Program, l: &Layouts, def: usize) -> Result<Vec<u8>, Refusal> {
        let ir = self.ir(p, l, def)?;
        self.compile(ir)
    }

    /// Machine code for a definition's IR.
    fn compile(&mut self, ir: Function) -> Result<Vec<u8>, Refusal> {
        self.context.func = ir;
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
    pub(crate) fn ir(&mut self, p: &Program, l: &Layouts, def: usize) -> Result<Function, Refusal> {
        let made;
        let f = match def.checked_sub(p.fns.len() + p.examples.len()) {
            None => p.def(def),
            Some(k) => {
                let c = constructors(p);
                made = match k.checked_sub(c.len()) {
                    None => constructor(p, c[k]),
                    Some(i) => equalities(p)
                        .get(i)
                        .map_or_else(|| state(p), |&t| equality(p, t)),
                };
                &made
            }
        };
        let name = UserFuncName::user(0, def as u32);
        let mut func = Function::with_name_signature(name, signature(p, f));
        let cx = &mut self.builder;
        let built = Gen::build(&*self.isa, &mut func, cx, (p, l), (def, f), self.plant);
        if built.is_err() {
            // A refusal leaves the builder's context half used.
            self.builder = FunctionBuilderContext::new();
        }
        built?;
        let frame: u32 = func.sized_stack_slots.values().map(|s| s.size).sum();
        if frame > FRAME_LIMIT {
            return Err(format!(
                "internal: a frame of {frame} bytes passes {FRAME_LIMIT}"
            ));
        }
        Ok(func)
    }
}

/// The definitions after the examples (M4-05 section 7): each variant with fields of an enum the
/// program uses, called through its slot when the variant is used as a function.
pub fn constructors(p: &Program) -> Vec<(EnumId, usize)> {
    let mut all = vec![];
    for (e, d) in p.enums.iter().enumerate() {
        if p.types.contains(&Type::Enum(e)) {
            let v = d.variants.iter().enumerate();
            all.extend(v.filter(|(_, v)| !v.fields.is_empty()).map(|(v, _)| (e, v)));
        }
    }
    all
}

/// The record and enum types that hold themselves, through any type: each has a definition
/// that compares two of its values, which inline equality calls where it meets the type again.
pub fn equalities(p: &Program) -> Vec<Ty> {
    if !p.types.contains(&Type::Bool) {
        return vec![];
    }
    let nominal = |t: &Ty| matches!(p.types[*t], Type::Record(_) | Type::Enum(_));
    (0..p.types.len())
        .filter(|t| nominal(t) && holds(p, *t, *t, &mut BTreeSet::new()))
        .collect()
}

/// Whether a value of `ty` holds a `target` anywhere inside it.
fn holds(p: &Program, ty: Ty, target: Ty, seen: &mut BTreeSet<Ty>) -> bool {
    let inner: Vec<Ty> = match &p.types[ty] {
        Type::List(t) | Type::Option(t) | Type::Effect(t) => vec![*t],
        Type::Map(k, v) | Type::Result(k, v) => vec![*k, *v],
        Type::Record(i) => p.records[*i].fields.iter().map(|f| f.ty).collect(),
        Type::Enum(i) => (p.enums[*i].variants.iter())
            .flat_map(|v| v.fields.iter().map(|f| f.ty))
            .collect(),
        _ => vec![],
    };
    (inner.into_iter()).any(|t| t == target || (seen.insert(t) && holds(p, t, target, seen)))
}

/// The slot id of the definition that compares two values of type `t`.
fn equality_slot(p: &Program, t: Ty) -> Option<usize> {
    let at = equalities(p).iter().position(|&e| e == t)?;
    Some(p.fns.len() + p.examples.len() + constructors(p).len() + at)
}

/// The slot ids before the type records: functions, examples, constructors, equalities, then
/// the app's state.
pub fn definitions(p: &Program) -> usize {
    let state = p.app.as_ref().is_some_and(|a| a.state.is_some());
    let made = constructors(p).len() + equalities(p).len();
    p.fns.len() + p.examples.len() + made + usize::from(state)
}

/// The slot id of the definition that builds the app's state from its defaults, if it has one.
pub fn state_slot(p: &Program) -> Option<usize> {
    p.app.as_ref()?.state.as_ref()?;
    Some(definitions(p) - 1)
}

/// The app's state as a function of nothing.
fn state(p: &Program) -> Fn {
    let value = p.app.as_ref().and_then(|a| a.state.clone());
    let value = value.expect("an app with a state");
    Fn {
        name: "state".into(),
        site: 0,
        params: vec![],
        locals: vec![],
        result: value.ty,
        body: Block {
            stmts: vec![],
            value: Some(Box::new(value)),
        },
        claims: vec![],
    }
}

/// `fn equal(a: T, b: T) -> Bool { a == b }` for type `t`.
fn equality(p: &Program, t: Ty) -> Fn {
    let local = |l| {
        Box::new(Expr {
            ty: Some(t),
            kind: ExprKind::Place(Place {
                root: Root::Local(l),
                steps: vec![],
            }),
        })
    };
    let bool = p.types.iter().position(|t| *t == Type::Bool);
    let value = Expr {
        ty: bool,
        kind: ExprKind::Binary(Bin::Eq(0), local(0), local(1)),
    };
    let param = Param {
        ty: t,
        inout: false,
    };
    Fn {
        name: "equal".into(),
        site: 0,
        params: vec![param.clone(), param],
        locals: vec![t, t],
        result: bool,
        body: Block {
            stmts: vec![],
            value: Some(Box::new(value)),
        },
        claims: vec![],
    }
}

/// Variant `v` of enum `e` as a function of its fields.
fn constructor(p: &Program, (e, v): (EnumId, usize)) -> Fn {
    let variant = &p.enums[e].variants[v];
    let t = p.types.iter().position(|t| *t == Type::Enum(e));
    let t = t.expect("a constructor's enum is used");
    let field = |(l, f): (usize, &Field)| Expr {
        ty: Some(f.ty),
        kind: ExprKind::Place(Place {
            root: Root::Local(l),
            steps: vec![],
        }),
    };
    let fields = variant.fields.iter().enumerate().map(field).collect();
    let value = Expr {
        ty: Some(t),
        kind: ExprKind::Variant(Tag::User(e, v), fields),
    };
    let param = |f: &Field| Param {
        ty: f.ty,
        inout: false,
    };
    Fn {
        name: variant.name.clone(),
        site: 0,
        params: variant.fields.iter().map(param).collect(),
        locals: variant.fields.iter().map(|f| f.ty).collect(),
        result: Some(t),
        body: Block {
            stmts: vec![],
            value: Some(Box::new(value)),
        },
        claims: vec![],
    }
}

fn has_sse41() -> bool {
    #[cfg(target_arch = "x86_64")]
    return std::arch::is_x86_feature_detected!("sse4.1");
    #[cfg(not(target_arch = "x86_64"))]
    false
}

/// `fn(ctx, args..., out) -> status` (M4-05 section 5); `out` only when there is a result.
fn signature(p: &Program, f: &Fn) -> Signature {
    callee_signature(p, &f.params, f.result)
}

fn callee_signature(p: &Program, params: &[Param], result: Option<Ty>) -> Signature {
    let mut sig = Signature::new(CallConv::SystemV);
    sig.params.push(AbiParam::new(types::I64));
    for q in params {
        let t = word(ir_type(p, q.ty));
        sig.params
            .push(AbiParam::new(if q.inout { types::I64 } else { t }));
    }
    if result.is_some() {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.returns.push(AbiParam::new(types::I32));
    sig
}

/// A Bool is I8 inside a function and a whole word across one.
fn word(t: IrType) -> IrType {
    if t == types::I8 { types::I64 } else { t }
}

/// A value held in memory is the I64 address of its words (`values`).
fn ir_type(p: &Program, t: Ty) -> IrType {
    match p.types[t] {
        Type::Float => types::F64,
        Type::Bool => types::I8,
        _ => types::I64,
    }
}

fn args(values: &[Value]) -> Vec<BlockArg> {
    values.iter().map(|&v| BlockArg::Value(v)).collect()
}

type Emitted = Result<Option<Value>, Refusal>;

struct Gen<'a> {
    b: FunctionBuilder<'a>,
    p: &'a Program,
    l: &'a Layouts,
    /// The type of each local.
    locals: &'a [Ty],
    params: &'a [Param],
    def: usize,
    /// The definition's first site.
    first: Site,
    ctx: Value,
    out: Option<Value>,
    vars: Vec<Variable>,
    /// Each enclosing loop's (continue, break) targets.
    loops: Vec<(Label, Label)>,
    /// Returns status 2; filled at the end.
    stop: Label,
    /// Where a callee writes its result.
    result_slot: Option<StackSlot>,
    /// Imported signatures: runtime routines by word count and status, definitions by parameters
    /// and result.
    rt_sigs: BTreeMap<(usize, bool), SigRef>,
    sigs: BTreeMap<(Vec<Param>, Option<Ty>), SigRef>,
    owners: Owners<Value>,
    plant: u8,
}

impl<'a> Gen<'a> {
    fn build(
        isa: &dyn TargetIsa,
        func: &'a mut Function,
        cx: &'a mut FunctionBuilderContext,
        (p, l): (&'a Program, &'a Layouts),
        (def, f): (usize, &'a Fn),
        plant: u8,
    ) -> Result<(), Refusal> {
        let mut b = FunctionBuilder::new(func, cx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params = b.block_params(entry).to_vec();
        let mut vars = vec![];
        for (i, &ty) in f.locals.iter().enumerate() {
            let t = match f.params.get(i) {
                Some(q) if q.inout => types::I64,
                _ => ir_type(p, ty),
            };
            let var = b.declare_var(t);
            if i >= f.params.len() && values::held(&p.types[ty]) {
                let at = values::slot(&mut b, l.get(ty).size);
                b.def_var(var, at);
            } else if i < f.params.len() {
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
        let stop = b.create_block();
        b.set_cold_block(stop);
        let mut g = Gen {
            b,
            p,
            l,
            locals: &f.locals,
            params: &f.params,
            def,
            first: f.site,
            ctx: params[0],
            out: f.result.map(|_| params[params.len() - 1]),
            vars,
            loops: vec![],
            stop,
            result_slot: None,
            rt_sigs: BTreeMap::new(),
            sigs: BTreeMap::new(),
            owners: Owners::default(),
            plant,
        };
        g.poll();
        g.stack_check();
        g.owners.enter(false);
        for s in &f.body.stmts {
            g.statement(s)?;
        }
        let e = f.body.value.as_deref();
        let v = e.map(|e| g.expr(e)).transpose()?.flatten();
        g.give(e, v);
        g.b.switch_to_block(stop);
        g.ret(2);
        g.b.seal_all_blocks();
        g.b.finalize(isa.frontend_config());
        Ok(())
    }

    fn ret(&mut self, status: i64) {
        let status = self.b.ins().iconst(types::I32, status);
        self.b.ins().return_(&[status]);
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
        let go = self.b.create_block();
        self.b.ins().brif(flag, self.stop, &[], go, &[]);
        self.b.switch_to_block(go);
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

    /// `definition << 32 | offset`, the offset counted from the definition's first site, so an
    /// edit elsewhere leaves the code alone; the trap record and frames hold it.
    fn site(&self, site: Site) -> u64 {
        (self.def << 32 | (site - self.first)) as u64
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

    /// Calls the runtime routine at `offset` in `ctx.rt`, which takes words and returns none.
    fn rt_call(&mut self, offset: usize, given: &[Value]) {
        self.rt_invoke(offset, given, false);
    }

    /// Calls a runtime routine that returns a status: nonzero returns 1, the trap recorded.
    fn rt_try(&mut self, offset: usize, given: &[Value]) {
        let call = self.rt_invoke(offset, given, true);
        let status = self.b.inst_results(call)[0];
        let [fail, ok] = [(); 2].map(|()| self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.ins().brif(status, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        self.ret(1);
        self.b.switch_to_block(ok);
    }

    fn rt_invoke(&mut self, offset: usize, given: &[Value], status: bool) -> Inst {
        let key = (given.len(), status);
        let sig = *self.rt_sigs.entry(key).or_insert_with(|| {
            let mut sig = Signature::new(CallConv::SystemV);
            sig.params = vec![AbiParam::new(types::I64); given.len()];
            if status {
                sig.returns.push(AbiParam::new(types::I32));
            }
            self.b.import_signature(sig)
        });
        let flags = MemFlagsData::trusted();
        let table =
            (self.b.ins()).load(types::I64, flags, self.ctx, offset_of!(Context, rt) as i32);
        let f = self.b.ins().load(types::I64, flags, table, offset as i32);
        self.b.ins().call_indirect(sig, f, given)
    }

    /// Calls a definition through its slot (M4-05 section 5): a function value first, then each
    /// argument in written order, an `inout` place's operands in its position, then each place
    /// walked to its address. A nonzero status returns at once, a trap after pushing `site` as a
    /// frame.
    fn call(&mut self, callee: &Callee, given: &[Arg], site: Site) -> Emitted {
        let (id, params, result) = match callee {
            Callee::Fn(id) => {
                let f = &self.p.fns[*id];
                (self.int_const(*id as i64), f.params.clone(), f.result)
            }
            Callee::Value(e) => {
                let Type::Fn(params, result) = &self.p.types[ty(e)] else {
                    unreachable!("a called value is a function");
                };
                let param = |&ty: &Ty| Param { ty, inout: false };
                (self.value(e)?, params.iter().map(param).collect(), *result)
            }
            Callee::Builtin(_) => unreachable!("a built-in is called in place"),
        };
        let mut values = vec![self.ctx];
        let mut places = vec![];
        for a in given {
            match a {
                Arg::Value(e) => {
                    let v = self.borrowed(e)?;
                    values.push(self.widen(v));
                }
                Arg::Inout(place) => {
                    places.push((values.len(), place, self.inout_operands(place)?));
                    values.push(self.ctx);
                }
            }
        }
        let mut spilled = vec![];
        for (i, place, operands) in places {
            values[i] = self.address(place, operands, &mut spilled).0;
        }
        let slot = self.invoke(id, params, result, values, site);
        self.reload(spilled);
        Ok(match (result, slot) {
            (Some(t), Some(at)) => {
                let v = self.get(t, at, 0);
                Some(self.temp(t, v))
            }
            _ => None,
        })
    }

    /// Calls slot `id` on `values` and the result's slot, which it returns.
    pub(super) fn invoke(
        &mut self,
        id: Value,
        params: Vec<Param>,
        result: Option<Ty>,
        mut values: Vec<Value>,
        site: Site,
    ) -> Option<Value> {
        let ir_result = result.map(|t| ir_type(self.p, t));
        let held = result.filter(|&t| values::held(&self.p.types[t]));
        let slot = match held {
            Some(t) => Some(values::slot(&mut self.b, self.l.get(t).size)),
            None => ir_result.map(|_| {
                let slot = self.result_slot();
                self.b.ins().stack_addr(types::I64, slot, 0)
            }),
        };
        values.extend(slot);
        let key = (params.clone(), result);
        let sig = match self.sigs.get(&key) {
            Some(&sig) => sig,
            None => {
                let sig = callee_signature(self.p, &params, result);
                let sig = self.b.import_signature(sig);
                self.sigs.insert(key, sig);
                sig
            }
        };
        let flags = MemFlagsData::trusted();
        let at = offset_of!(Context, slots) as i32;
        let slots = self.b.ins().load(types::I64, flags, self.ctx, at);
        let word = self.b.ins().imul_imm_s(id, 8);
        let at = self.b.ins().iadd(slots, word);
        let code = self.b.ins().load(types::I64, flags, at, 0);
        let call = self.b.ins().call_indirect(sig, code, &values);
        let status = self.b.inst_results(call)[0];
        self.pass_on(status, site);
        slot
    }

    /// Whether `x` and `y` of type `t` are equal, by the definition that compares them.
    pub(super) fn equal_call(&mut self, t: Ty, x: Value, y: Value) -> Value {
        let slot = equality_slot(self.p, t).expect("a type that holds itself");
        let id = self.int_const(slot as i64);
        let param = Param {
            ty: t,
            inout: false,
        };
        let bool = self.p.types.iter().position(|t| *t == Type::Bool);
        let values = vec![self.ctx, x, y];
        let at = self.invoke(id, vec![param.clone(), param], bool, values, self.first);
        let word = (self.b.ins()).load(types::I64, MemFlagsData::trusted(), at.unwrap(), 0);
        self.b.ins().ireduce(types::I8, word)
    }

    /// An argument a callee only reads: a place is lent as it is, a List slice as a view.
    fn borrowed(&mut self, e: &Expr) -> Result<Value, Refusal> {
        match &e.kind {
            ExprKind::Place(place) => Ok(self.borrow(place)?.0),
            _ => self.value(e),
        }
    }

    /// Returns a callee's nonzero status: 2 as it is, 1 after pushing the call site.
    fn pass_on(&mut self, status: Value, site: Site) {
        let [fail, push, ok] = [(); 3].map(|()| self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.set_cold_block(push);
        self.b.ins().brif(status, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        let stopped = self.b.ins().icmp_imm_s(IntCC::Equal, status, 2);
        self.b.ins().brif(stopped, self.stop, &[], push, &[]);
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

    fn stmt(&mut self, s: &Stmt) -> Result<(), Refusal> {
        match s {
            Stmt::Let(l, e) => {
                let v = self.value(e)?;
                self.bind_owned(*l, v);
            }
            Stmt::Assign(place, op, e) => self.assign(place, op.as_ref(), e)?,
            Stmt::Return(e) => {
                let v = e.as_ref().map(|e| self.value(e)).transpose()?;
                self.give(e.as_ref(), v);
                self.dead();
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = *self.loops.last().expect("a loop holds it");
                let to = if matches!(s, Stmt::Break) { exit } else { next };
                self.leave_loop();
                self.b.ins().jump(to, &[]);
                self.dead();
            }
            Stmt::While(cond, body) => {
                let [head, inner, exit] = [(); 3].map(|()| self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                self.poll();
                let mark = self.owners.mark();
                let c = self.value(cond)?;
                self.drop_temps(mark);
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
                let less = [IntCC::SignedLessThan, IntCC::SignedLessThanOrEqual];
                let more = self
                    .b
                    .ins()
                    .icmp(less[usize::from(self.plant == 3)], at, to);
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
            Stmt::For(l, Walk::List(xs), body) => self.each(*l, xs, body)?,
            Stmt::Expr(e) => {
                self.expr(e)?;
            }
        }
        Ok(())
    }

    fn compound(&mut self, op: &Compound, old: Value, v: Value) -> Result<Value, Refusal> {
        Ok(match op {
            Compound::Int(a, site) => self.int(a, *site, old, v),
            Compound::Float(a) => self.float(a, old, v),
            Compound::Join(site) => self.text(JOIN, *site, &[old, v]),
        })
    }

    fn body(&mut self, next: Label, exit: Label, body: &Block) -> Result<(), Refusal> {
        self.loops.push((next, exit));
        self.loop_body(body)?;
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
            ExprKind::Str(s) => self.literal(s),
            ExprKind::Interp(parts, site) => self.interpolate(parts, *site)?,
            ExprKind::List(items) => self.list(ty(e), items)?,
            ExprKind::Map(entries) => self.map(ty(e), entries)?,
            ExprKind::Place(place) => self.read(place)?,
            ExprKind::Record(_, fields) => return self.construct(ty(e), 0, fields),
            ExprKind::Variant(tag, fields) => {
                return self.construct(ty(e), values::alternative(tag)?, fields);
            }
            ExprKind::Match(subject, arms) => return self.matched(e, subject, arms),
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
            ExprKind::Binary(op, x, y) if self.p.types[ty(x)] == Type::String => {
                self.text_binary(op, x, y)?
            }
            ExprKind::Binary(op, x, y) => {
                if values::held(&self.p.types[ty(x)]) {
                    let (Bin::Eq(_) | Bin::Ne(_)) = op else {
                        return Err(
                            "ordering Lists, records or variants is not compiled yet".into()
                        );
                    };
                    let t = ty(x);
                    let (x, y) = (self.value(x)?, self.value(y)?);
                    let eq = self.equal(t, x, y)?;
                    return Ok(Some(match op {
                        Bin::Ne(_) => self.b.ins().icmp_imm_s(IntCC::Equal, eq, 0),
                        _ => eq,
                    }));
                }
                let float = self.is_float(x);
                let (x, y) = (self.value(x)?, self.value(y)?);
                match op {
                    Bin::Int(a, site) => self.int(a, *site, x, y),
                    Bin::Float(a) => self.float(a, x, y),
                    _ => self.compare(op, float, x, y),
                }
            }
            ExprKind::Call(Callee::Builtin(row), given, site) => {
                let name = ROWS[*row].1;
                if matches!(given.first(), Some(Arg::Inout(_))) {
                    return self.items(name, given, *site);
                }
                let mut x = vec![];
                for a in given {
                    match a {
                        Arg::Value(e) => x.push(self.borrowed(e)?),
                        Arg::Inout(_) => return Err("an inout argument is not compiled yet".into()),
                    }
                }
                let arg = |i: usize| match &given[i] {
                    Arg::Value(e) => ty(e),
                    Arg::Inout(_) => unreachable!("taken above"),
                };
                return Ok(Some(match ROWS[*row].1 {
                    "unwrap_or(opt: Option<T>, fallback: T) -> T" => {
                        self.unwrap_or(arg(0), x[0], x[1])
                    }
                    "ok_or(opt: Option<T>, error: E) -> Result<T, E>" => {
                        self.ok_or((arg(0), x[0]), ty(e), x[1])
                    }
                    "contains(xs: List<T>, item: T) -> Bool" => {
                        let i = self.position(arg(1), x[0], x[1])?;
                        self.b
                            .ins()
                            .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, i, 0)
                    }
                    "find(xs: List<T>, item: T) -> Option<Int>" => {
                        let i = self.position(arg(1), x[0], x[1])?;
                        self.found(ty(e), i)
                    }
                    "get(m: Map<K, V>, key: K) -> Option<V>" => {
                        self.get_key((arg(0), x[0]), x[1], ty(e), *site)
                    }
                    "has(m: Map<K, V>, key: K) -> Bool" => self.has_key(arg(0), x[0], x[1]),
                    "keys(m: Map<K, V>) -> List<K>" => {
                        self.listed((arg(0), x[0]), true, ty(e), *site)
                    }
                    "values(m: Map<K, V>) -> List<V>" => {
                        self.listed((arg(0), x[0]), false, ty(e), *site)
                    }
                    row => return self.builtin(row, &x, *site, e),
                }));
            }
            ExprKind::Call(callee, given, site) => return self.call(callee, given, *site),
            ExprKind::FnValue(f) => {
                let id = match *f {
                    FnRef::Fn(id) => id,
                    FnRef::Variant(e, v) => {
                        let k = constructors(self.p).iter().position(|&c| c == (e, v));
                        self.p.fns.len() + self.p.examples.len() + k.expect("a constructor")
                    }
                };
                self.int_const(id as i64)
            }
            ExprKind::If(c, then, otherwise) => {
                let c = self.value(c)?;
                let [yes, no] = [(); 2].map(|()| self.b.create_block());
                self.b.ins().brif(c, yes, &[], no, &[]);
                let branches = [(yes, Some(then), vec![]), (no, otherwise.as_ref(), vec![])];
                return self.join(e, branches);
            }
        };
        Ok(Some(v))
    }

    /// After a trap: dead code, and a stand-in value of the node's type.
    fn unreached(&mut self, e: &Expr) -> Emitted {
        self.dead();
        let t = e.ty.map(|t| ir_type(self.p, t));
        Ok(t.map(|t| self.zero(t)))
    }

    /// Runs each branch from its label, its binds first, then meets in one block that takes the
    /// value, if any, as a temporary.
    fn join<'b>(
        &mut self,
        e: &Expr,
        branches: impl IntoIterator<Item = (Label, Option<&'b Block>, Vec<(Local, Value)>)>,
    ) -> Emitted {
        let t = e.ty.map(|t| ir_type(self.p, t));
        let merge = self.b.create_block();
        for (label, body, binds) in branches {
            self.b.switch_to_block(label);
            let v = match body {
                Some(body) => self.scoped(&binds, body)?,
                None => None,
            };
            let v = t.map(|t| v.unwrap_or_else(|| self.zero(t)));
            self.b.ins().jump(merge, &args(v.as_slice()));
        }
        self.b.switch_to_block(merge);
        Ok(t.map(|t| {
            let v = self.b.append_block_param(merge, t);
            self.temp(ty(e), v)
        }))
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
        let mark = self.owners.mark();
        let y = self.value(y)?;
        self.drop_temps(mark);
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
            Bin::Order(Cmp::Gt) if self.plant == 1 => {
                (IntCC::SignedGreaterThanOrEqual, FloatCC::GreaterThan)
            }
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
            Arith::Div | Arith::Rem => {
                let div = matches!(a, Arith::Div) || self.plant == 5;
                return self.divide(div, site, x, y);
            }
        };
        if !(self.plant == 2 && matches!(a, Arith::Sub)) {
            self.check(overflow, rt::OVERFLOW, site, &[x, y]);
        }
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
            "len(x: String) -> Int" => self.length(x[0], true),
            "len(x: List<T>) -> Int" => self.length(x[0], false),
            "len(x: Map<K, V>) -> Int" => self.map_len(x[0]),
            "is_empty(x: Map<K, V>) -> Bool" => {
                let n = self.map_len(x[0]);
                self.b.ins().icmp_imm_s(IntCC::Equal, n, 0)
            }
            "is_empty(x: String) -> Bool" | "is_empty(x: List<T>) -> Bool" => {
                let n = self.length(x[0], row.contains("String"));
                self.b.ins().icmp_imm_s(IntCC::Equal, n, 0)
            }
            "min(a: String, b: String) -> String" => self.text(MIN, site, x),
            "max(a: String, b: String) -> String" => self.text(MAX, site, x),
            "code_at(text: String, i: Int) -> Int" => self.text(CODE_AT, site, x),
            "from_code(n: Int) -> String" => self.text(FROM_CODE, site, x),
            "words(text: String) -> List<String>" => self.text(WORDS, site, x),
            "split(text: String, sep: String) -> List<String>" => self.text(SPLIT, site, x),
            "chars(text: String) -> List<String>" => self.text(CHARS, site, x),
            "lines(text: String) -> List<String>" => self.text(LINES, site, x),
            "contains(text: String, part: String) -> Bool" => self.text(CONTAINS, site, x),
            "starts_with(text: String, part: String) -> Bool" => self.text(STARTS_WITH, site, x),
            "ends_with(text: String, part: String) -> Bool" => self.text(ENDS_WITH, site, x),
            "find(text: String, part: String) -> Option<Int>" => self.text(FIND, site, x),
            "parse_int(text: String) -> Option<Int>" => self.text(PARSE_INT, site, x),
            "parse_float(text: String) -> Option<Float>" => self.text(PARSE_FLOAT, site, x),
            "join(parts: List<String>, sep: String) -> String" => self.text(JOIN_PARTS, site, x),
            "repeat(text: String, n: Int) -> String" => self.text(REPEAT, site, x),
            "pad_left(text: String, width: Int) -> String" => self.text(PAD_LEFT, site, x),
            "pad_right(text: String, width: Int) -> String" => self.text(PAD_RIGHT, site, x),
            "trim(text: String) -> String" => self.text(TRIM, site, x),
            "trim_start(text: String) -> String" => self.text(TRIM_START, site, x),
            "trim_end(text: String) -> String" => self.text(TRIM_END, site, x),
            "replace(text: String, old: String, new: String) -> String" => {
                self.text(REPLACE, site, x)
            }
            "upper(text: String) -> String" => self.text(UPPER, site, x),
            "lower(text: String) -> String" => self.text(LOWER, site, x),
            "fixed(x: Float; digits: Int) -> String" => self.text(FIXED, site, x),
            "list_of(item: T; count: Int) -> List<T>" => self.list_of(ty(e), x, site)?,
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

fn ty(e: &Expr) -> Ty {
    e.ty.expect("the node has a type")
}

mod cache;
pub use cache::Cache;
mod equal;
mod items;
mod map;
mod owned;
#[cfg(test)]
mod tests;
mod values;
mod words;
