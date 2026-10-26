//! Lowers functions over `Int`, `Bool`, `Float` and the records and variants that own nothing to
//! IR text (M6-07 to M6-09). A function is `i32 @p_<name>(args..., ptr %out)`, `%out` only when
//! it has a result; the status is 0 when it finished and 1 when it trapped, after `paka_rt1_trap`
//! recorded the trap's text (M4-05 section 5).
//! Checked `+ - *` go through the overflow intrinsics; `/`, `%` and shifts through the helpers
//! that check first (M6-01 section 3). Locals live in stack slots that LLVM turns into registers.
//! A record or variant lies in memory as M4-22's layout table gives it and is passed by address;
//! so is an `inout` argument. A value is copied whole when it is stored.

use super::constant;
use crate::core_form::{
    Arg, Arith, Arm, Bin, Block, Callee, Cmp, Compound, Expr, ExprKind, FloatArith, Fn, Place,
    Program, Root, Site, Step, Stmt, Tag, Ty, Type, Un, Walk,
};
use paka::builtins::ROWS;
use paka::layout::Layouts;
use paka::source::in_file;
use paka_rt::abi::table::{STACK_LIMIT, TRAP};
use paka_rt::ops::Int;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// The largest frame inlined, and the largest built: the runtime keeps 1 MiB under its limit.
const INLINE_FRAME: usize = 4 << 10;
const MAX_FRAME: usize = 256 << 10;

/// The IR of `update`'s statements, which leave an open block, and of every other function with
/// the constants and declarations they need; `None` when any of it is beyond this lowering.
pub fn lower(core: &mut Program, update: usize) -> Option<(String, String)> {
    let layouts = Layouts::new(core);
    let core = &*core;
    let mut module = Module {
        core,
        layouts,
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
    if !defines.is_empty() {
        code += &format!("{STACK_LIMIT}\n");
    }
    for declare in module.intrinsics.values() {
        code += declare;
    }
    Some((body, defines + &code))
}

struct Module<'a> {
    core: &'a Program,
    layouts: Layouts,
    /// Each trap text and the number of its constant `@t<n>`.
    traps: BTreeMap<String, usize>,
    /// Each intrinsic used, and its declaration.
    intrinsics: BTreeMap<String, String>,
    /// The functions called and not yet lowered.
    wanted: Vec<usize>,
    seen: BTreeSet<usize>,
}

impl Module<'_> {
    /// A whole `define`, or for `update` the lines that start its body.
    fn function(&mut self, f: &Fn, update: bool) -> Option<String> {
        let result = match f.result {
            Some(t) if !update => Some(self.ir(t).map(|_| t)?),
            _ => None,
        };
        // `update`'s message is beyond this lowering.
        let held = |n: usize| update && n < f.params.len();
        let locals = (f.locals.iter().enumerate())
            .map(|(n, &t)| self.ir(t).filter(|_| !held(n)).map(|_| t))
            .collect();
        let mut b = Body {
            module: self,
            slots: String::new(),
            frame: 0,
            lines: String::new(),
            names: 0,
            loops: vec![],
            locals,
            at: vec![],
            result,
            update,
        };
        for n in 0..f.locals.len() {
            let ty = b.locals[n].map(|t| b.ty(t));
            let by_address = f
                .params
                .get(n)
                .is_some_and(|p| p.inout || ty == Some("ptr"));
            let at = format!("%{}{n}", if by_address { 'a' } else { 'v' });
            match (ty, b.locals[n]) {
                (Some(_), Some(t)) if !by_address => b.hold(&at, t),
                _ => {}
            }
            b.at.push(at);
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
            let ty = b.ty(b.locals[n]?);
            if p.inout || ty == "ptr" {
                params.push(format!("ptr %a{n}"));
            } else {
                params.push(format!("{ty} %a{n}"));
                b.store(ty, &format!("%a{n}"), &format!("%v{n}"));
            }
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
        let (check, inline) = b.stack_check(f.site)?;
        Some(format!(
            "\ndefine internal i32 @p_{name}({params}){inline} {{\nentry:\n{}{check}{}}}\n",
            b.slots, b.lines
        ))
    }

    /// The IR type of an `Int`, a `Bool`, a `Float`, or `ptr` for a record or variant that owns
    /// nothing.
    fn ir(&self, t: Ty) -> Option<&'static str> {
        match self.core.types[t] {
            Type::Int => Some("i64"),
            Type::Bool => Some("i1"),
            Type::Float => Some("double"),
            Type::Record(_) | Type::Enum(_) | Type::Option(_) | Type::Result(..) => {
                (!self.layouts.owns(t)).then_some("ptr")
            }
            _ => None,
        }
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

struct Body<'m, 'a> {
    module: &'m mut Module<'a>,
    /// The `alloca` lines, which go in the first block, and their bytes.
    slots: String,
    frame: usize,
    lines: String,
    /// Counts the `%t` values and the `b` labels.
    names: usize,
    /// The labels `continue` and `break` go to, innermost last.
    loops: Vec<(String, String)>,
    /// Each local's type; `None` for one this lowering does not hold.
    locals: Vec<Option<Ty>>,
    /// Each local's address: its slot, or the parameter that points to it.
    at: Vec<String>,
    result: Option<Ty>,
    update: bool,
}

/// An expression's operand; an expression with no value gives an empty one.
type Value = Option<String>;

/// The fields a match arm binds: each one's type, the value's address, the field's offset in it
/// and the local it goes to.
type Binds = Vec<(Ty, String, usize, usize)>;

impl Body<'_, '_> {
    fn line(&mut self, text: String) {
        let _ = writeln!(self.lines, "  {text}");
    }

    fn slot(&mut self, name: &str, ty: &str) {
        let align = if ty == "i1" { 1 } else { 8 };
        self.frame += 8;
        let _ = writeln!(self.slots, "  {name} = alloca {ty}, align {align}");
    }

    fn ty(&self, t: Ty) -> &'static str {
        self.module.ir(t).expect("a type this lowering holds")
    }

    fn size(&self, t: Ty) -> usize {
        self.module.layouts.get(t).size
    }

    /// A slot named `name` for a value of type `t`.
    fn hold(&mut self, name: &str, t: Ty) {
        match self.ty(t) {
            "ptr" => {
                let size = self.size(t);
                self.frame += size;
                let _ = writeln!(self.slots, "  {name} = alloca [{size} x i8], align 8");
            }
            ty => self.slot(name, ty),
        }
    }

    /// The value of type `t` at `at`: loaded, or the address itself for one held in memory.
    fn get(&mut self, t: Ty, at: &str) -> String {
        match self.ty(t) {
            "ptr" => at.into(),
            ty => self.load(ty, at),
        }
    }

    /// Stores `v` of type `t` at `at`, copying a value held in memory.
    fn put(&mut self, t: Ty, v: &str, at: &str) {
        match self.ty(t) {
            "ptr" => {
                let size = self.size(t);
                let name = "@llvm.memmove.p0.p0.i64";
                self.declare(name, format!("declare void {name}(ptr, ptr, i64, i1)\n"));
                self.line(format!(
                    "call void {name}(ptr {at}, ptr {v}, i64 {size}, i1 false)"
                ));
            }
            ty => self.store(ty, v, at),
        }
    }

    /// The address `offset` bytes into `base`; no `inbounds`, since it may be passed to a call.
    fn offset(&mut self, base: &str, offset: usize) -> String {
        match offset {
            0 => base.into(),
            _ => self.op(format!("getelementptr i8, ptr {base}, i64 {offset}")),
        }
    }

    /// The address of `place` and its type.
    fn address(&mut self, place: &Place) -> Option<(String, Ty)> {
        let (mut at, mut t) = match &place.root {
            Root::Local(l) => (self.at[*l].clone(), self.locals[*l]?),
            Root::Value(e) => (self.expr(e)??, e.ty?),
        };
        for step in &place.steps {
            let Step::Field(i) = step else { return None };
            let field = self.module.layouts.get(t).alternatives[0][*i];
            if field.boxed {
                return None;
            }
            at = self.offset(&at, field.offset);
            t = field.ty;
        }
        self.module.ir(t)?;
        Some((at, t))
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

    fn declare(&mut self, name: &str, text: String) {
        self.module.intrinsics.entry(name.into()).or_insert(text);
    }

    /// Writes the result and returns status 0.
    fn finish(&mut self, value: Value) -> Option<()> {
        // No value only where a `return` came before, so nothing reaches here.
        if let (Some(t), Some(v)) = (self.result, value) {
            self.put(t, &v, "%out");
        }
        self.ended("ret i32 0".into());
        Some(())
    }

    /// The first lines of a function: a `stack` trap at `site` when its frame would reach below the
    /// runtime's limit (M6-16). A large frame is never inlined, so the frame it lands in is the one
    /// its check counted; past `MAX_FRAME` the margin under the limit might not hold it.
    fn stack_check(&mut self, site: Site) -> Option<(String, &'static str)> {
        if self.frame > MAX_FRAME {
            return None;
        }
        let inline = if self.frame > INLINE_FRAME {
            " noinline"
        } else {
            ""
        };
        let _ = writeln!(self.slots, "  %mark = alloca i8, align 1");
        let (text, len) = self.module.trap("stack", site);
        let check = format!(
            "  %mark.at = ptrtoint ptr %mark to i64\n  \
             %limit = load i64, ptr @paka_rt1_stack_limit, align 8\n  \
             %room = sub i64 %mark.at, {}\n  \
             %deep = icmp ult i64 %room, %limit\n  \
             br i1 %deep, label %stack, label %body\n\
             stack:\n  \
             call void @paka_rt1_trap(ptr {text}, i64 {len})\n  \
             ret i32 1\n\
             body:\n",
            self.frame
        );
        Some((check, inline))
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
                let v = self.expr(e)??;
                let at = self.at[*l].clone();
                self.put(self.locals[*l]?, &v, &at);
            }
            Stmt::Assign(place, op, e) => {
                let (at, t) = self.address(place)?;
                let mut v = self.expr(e)?;
                if let Some(op) = op {
                    let old = self.get(t, &at);
                    v = Some(match op {
                        Compound::Int(a, site) => self.int(a, *site, &old, &v?),
                        Compound::Float(a) => self.float(a, &old, &v?),
                        Compound::Join(_) => return None,
                    });
                }
                self.put(t, &v?, &at);
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
                let local = self.at[*l].clone();
                self.put(self.locals[*l]?, &at, &local);
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
            self.module.ir(t)?;
        }
        let v = match &e.kind {
            ExprKind::Int(n) => raw(*n).to_string(),
            ExprKind::Bool(v) => v.to_string(),
            ExprKind::Float(f) => format!("0x{:016X}", f.bits()),
            ExprKind::Place(place) => {
                let (at, t) = self.address(place)?;
                self.get(t, &at)
            }
            ExprKind::Record(_, fields) => self.construct(e.ty?, None, fields)?,
            ExprKind::Variant(tag, fields) => {
                self.construct(e.ty?, Some(alternative(tag)?), fields)?
            }
            ExprKind::Match(subject, arms) => return self.matched(e, subject, arms),
            ExprKind::Unary(Un::Neg(site), x) => {
                let x = self.expr(x)??;
                self.overflowing("ssub", *site, "0", &x)
            }
            ExprKind::Unary(Un::NegFloat, x) => {
                let x = self.expr(x)??;
                self.op(format!("fneg double {x}"))
            }
            ExprKind::Unary(Un::Not, x) => {
                let x = self.expr(x)??;
                self.op(format!("xor i1 {x}, true"))
            }
            ExprKind::Binary(op @ (Bin::And | Bin::Or), x, y) => {
                self.short(matches!(op, Bin::And), x, y)?
            }
            ExprKind::Binary(op, x, y) => {
                let ty = self.module.ir(x.ty?)?;
                if ty == "ptr" {
                    return None;
                }
                let (x, y) = (self.expr(x)??, self.expr(y)??);
                let float = ty == "double";
                let cc = match op {
                    Bin::Int(a, site) => return Some(Some(self.int(a, *site, &x, &y))),
                    Bin::Float(a) => return Some(Some(self.float(a, &x, &y))),
                    Bin::Eq(_) if float => "fcmp oeq",
                    Bin::Ne(_) if float => "fcmp une",
                    Bin::Eq(_) => "icmp eq",
                    Bin::Ne(_) => "icmp ne",
                    Bin::Order(c) if float => {
                        order(c, ["fcmp olt", "fcmp ole", "fcmp ogt", "fcmp oge"])
                    }
                    Bin::Order(c) if ty == "i64" => {
                        order(c, ["icmp slt", "icmp sle", "icmp sgt", "icmp sge"])
                    }
                    _ => return None,
                };
                self.op(format!("{cc} {ty} {x}, {y}"))
            }
            ExprKind::Call(callee, given, site) => {
                let mut args = vec![];
                for arg in given {
                    args.push(match arg {
                        Arg::Value(a) => (self.module.ir(a.ty?)?, self.expr(a)??),
                        Arg::Inout(place) => ("ptr", self.address(place)?.0),
                    });
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
        let c = self.expr(cond)??;
        let [yes, no] = ["b"; 2].map(|s| self.name(s));
        self.line(format!("br i1 {c}, label %{yes}, label %{no}"));
        let branches = [(yes, Some(then), vec![]), (no, otherwise.as_ref(), vec![])];
        self.join(e, branches)
    }

    /// Runs each branch from its label, after copying its bound fields, and gives the value of
    /// the one taken.
    fn join<'e>(
        &mut self,
        e: &Expr,
        branches: impl IntoIterator<Item = (String, Option<&'e Block>, Binds)>,
    ) -> Option<Value> {
        let ty = match e.ty {
            Some(t) => Some(self.module.ir(t)?),
            None => None,
        };
        let merge = self.name("b");
        let mut incoming = vec![];
        for (label, block, binds) in branches {
            self.start(&label);
            for (t, base, offset, l) in binds {
                let from = self.offset(&base, offset);
                let v = self.get(t, &from);
                let at = self.at[l].clone();
                self.put(self.locals[l]?, &v, &at);
            }
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

    /// A new record, or variant `alt`, of type `t` in a slot of its own.
    fn construct(&mut self, t: Ty, alt: Option<usize>, fields: &[Expr]) -> Option<String> {
        let value = self.name("%h");
        self.hold(&value, t);
        if let Some(alt) = alt {
            self.store("i64", &alt.to_string(), &value);
        }
        let places = self.module.layouts.get(t).alternatives[alt.unwrap_or(0)].clone();
        for (field, place) in fields.iter().zip(places) {
            let v = self.expr(field)??;
            let at = self.offset(&value, place.offset);
            self.put(place.ty, &v, &at);
        }
        Some(value)
    }

    /// A `match` on a variant: a switch on its tag word.
    fn matched(&mut self, e: &Expr, subject: &Expr, arms: &[Arm]) -> Option<Value> {
        let t = subject.ty?;
        let at = self.expr(subject)??;
        let tag = self.load("i64", &at);
        let labels: Vec<String> = arms.iter().map(|_| self.name("b")).collect();
        let mut cases = vec![];
        for (arm, label) in arms.iter().zip(&labels) {
            for tag in &arm.tags {
                cases.push(format!("i64 {}, label %{label}", alternative(tag)?));
            }
        }
        // The arms cover every tag, so the default is never taken.
        self.line(format!(
            "switch i64 {tag}, label %{} [ {} ]",
            labels[0],
            cases.join(" ")
        ));
        let mut branches = vec![];
        for (arm, label) in arms.iter().zip(labels) {
            let mut binds = vec![];
            if let [tag] = &arm.tags[..] {
                let alt = alternative(tag)?;
                let places = self.module.layouts.get(t).alternatives[alt].clone();
                for (bind, place) in arm.binds.iter().zip(places) {
                    if let Some(l) = bind {
                        binds.push((place.ty, at.clone(), place.offset, *l));
                    }
                }
            }
            branches.push((label, Some(&arm.body), binds));
        }
        self.join(e, branches)
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
                self.module.ir(t)?;
                let r = self.name("%r");
                self.hold(&r, t);
                list.push(format!("ptr {r}"));
                Some((t, r))
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
        Some(out.map(|(t, r)| self.get(t, &r)))
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
            "min(a: Float, b: Float) -> Float" => self.extreme(true, x[0], x[1]),
            "max(a: Float, b: Float) -> Float" => self.extreme(false, x[0], x[1]),
            "abs(x: Float) -> Float" => self.exact("@llvm.fabs.f64", x[0]),
            "sqrt(x: Float) -> Float" => self.exact("@llvm.sqrt.f64", x[0]),
            "floor(x: Float) -> Float" => self.exact("@llvm.floor.f64", x[0]),
            "ceil(x: Float) -> Float" => self.exact("@llvm.ceil.f64", x[0]),
            "trunc(x: Float) -> Float" => self.exact("@llvm.trunc.f64", x[0]),
            "round(x: Float) -> Float" => self.exact("@llvm.round.f64", x[0]),
            "to_float(n: Int) -> Float" => self.op(format!("sitofp i64 {} to double", x[0])),
            "to_int(x: Float) -> Int" => self.integer(x[0], site),
            _ => return None,
        };
        Some(Some(v))
    }

    fn pick(&mut self, cc: &str, x: &str, y: &str) -> String {
        let first = self.op(format!("icmp {cc} i64 {x}, {y}"));
        self.op(format!("select i1 {first}, i64 {x}, i64 {y}"))
    }

    fn float(&mut self, a: &FloatArith, x: &str, y: &str) -> String {
        let op = match a {
            FloatArith::Add => "fadd",
            FloatArith::Sub => "fsub",
            FloatArith::Mul => "fmul",
            FloatArith::Div => "fdiv",
        };
        self.op(format!("{op} double {x}, {y}"))
    }

    /// One of the six float intrinsics, each exact for every input (M6-01 section 3); `round` is
    /// ties away from zero, as Paka's.
    fn exact(&mut self, name: &str, x: &str) -> String {
        let declare = format!("declare double {name}(double)\n");
        self.declare(name, declare);
        self.op(format!("call double {name}(double {x})"))
    }

    /// `min` or `max`: NaN when either is NaN; of two equal values the one whose sign `1 / x`
    /// shows, so `-0.0` is below `0.0`.
    fn extreme(&mut self, min: bool, x: &str, y: &str) -> String {
        let (first, other) = if min { ("olt", "ogt") } else { ("ogt", "olt") };
        let inverse = self.op(format!("fdiv double 1.0, {x}"));
        let negative = self.op(format!("fcmp olt double {inverse}, 0.0"));
        let (on_negative, on_positive) = if min { (x, y) } else { (y, x) };
        let tie = self.op(format!(
            "select i1 {negative}, double {on_negative}, double {on_positive}"
        ));
        let below = self.op(format!("fcmp {first} double {x}, {y}"));
        let v = self.op(format!("select i1 {below}, double {x}, double {tie}"));
        let above = self.op(format!("fcmp {other} double {x}, {y}"));
        let v = self.op(format!("select i1 {above}, double {y}, double {v}"));
        let nan = self.op(format!("fcmp uno double {x}, {y}"));
        self.op(format!(
            "select i1 {nan}, double 0x7FF8000000000000, double {v}"
        ))
    }

    /// `fptosi` after the check that `x` is in -2^63 to below 2^63, which NaN fails; `x` is
    /// selected to 0.0 where the instruction would be poison (M6-01 section 3).
    fn integer(&mut self, x: &str, site: Site) -> String {
        let low = self.op(format!("fcmp oge double {x}, 0xC3E0000000000000"));
        let high = self.op(format!("fcmp olt double {x}, 0x43E0000000000000"));
        let inside = self.op(format!("and i1 {low}, {high}"));
        let bad = self.op(format!("xor i1 {inside}, true"));
        self.check(&bad, "bad_conversion", site);
        let safe = self.op(format!("select i1 {bad}, double 0.0, double {x}"));
        self.op(format!("fptosi double {safe} to i64"))
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
    fn overflowing(&mut self, op: &str, site: Site, x: &str, y: &str) -> String {
        let name = format!("@llvm.{op}.with.overflow.i64");
        let declare = format!("declare {{ i64, i1 }} {name}(i64, i64)\n");
        self.declare(&name, declare);
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
    match ty {
        "i1" => "false",
        "double" => "0.0",
        "ptr" => "null",
        _ => "0",
    }
    .into()
}

/// The condition code of `c` among `[lt, le, gt, ge]`.
fn order(c: &Cmp, codes: [&'static str; 4]) -> &'static str {
    codes[match c {
        Cmp::Lt => 0,
        Cmp::Le => 1,
        Cmp::Gt => 2,
        Cmp::Ge => 3,
    }]
}

/// The tag of a variant: its position, `Some` and `Ok` first.
fn alternative(tag: &Tag) -> Option<usize> {
    match tag {
        Tag::User(_, i) => Some(*i),
        Tag::Some | Tag::Ok => Some(0),
        Tag::None | Tag::Err => Some(1),
        Tag::Effect(_) => None,
    }
}

/// A literal leaves Paka here, into the IR.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}
