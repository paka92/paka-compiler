//! Lowers functions over `Int` and `Bool` to IR text (M6-07). A function is
//! `i32 @p_<name>(args..., ptr %out)`, `%out` only when it has a result; the status is 0 when it
//! finished and 1 when it trapped, after `paka_rt1_trap` recorded the trap's text (M4-05
//! section 5).
//! Checked `+ - *` go through the overflow intrinsics; `/`, `%` and shifts through the helpers
//! that check first (M6-01 section 3). Locals live in stack slots that LLVM turns into registers.

use super::constant;
use crate::core_form::{
    Arg, Arith, Bin, Block, Callee, Cmp, Compound, Expr, ExprKind, Fn, Place, Program, Root, Site,
    Stmt, Type, Un, Walk,
};
use paka::builtins::ROWS;
use paka::source::in_file;
use paka_rt::ops::Int;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const TRAP: &str = "declare void @paka_rt1_trap(ptr, i64) cold nounwind";

/// The IR of `update`'s statements, which leave an open block, and of every other function with
/// the constants and declarations they need; `None` when any of it is beyond this lowering.
pub fn lower(core: &Program, update: usize) -> Option<(String, String)> {
    let mut module = Module {
        core,
        traps: BTreeMap::new(),
        intrinsics: BTreeMap::new(),
        wanted: vec![],
        seen: BTreeSet::from([update]),
    };
    let body = module.function(&core.fns[update], true)?;
    let mut defines = String::new();
    while let Some(id) = module.wanted.pop() {
        defines += &module.function(&core.fns[id], false)?;
    }
    let mut code = String::new();
    for (text, n) in &module.traps {
        code += &constant(&format!("t{n}"), text);
    }
    if !module.traps.is_empty() {
        code += &format!("\n{TRAP}\n");
    }
    for declare in module.intrinsics.values() {
        code += declare;
    }
    Some((body, defines + &code))
}

struct Module<'a> {
    core: &'a Program,
    /// Each trap text and the number of its constant `@t<n>`.
    traps: BTreeMap<String, usize>,
    /// Each intrinsic used, and its declaration.
    intrinsics: BTreeMap<&'static str, String>,
    /// The functions called and not yet lowered.
    wanted: Vec<usize>,
    seen: BTreeSet<usize>,
}

impl Module<'_> {
    /// A whole `define`, or for `update` the lines that start its body.
    fn function(&mut self, f: &Fn, update: bool) -> Option<String> {
        let core = self.core;
        let mut b = Body {
            module: self,
            slots: String::new(),
            lines: String::new(),
            names: 0,
            loops: vec![],
            locals: f.locals.iter().map(|&t| ir(core, t)).collect(),
            result: f.result.map(|t| ir(core, t)),
            update,
        };
        for (n, ty) in b.locals.clone().into_iter().enumerate() {
            if let Some(ty) = ty {
                b.slot(&format!("%v{n}"), ty);
            }
        }
        if update {
            b.statements(&f.body.stmts)?;
            let empty = b.slots.is_empty() && b.lines.is_empty();
            return Some(match empty {
                true => String::new(),
                false => format!("entry:\n{}{}", b.slots, b.lines),
            });
        }
        let mut params = vec![];
        for (n, p) in f.params.iter().enumerate() {
            let ty = b.locals[n].filter(|_| !p.inout)?;
            params.push(format!("{ty} %a{n}"));
            b.store(ty, &format!("%a{n}"), &format!("%v{n}"));
        }
        if f.result.is_some() {
            params.push("ptr %out".into());
        }
        let value = b.block(&f.body)?;
        b.finish(value)?;
        // Drops the label of the block nothing reaches that `finish` opened.
        let end = b.lines.trim_end().rfind('\n').map_or(0, |i| i + 1);
        b.lines.truncate(end);
        let (name, params) = (&f.name, params.join(", "));
        Some(format!(
            "\ndefine internal i32 @p_{name}({params}) {{\nentry:\n{}{}}}\n",
            b.slots, b.lines
        ))
    }

    /// The constant naming a trap of `kind` at `site`, and its length.
    fn trap(&mut self, kind: &str, site: Site) -> (String, usize) {
        let at = &self.core.sites[site];
        let ((file, _), line) = in_file(&self.core.files, |f| f.1, at.line);
        let text = format!("{kind} at {file}:{line}:{}", at.col);
        let next = self.traps.len();
        let n = *self.traps.entry(text.clone()).or_insert(next);
        (format!("@t{n}"), text.len())
    }
}

/// The IR type of an `Int` or a `Bool`.
fn ir(core: &Program, t: usize) -> Option<&'static str> {
    match core.types[t] {
        Type::Int => Some("i64"),
        Type::Bool => Some("i1"),
        _ => None,
    }
}

struct Body<'m, 'a> {
    module: &'m mut Module<'a>,
    /// The `alloca` lines, which go in the first block.
    slots: String,
    lines: String,
    /// Counts the `%t` values and the `b` labels.
    names: usize,
    /// The labels `continue` and `break` go to, innermost last.
    loops: Vec<(String, String)>,
    /// Each local's IR type; `None` for one this lowering does not hold.
    locals: Vec<Option<&'static str>>,
    /// `Some(None)` for a result this lowering does not hold.
    result: Option<Option<&'static str>>,
    update: bool,
}

/// An expression's operand; an expression with no value gives an empty one.
type Value = Option<String>;

impl Body<'_, '_> {
    fn line(&mut self, text: String) {
        let _ = writeln!(self.lines, "  {text}");
    }

    fn slot(&mut self, name: &str, ty: &str) {
        let align = if ty == "i1" { 1 } else { 8 };
        let _ = writeln!(self.slots, "  {name} = alloca {ty}, align {align}");
    }

    fn name(&mut self, stem: &str) -> String {
        self.names += 1;
        format!("{stem}{}", self.names)
    }

    /// `inst` into a new value.
    fn op(&mut self, inst: String) -> String {
        let t = self.name("%t");
        self.line(format!("{t} = {inst}"));
        t
    }

    fn start(&mut self, label: &str) {
        let _ = writeln!(self.lines, "{label}:");
    }

    /// Ends the block; whatever follows is in a block nothing reaches.
    fn ended(&mut self, last: String) {
        self.line(last);
        let dead = self.name("b");
        self.start(&dead);
    }

    fn load(&mut self, ty: &str, at: &str) -> String {
        let align = if ty == "i1" { 1 } else { 8 };
        self.op(format!("load {ty}, ptr {at}, align {align}"))
    }

    fn store(&mut self, ty: &str, v: &str, at: &str) {
        let align = if ty == "i1" { 1 } else { 8 };
        self.line(format!("store {ty} {v}, ptr {at}, align {align}"));
    }

    /// Writes the result and returns status 0.
    fn finish(&mut self, value: Value) -> Option<()> {
        if let Some(ty) = self.result {
            let ty = ty?;
            // No value only where a `return` came before, so nothing reaches here.
            let v = value.unwrap_or_else(|| stand_in(ty));
            self.store(ty, &v, "%out");
        }
        self.ended("ret i32 0".into());
        Some(())
    }

    /// Traps with `kind` at `site` when `bad` holds.
    fn check(&mut self, bad: &str, kind: &str, site: Site) {
        let (trap, ok) = (self.name("b"), self.name("b"));
        self.line(format!("br i1 {bad}, label %{trap}, label %{ok}"));
        self.start(&trap);
        self.trap(kind, site);
        self.start(&ok);
    }

    fn trap(&mut self, kind: &str, site: Site) {
        let (text, len) = self.module.trap(kind, site);
        self.line(format!("call void @paka_rt1_trap(ptr {text}, i64 {len})"));
        self.line("ret i32 1".into());
    }

    fn block(&mut self, block: &Block) -> Option<Value> {
        self.statements(&block.stmts)?;
        match &block.value {
            Some(e) => self.expr(e),
            None => Some(None),
        }
    }

    fn statements(&mut self, stmts: &[Stmt]) -> Option<()> {
        stmts.iter().try_for_each(|s| self.stmt(s))
    }

    fn stmt(&mut self, s: &Stmt) -> Option<()> {
        match s {
            Stmt::Let(l, e) => {
                let v = self.expr(e)?;
                self.store(self.locals[*l]?, &v?, &format!("%v{l}"));
            }
            Stmt::Assign(place, op, e) => {
                let l = local(place)?;
                let ty = self.locals[l]?;
                let at = format!("%v{l}");
                let mut v = self.expr(e)?;
                if let Some(op) = op {
                    let Compound::Int(a, site) = op else {
                        return None;
                    };
                    let old = self.load(ty, &at);
                    v = Some(self.int(a, *site, &old, &v?));
                }
                self.store(ty, &v?, &at);
            }
            Stmt::Return(e) if !self.update => {
                let v = match e {
                    Some(e) => self.expr(e)?,
                    None => None,
                };
                self.finish(v)?;
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = self.loops.last()?.clone();
                let to = if matches!(s, Stmt::Break) { exit } else { next };
                self.ended(format!("br label %{to}"));
            }
            Stmt::While(cond, body) => {
                let [head, inner, exit] = ["b"; 3].map(|s| self.name(s));
                self.line(format!("br label %{head}"));
                self.start(&head);
                let c = self.expr(cond)??;
                self.line(format!("br i1 {c}, label %{inner}, label %{exit}"));
                self.start(&inner);
                self.looped(&head, &exit, body)?;
                self.line(format!("br label %{head}"));
                self.start(&exit);
            }
            Stmt::For(l, Walk::Range(from, to), body) => {
                let (from, to) = (self.expr(from)??, self.expr(to)??);
                let i = self.name("%i");
                self.slot(&i, "i64");
                self.store("i64", &from, &i);
                let [head, inner, next, exit] = ["b"; 4].map(|s| self.name(s));
                self.line(format!("br label %{head}"));
                self.start(&head);
                let at = self.load("i64", &i);
                let more = self.op(format!("icmp slt i64 {at}, {to}"));
                self.line(format!("br i1 {more}, label %{inner}, label %{exit}"));
                self.start(&inner);
                self.store(self.locals[*l]?, &at, &format!("%v{l}"));
                self.looped(&next, &exit, body)?;
                self.line(format!("br label %{next}"));
                self.start(&next);
                // `at < to`, so the step never passes the Int range.
                let at = self.load("i64", &i);
                let step = self.op(format!("add i64 {at}, 1"));
                self.store("i64", &step, &i);
                self.line(format!("br label %{head}"));
                self.start(&exit);
            }
            Stmt::Expr(e) => drop(self.expr(e)?),
            Stmt::Return(_) | Stmt::For(..) => return None,
        }
        Some(())
    }

    fn looped(&mut self, next: &str, exit: &str, body: &Block) -> Option<()> {
        self.loops.push((next.into(), exit.into()));
        self.block(body)?;
        self.loops.pop();
        Some(())
    }

    /// `None` when `e` is beyond this lowering; then `Some` of its operand, if it has a value.
    fn expr(&mut self, e: &Expr) -> Option<Value> {
        if let Some(t) = e.ty {
            ir(self.module.core, t)?;
        }
        let v = match &e.kind {
            ExprKind::Int(n) => raw(*n).to_string(),
            ExprKind::Bool(v) => v.to_string(),
            ExprKind::Place(place) => {
                let l = local(place)?;
                self.load(self.locals[l]?, &format!("%v{l}"))
            }
            ExprKind::Unary(Un::Neg(site), x) => {
                let x = self.expr(x)??;
                self.overflowing("ssub", *site, "0", &x)
            }
            ExprKind::Unary(Un::Not, x) => {
                let x = self.expr(x)??;
                self.op(format!("xor i1 {x}, true"))
            }
            ExprKind::Binary(op @ (Bin::And | Bin::Or), x, y) => {
                self.short(matches!(op, Bin::And), x, y)?
            }
            ExprKind::Binary(op, x, y) => {
                let ty = ir(self.module.core, x.ty?)?;
                let (x, y) = (self.expr(x)??, self.expr(y)??);
                let cc = match op {
                    Bin::Int(a, site) => return Some(Some(self.int(a, *site, &x, &y))),
                    Bin::Eq(_) => "eq",
                    Bin::Ne(_) => "ne",
                    Bin::Order(_) if ty != "i64" => return None,
                    Bin::Order(Cmp::Lt) => "slt",
                    Bin::Order(Cmp::Le) => "sle",
                    Bin::Order(Cmp::Gt) => "sgt",
                    Bin::Order(Cmp::Ge) => "sge",
                    _ => return None,
                };
                self.op(format!("icmp {cc} {ty} {x}, {y}"))
            }
            ExprKind::Call(callee, given, site) => {
                let mut args = vec![];
                for arg in given {
                    let Arg::Value(a) = arg else { return None };
                    let ty = ir(self.module.core, a.ty?)?;
                    args.push((ty, self.expr(a)??));
                }
                return match callee {
                    Callee::Fn(id) => self.call(*id, &args),
                    Callee::Builtin(row) => self.builtin(ROWS.get(*row)?.1, &args, *site),
                    Callee::Value(_) => None,
                };
            }
            ExprKind::If(cond, then, otherwise) => return self.choose(e, cond, then, otherwise),
            _ => return None,
        };
        Some(Some(v))
    }

    /// `x && y` or `x || y`: `y` runs only when `x` does not decide.
    fn short(&mut self, and: bool, x: &Expr, y: &Expr) -> Option<String> {
        let x = self.expr(x)??;
        let from = self.current();
        let [rhs, merge] = ["b"; 2].map(|s| self.name(s));
        let (yes, no) = if and { (&rhs, &merge) } else { (&merge, &rhs) };
        self.line(format!("br i1 {x}, label %{yes}, label %{no}"));
        self.start(&rhs);
        let y = self.expr(y)??;
        let end = self.current();
        self.line(format!("br label %{merge}"));
        self.start(&merge);
        Some(self.op(format!("phi i1 [ {}, %{from} ], [ {y}, %{end} ]", !and)))
    }

    /// The label of the block being written.
    fn current(&self) -> String {
        let last = self.lines.lines().rev().find(|l| !l.starts_with(' '));
        last.map_or("entry".into(), |l| l.trim_end_matches(':').into())
    }

    fn choose(
        &mut self,
        e: &Expr,
        cond: &Expr,
        then: &Block,
        otherwise: &Option<Block>,
    ) -> Option<Value> {
        let ty = match e.ty {
            Some(t) => Some(ir(self.module.core, t)?),
            None => None,
        };
        let c = self.expr(cond)??;
        let [yes, no, merge] = ["b"; 3].map(|s| self.name(s));
        self.line(format!("br i1 {c}, label %{yes}, label %{no}"));
        let mut incoming = vec![];
        for (label, block) in [(yes, Some(then)), (no, otherwise.as_ref())] {
            self.start(&label);
            let v = match block {
                Some(block) => self.block(block)?,
                None => None,
            };
            let v = v.unwrap_or_else(|| stand_in(ty.unwrap_or("i64")));
            incoming.push(format!("[ {v}, %{} ]", self.current()));
            self.line(format!("br label %{merge}"));
        }
        self.start(&merge);
        Some(ty.map(|ty| self.op(format!("phi {ty} {}", incoming.join(", ")))))
    }

    /// Calls a Paka function and passes a trap on.
    fn call(&mut self, id: usize, args: &[(&str, String)]) -> Option<Value> {
        let core = self.module.core;
        let f = &core.fns[id];
        if self.module.seen.insert(id) {
            self.module.wanted.push(id);
        }
        let mut list: Vec<String> = args.iter().map(|(ty, v)| format!("{ty} {v}")).collect();
        let out = match f.result {
            Some(t) => {
                let ty = ir(core, t)?;
                let r = self.name("%r");
                self.slot(&r, ty);
                list.push(format!("ptr {r}"));
                Some((ty, r))
            }
            None => None,
        };
        let status = self.op(format!("call i32 @p_{}({})", f.name, list.join(", ")));
        let failed = self.op(format!("icmp ne i32 {status}, 0"));
        let (pass, ok) = (self.name("b"), self.name("b"));
        self.line(format!("br i1 {failed}, label %{pass}, label %{ok}"));
        self.start(&pass);
        self.line(format!("ret i32 {status}"));
        self.start(&ok);
        Some(out.map(|(ty, r)| self.load(ty, &r)))
    }

    fn builtin(&mut self, row: &str, args: &[(&str, String)], site: Site) -> Option<Value> {
        let x: Vec<&str> = args.iter().map(|(_, v)| v.as_str()).collect();
        let v = match row {
            "assert(cond: Bool)" => {
                let failed = self.op(format!("xor i1 {}, true", x[0]));
                self.check(&failed, "assert", site);
                return Some(None);
            }
            "min(a: Int, b: Int) -> Int" => self.pick("slt", x[0], x[1]),
            "max(a: Int, b: Int) -> Int" => self.pick("sgt", x[0], x[1]),
            "abs(x: Int) -> Int" => {
                let min = self.op(format!("icmp eq i64 {}, {}", x[0], i64::MIN));
                self.check(&min, "overflow", site);
                let flipped = self.op(format!("sub i64 0, {}", x[0]));
                self.pick("sgt", x[0], &flipped)
            }
            "wrap_add(a: Int, b: Int) -> Int" => self.op(format!("add i64 {}, {}", x[0], x[1])),
            "wrap_sub(a: Int, b: Int) -> Int" => self.op(format!("sub i64 {}, {}", x[0], x[1])),
            "wrap_mul(a: Int, b: Int) -> Int" => self.op(format!("mul i64 {}, {}", x[0], x[1])),
            "bit_and(a: Int, b: Int) -> Int" => self.op(format!("and i64 {}, {}", x[0], x[1])),
            "bit_or(a: Int, b: Int) -> Int" => self.op(format!("or i64 {}, {}", x[0], x[1])),
            "bit_xor(a: Int, b: Int) -> Int" => self.op(format!("xor i64 {}, {}", x[0], x[1])),
            "bit_not(a: Int) -> Int" => self.op(format!("xor i64 {}, -1", x[0])),
            "shift_left(a: Int, count: Int) -> Int" => self.shift("shl", x[0], x[1], site),
            "shift_right(a: Int, count: Int) -> Int" => self.shift("ashr", x[0], x[1], site),
            _ => return None,
        };
        Some(Some(v))
    }

    fn pick(&mut self, cc: &str, x: &str, y: &str) -> String {
        let first = self.op(format!("icmp {cc} i64 {x}, {y}"));
        self.op(format!("select i1 {first}, i64 {x}, i64 {y}"))
    }

    fn int(&mut self, a: &Arith, site: Site, x: &str, y: &str) -> String {
        match a {
            Arith::Add => self.overflowing("sadd", site, x, y),
            Arith::Sub => self.overflowing("ssub", site, x, y),
            Arith::Mul => self.overflowing("smul", site, x, y),
            Arith::Div => self.divide("sdiv", site, x, y),
            Arith::Rem => self.divide("srem", site, x, y),
        }
    }

    /// `x op y` through `@llvm.<op>.with.overflow.i64`, trapping `overflow`.
    fn overflowing(&mut self, op: &'static str, site: Site, x: &str, y: &str) -> String {
        let name = format!("@llvm.{op}.with.overflow.i64");
        let declare = format!("declare {{ i64, i1 }} {name}(i64, i64)\n");
        self.module.intrinsics.entry(op).or_insert(declare);
        let pair = self.op(format!("call {{ i64, i1 }} {name}(i64 {x}, i64 {y})"));
        let over = self.op(format!("extractvalue {{ i64, i1 }} {pair}, 1"));
        self.check(&over, "overflow", site);
        self.op(format!("extractvalue {{ i64, i1 }} {pair}, 0"))
    }

    /// `sdiv` or `srem` after its checks. The divisor is selected to 1 where the instruction would
    /// be undefined: 0, and -1 (under the smallest Int for `sdiv`, which traps; `x % -1` is 0 as
    /// `x % 1` is), so the instruction is defined for every input (M6-01 section 3).
    fn divide(&mut self, op: &str, site: Site, x: &str, y: &str) -> String {
        let zero = self.op(format!("icmp eq i64 {y}, 0"));
        self.check(&zero, "divide_by_zero", site);
        let mut risky = self.op(format!("icmp eq i64 {y}, -1"));
        if op == "sdiv" {
            let min = self.op(format!("icmp eq i64 {x}, {}", i64::MIN));
            risky = self.op(format!("and i1 {min}, {risky}"));
            self.check(&risky, "overflow", site);
        }
        let risky = self.op(format!("or i1 {zero}, {risky}"));
        let d = self.op(format!("select i1 {risky}, i64 1, i64 {y}"));
        self.op(format!("{op} i64 {x}, {d}"))
    }

    /// `shl` or `ashr` after the 0 to 63 check; the count is masked to that range, so the
    /// instruction is defined for every input (M6-01 section 3).
    fn shift(&mut self, op: &str, x: &str, count: &str, site: Site) -> String {
        let outside = self.op(format!("icmp ugt i64 {count}, 63"));
        self.check(&outside, "out_of_bounds", site);
        let k = self.op(format!("and i64 {count}, 63"));
        self.op(format!("{op} i64 {x}, {k}"))
    }
}

/// The value of a branch nothing reaches.
fn stand_in(ty: &str) -> String {
    if ty == "i1" { "false" } else { "0" }.into()
}

/// The local a place names, when it has no steps.
fn local(place: &Place) -> Option<usize> {
    match (&place.root, &place.steps[..]) {
        (Root::Local(l), []) => Some(*l),
        _ => None,
    }
}

/// A literal leaves Paka here, into the IR.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}
