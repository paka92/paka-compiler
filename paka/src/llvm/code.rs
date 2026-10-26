//! Lowers functions over `Int`, `Bool`, `Float`, `String`, records and variants to IR text (M6-07
//! to M6-11). A function is `i32 @p_<name>(args..., ptr %out)`, `%out` only when
//! it has a result; the status is 0 when it finished and 1 when it trapped, after `paka_rt1_trap`
//! recorded the trap's text (M4-05 section 5).
//! Checked `+ - *` go through the overflow intrinsics; `/`, `%` and shifts through the helpers
//! that check first (M6-01 section 3). Locals live in stack slots that LLVM turns into registers.
//! A record or variant lies in memory as M4-22's layout table gives it and is passed by address;
//! so is an `inout` argument. A value is copied whole when it is stored. Who owns what, and so
//! what is dropped where, follows the dev back end (`owned`); Strings go through the runtime
//! (`text`).

use super::constant;
use crate::core_form::{
    Arg, Arith, Arm, Bin, Block, Callee, Cmp, Compound, Effect, Expr, ExprKind, FloatArith, Fn,
    FnRef, Grant, Local, Place, Program, Root, Site, Step, Stmt, Tag, Ty, Type, Un, Walk,
};
use paka::builtins::ROWS;
use paka::layout::{self, Layouts};
use paka::owners::Owners;
use paka::source::in_file;
use paka_rt::abi::dev::records::Spec;
use paka_rt::abi::dev::words::{INDEX, JOIN, SLICE};
use paka_rt::abi::table::{ALLOC, STACK_LIMIT, TRAP};
use paka_rt::ops::Int;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

mod items;
mod map;
mod owned;
mod rows;
mod sort;
mod text;

/// The largest frame inlined, and the largest built: the runtime keeps 1 MiB under its limit.
const INLINE_FRAME: usize = 4 << 10;
const MAX_FRAME: usize = 256 << 10;

/// The IR of `update`'s body, and of every other function with the constants and declarations
/// they need; `None` when any of it is beyond this lowering. With `computed`, the body builds the
/// effect list `update` ends in and returns; otherwise it leaves an open block for the constant
/// list to be stored.
pub fn lower(core: &mut Program, update: usize, computed: bool) -> Option<(String, String)> {
    let layouts = Layouts::new(core);
    let core = &*core;
    let mut module = Module::new(core, layouts, true);
    module.seen.insert(update);
    let body = module.function(&core.fns[update], Some(computed))?;
    Some((body, module.rest(core.fns[update].site)?))
}

/// The IR of a headless app beyond the M1 view (ABI section 13): every function its handler and
/// state reach, `@app.call`, and the `@app` constant `paka_rt1_app` takes; `None` when any of it
/// is beyond this lowering.
pub fn lower_app(core: &mut Program) -> Option<String> {
    let layouts = Layouts::new(core);
    let core = &*core;
    let (app, mut module) = (core.app.as_ref()?, Module::new(core, layouts, false));
    let update = &core.fns[app.update];
    let (msg, effects) = (update.params.last()?.ty, update.result?);
    let string = core.types.iter().position(|t| *t == Type::String)?;
    let answer = core
        .types
        .iter()
        .position(|t| *t == Type::Fn(vec![string], Some(msg)))?;
    let Type::Enum(e) = core.types[msg] else {
        return None;
    };
    let start = app.start?;
    module.seen.insert(app.update);
    let mut defines = module.function(update, None)?;
    let state = app.state.as_ref().map(|value| Fn {
        name: ".state".into(),
        site: update.site,
        params: vec![],
        locals: vec![],
        result: value.ty,
        body: Block {
            stmts: vec![],
            value: Some(Box::new(value.clone())),
        },
        claims: vec![],
    });
    if let Some(f) = &state {
        defines += &module.function(f, None)?;
    }
    module.dispatched.insert(answer);
    let handler = ["ptr %a", "ptr %b", "ptr %c"][..=update.params.len()].join(", ");
    let fields = core.enums[e].variants[start].fields.iter();
    let fields = fields
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let record = |m: &mut Module, t: Ty| match m.specs[t] {
        Some(_) => m.record(t),
        None => "null".into(),
    };
    let typed =
        |m: &mut Module, t: Ty| format!("ptr {}, i64 {}", record(m, t), m.layouts.get(t).size);
    let state_ty = update
        .params
        .first()
        .filter(|_| state.is_some())
        .map(|p| p.ty);
    let words = [
        format!("ptr @name, i64 {}", app.name.len()),
        "ptr @app.call".into(),
        format!("i64 {}", grants(&app.grants)),
        format!("i64 {start}"),
        match fields.is_empty() {
            true => "ptr null, i64 0".into(),
            false => format!("ptr @fields, i64 {}", fields.len()),
        },
        typed(&mut module, msg),
        format!("i64 {}", u64::from(state_ty.is_some())),
        match state_ty {
            Some(t) => typed(&mut module, t),
            None => "ptr null, i64 0".into(),
        },
        format!("ptr {}", record(&mut module, effects)),
    ];
    let mut code = constant("name", &app.name);
    if !fields.is_empty() {
        code += &constant("fields", &fields);
    }
    code += &format!(
        "@app = private constant {{ ptr, i64, ptr, i64, i64, ptr, i64, ptr, i64, i64, ptr, i64, ptr }} {{ {} }}

define internal i32 @app.call(i64 %id, ptr %a, ptr %b, ptr %c) {{
entry:
  switch i64 %id, label %answer [ i64 -1, label %handler i64 -2, label %state ]
handler:
  %h = call i32 @p_{}({handler})
  ret i32 %h
state:
  {}
answer:
  %v = call i32 @v{answer}(i64 %id, ptr %a, ptr %b)
  ret i32 %v
}}
",
        words.join(", "),
        update.name,
        match state {
            Some(_) => "%s = call i32 @p_.state(ptr %a)\n  ret i32 %s",
            None => "ret i32 1",
        },
    );
    Some(code + &defines + &module.rest(update.site)?)
}

/// The grants word of ABI section 13: bit 0 `Console`, bit 1 `ReadFiles`, bit 2 `WriteFiles`.
fn grants(grants: &BTreeSet<Grant>) -> u64 {
    let bit = |g: &Grant| match g {
        Grant::Console => 1,
        Grant::ReadFiles => 2,
        Grant::WriteFiles => 4,
    };
    grants.iter().map(bit).sum()
}

struct Module<'a> {
    core: &'a Program,
    layouts: Layouts,
    /// Each type's record contents for the runtime; `None` for one that owns nothing.
    specs: Vec<Option<Spec>>,
    /// Whether this lowering holds values of each type.
    held: Vec<bool>,
    /// Each byte constant, a String literal or the text of a trap, and the number of its `@t<n>`.
    texts: BTreeMap<String, usize>,
    /// The type records used, as constants `@r<t>`.
    records: BTreeMap<Ty, String>,
    /// Each intrinsic and runtime entry used, and its declaration.
    intrinsics: BTreeMap<String, String>,
    /// The functions called and not yet lowered.
    wanted: Vec<usize>,
    seen: BTreeSet<usize>,
    /// The types compared whole, and those whose equality is not yet lowered.
    equalities: Vec<Ty>,
    compared: BTreeSet<Ty>,
    /// Each function value made, its id its position, and its type.
    fn_values: Vec<(FnRef, Ty)>,
    /// The function types called through a value.
    dispatched: BTreeSet<Ty>,
    /// The sort functions called: by item type, and the type of `before` for `sort_by`.
    sorts: BTreeSet<(Ty, Option<Ty>)>,
    /// For the M1 run loop, which performs `Print` and `Exit` only.
    m1: bool,
}

impl<'a> Module<'a> {
    fn new(core: &'a Program, layouts: Layouts, m1: bool) -> Module<'a> {
        let mut held: Vec<bool> = (core.types.iter())
            .map(|t| {
                matches!(
                    t,
                    Type::Int
                        | Type::Bool
                        | Type::Float
                        | Type::String
                        | Type::List(_)
                        | Type::Map(..)
                        | Type::Record(_)
                        | Type::Enum(_)
                        | Type::Option(_)
                        | Type::Result(..)
                        | Type::Effect(_)
                        | Type::Fn(..)
                )
            })
            .collect();
        // A type is held when every field or item it has is; a cycle holds unless something on
        // it is not.
        while let Some(t) = (0..held.len()).find(|&t| {
            let fields = layouts.get(t).alternatives.iter().flatten();
            let item = match &core.types[t] {
                Type::List(item) => held[*item],
                Type::Map(k, v) => held[*k] && held[*v],
                Type::Fn(params, result) => params.iter().chain(result).all(|&p| held[p]),
                _ => true,
            };
            held[t] && !(item && fields.clone().all(|p| held[p.ty]))
        }) {
            held[t] = false;
        }
        Module {
            core,
            specs: layouts.specs(),
            layouts,
            held,
            texts: BTreeMap::new(),
            records: BTreeMap::new(),
            intrinsics: BTreeMap::new(),
            wanted: vec![],
            seen: BTreeSet::new(),
            equalities: vec![],
            compared: BTreeSet::new(),
            fn_values: vec![],
            dispatched: BTreeSet::new(),
            sorts: BTreeSet::new(),
            m1,
        }
    }

    /// Every function, equality, sort and dispatcher still wanted, then the constants and
    /// declarations; `site` is where their own traps are reported.
    fn rest(&mut self, site: Site) -> Option<String> {
        let mut defines = String::new();
        loop {
            if let Some(id) = self.wanted.pop() {
                defines += &self.function(&self.core.fns[id], None)?;
            } else if let Some(t) = self.equalities.pop() {
                defines += &self.body(site, None).equality(t);
            } else {
                break;
            }
        }
        for (item, by) in std::mem::take(&mut self.sorts) {
            defines += &self.body(site, None).sorter(item, by)?;
        }
        for t in std::mem::take(&mut self.dispatched) {
            defines += &self.dispatcher(t, site)?;
        }
        let mut code = String::new();
        for (text, n) in &self.texts {
            code += &constant(&format!("t{n}"), text);
        }
        for record in self.records.values() {
            code += record;
        }
        if !code.is_empty() {
            code += "\n";
        }
        if !defines.is_empty() {
            code += &format!("{STACK_LIMIT}\n");
        }
        for declare in self.intrinsics.values() {
            code += declare;
        }
        Some(defines + &code)
    }

    /// The tag of a variant: its position, `Some` and `Ok` first. No run loop performs a
    /// foreign effect.
    fn alternative(&self, tag: &Tag) -> Option<usize> {
        match tag {
            Tag::User(_, i) => Some(*i),
            Tag::Some | Tag::Ok => Some(0),
            Tag::None | Tag::Err => Some(1),
            Tag::Effect(Effect::Foreign) => None,
            Tag::Effect(e @ (Effect::Print | Effect::Exit)) => Some(*e as usize),
            Tag::Effect(e) => (!self.m1).then_some(*e as usize),
        }
    }

    fn body(&mut self, site: Site, update: Option<bool>) -> Body<'_, 'a> {
        Body {
            module: self,
            slots: String::new(),
            frame: 0,
            lines: String::new(),
            names: 0,
            loops: vec![],
            locals: vec![],
            at: vec![],
            result: None,
            update,
            site,
            owners: Owners::default(),
            temps: vec![],
        }
    }

    /// A whole `define`, or `update`'s body; `update` is `Some(computed)` for it.
    fn function(&mut self, f: &Fn, update: Option<bool>) -> Option<String> {
        let result = match f.result {
            Some(t) if update.is_none() => Some(self.ir(t).map(|_| t)?),
            _ => None,
        };
        let locals = (f.locals.iter()).map(|&t| self.ir(t).map(|_| t)).collect();
        let mut b = self.body(f.site, update);
        (b.locals, b.result) = (locals, result);
        for n in 0..f.locals.len() {
            if update.is_some() && n < f.params.len() {
                b.at.push("%msg".into());
                continue;
            }
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
        b.owners.enter(false);
        if let Some(computed) = update {
            b.statements(&f.body.stmts)?;
            match computed {
                true => b.effects(f.body.value.as_deref()?)?,
                false => b.leave_fn(None),
            }
            // `update`'s message is its start variant, the tag `%m`, held once it is read.
            let msg = f.params.len().checked_sub(1).and_then(|n| b.locals[n]);
            if let Some(t) = msg.filter(|_| b.lines.contains("%msg")) {
                b.hold("%msg", t);
                b.lines.insert_str(0, "  store i64 %m, ptr %msg, align 8\n");
            }
            let empty = b.slots.is_empty() && b.lines.is_empty();
            return Some(match empty {
                true => String::new(),
                false => format!("entry:\n{}{}", b.slots, b.lines),
            });
        }
        // The block after the stack check.
        b.lines.insert_str(0, "body:\n");
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
        b.statements(&f.body.stmts)?;
        let e = f.body.value.as_deref();
        let v = match e {
            Some(e) => b.expr(e)?,
            None => None,
        };
        b.give(e, v);
        // Drops the label of the block nothing reaches that `give` opened.
        let end = b.lines.trim_end().rfind('\n').map_or(0, |i| i + 1);
        b.lines.truncate(end);
        let (name, params) = (&f.name, params.join(", "));
        let (check, inline) = b.stack_check(f.site)?;
        Some(format!(
            "\ndefine internal i32 @p_{name}({params}){inline} {{\nentry:\n{}{check}{}}}\n",
            b.slots, b.lines
        ))
    }

    /// The IR type of an `Int`, a function value, a `Bool`, a `Float`, or `ptr` for a value held in memory: a
    /// String, or a record or variant of what this lowering holds.
    fn ir(&self, t: Ty) -> Option<&'static str> {
        match self.core.types[t] {
            _ if !self.held[t] => None,
            // A function value is its id.
            Type::Int | Type::Fn(..) => Some("i64"),
            Type::Bool => Some("i1"),
            Type::Float => Some("double"),
            _ => Some("ptr"),
        }
    }

    /// The byte constant holding `text`, and its length.
    fn text(&mut self, text: String) -> (String, usize) {
        let (next, len) = (self.texts.len(), text.len());
        let n = *self.texts.entry(text).or_insert(next);
        (format!("@t{n}"), len)
    }

    /// The constant `at <file>:<line>:<col>` of `site`, and its length.
    fn place(&mut self, site: Site) -> (String, usize) {
        self.text(self.at(site))
    }

    fn at(&self, site: Site) -> String {
        let at = &self.core.sites[site];
        let ((file, _), line) = in_file(&self.core.files, |f| f.1, at.line);
        format!("at {file}:{line}:{}", at.col)
    }

    /// The constant naming a trap of `kind` at `site`, and its length.
    fn trap(&mut self, kind: &str, site: Site) -> (String, usize) {
        self.declare(TRAP);
        self.text(format!("{kind} {}", self.at(site)))
    }

    /// Lowers function `id` once.
    fn want(&mut self, id: usize) {
        if self.seen.insert(id) {
            self.wanted.push(id);
        }
    }

    /// The id of function value `f` of type `t`: its position among those made.
    fn fn_value(&mut self, f: &FnRef, t: Ty) -> usize {
        if let FnRef::Fn(id) = f {
            self.want(*id);
        }
        let known = self.fn_values.iter().position(|(g, _)| g == f);
        known.unwrap_or_else(|| {
            self.fn_values.push((f.clone(), t));
            self.fn_values.len() - 1
        })
    }

    /// `i32 @v<t>(i64 %id, args..., ptr %out)`: calls the function value `%id` of type `t`, a
    /// switch over every value of that type made, each a direct call or a variant built in place.
    fn dispatcher(&mut self, t: Ty, site: Site) -> Option<String> {
        let core = self.core;
        let Type::Fn(params, result) = &core.types[t] else {
            return None;
        };
        let cases: Vec<(usize, FnRef)> = (self.fn_values.iter().enumerate())
            .filter(|(_, (_, u))| *u == t)
            .map(|(id, (f, _))| (id, f.clone()))
            .collect();
        let mut b = self.body(site, None);
        let mut args: Vec<String> = (params.iter().enumerate())
            .map(|(n, &p)| format!("{} %a{n}", b.ty(p)))
            .collect();
        args.extend(result.map(|_| "ptr %out".to_string()));
        let labels: Vec<String> = cases.iter().map(|_| b.name("b")).collect();
        let arms: Vec<String> = (cases.iter().zip(&labels))
            .map(|((id, _), label)| format!("i64 {id}, label %{label}"))
            .collect();
        match labels.first() {
            // Every value of the type is among the cases, so the default is never taken.
            Some(first) => b.line(format!(
                "switch i64 %id, label %{first} [ {} ]",
                arms.join(" ")
            )),
            None => b.line("ret i32 0".into()),
        }
        for ((_, f), label) in cases.iter().zip(&labels) {
            b.start(label);
            match f {
                FnRef::Fn(id) => {
                    let name = &core.fns[*id].name;
                    let status = b.op(format!("call i32 @p_{name}({})", args.join(", ")));
                    b.line(format!("ret i32 {status}"));
                }
                FnRef::Variant(_, alt) => {
                    b.store("i64", &alt.to_string(), "%out");
                    let places = b.module.layouts.get((*result)?).alternatives[*alt].clone();
                    for (n, place) in places.iter().enumerate() {
                        b.fill("%out", place, &format!("%a{n}"));
                    }
                    b.line("ret i32 0".into());
                }
            }
        }
        let sig = [vec!["i64 %id".to_string()], args].concat().join(", ");
        Some(format!(
            "\ndefine internal i32 @v{t}({sig}) {{\nentry:\n{}{}}}\n",
            b.slots, b.lines
        ))
    }

    /// Declares the intrinsic or runtime entry `line` names.
    fn declare(&mut self, line: &str) {
        let name = line.split(['@', '(']).nth(1).expect("a declared name");
        let text = format!("{line}\n");
        self.intrinsics.entry(name.into()).or_insert(text);
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
    /// `Some(computed)` for `update`.
    update: Option<bool>,
    /// The function's site, where a `memory` trap of its own is reported.
    site: Site,
    /// The owners in scope, a temporary by its number in `temps`.
    owners: Owners<usize>,
    temps: Vec<String>,
}

/// An expression's operand; an expression with no value gives an empty one.
type Value = Option<String>;

/// The fields a match arm binds: each one's place, the value's address and the local it goes to.
type Binds = Vec<(layout::Place, String, Local)>;

/// A place step, its operands already evaluated.
enum Op {
    Field(usize),
    Item(String, Site),
    Slice(String, String, Site),
    /// A Map key; true when a missing entry is added.
    Key(String, Site, bool),
}

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
            "ptr" => self.bytes(name, self.size(t)),
            ty => self.slot(name, ty),
        }
    }

    /// A slot of `size` bytes.
    fn bytes(&mut self, name: &str, size: usize) {
        self.frame += size;
        let _ = writeln!(self.slots, "  {name} = alloca [{size} x i8], align 8");
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
            "ptr" => self.memmove(at, v, self.size(t)),
            ty => self.store(ty, v, at),
        }
    }

    fn memmove(&mut self, to: &str, from: &str, bytes: impl std::fmt::Display) {
        let name = "@llvm.memmove.p0.p0.i64";
        self.declare(&format!("declare void {name}(ptr, ptr, i64, i1)"));
        self.line(format!(
            "call void {name}(ptr {to}, ptr {from}, i64 {bytes}, i1 false)"
        ));
    }

    /// The address `offset` bytes into `base`; no `inbounds`, since it may be passed to a call.
    fn offset(&mut self, base: &str, offset: usize) -> String {
        match offset {
            0 => base.into(),
            _ => self.op(format!("getelementptr i8, ptr {base}, i64 {offset}")),
        }
    }

    /// The address of `place` and its type: its root and its steps' operands in written order,
    /// then the walk. A String item or slice is a new String.
    fn address(&mut self, place: &Place) -> Option<(String, Ty)> {
        let (root, t, ops) = self.operands(place)?;
        self.walk(root, t, ops)
    }

    /// The root of `place`, its type and the steps with their operands evaluated.
    fn operands(&mut self, place: &Place) -> Option<(String, Ty, Vec<Op>)> {
        let (root, t) = match &place.root {
            Root::Local(l) => (self.at[*l].clone(), self.locals[*l]?),
            Root::Value(e) => (self.expr(e)??, e.ty?),
        };
        let mut ops = vec![];
        for step in &place.steps {
            ops.push(match step {
                Step::Field(i) => Op::Field(*i),
                Step::Item(i, site) => Op::Item(self.expr(i)??, *site),
                Step::Slice(a, b, site) => Op::Slice(self.expr(a)??, self.expr(b)??, *site),
                Step::Key(k, site) => Op::Key(self.expr(k)??, *site, false),
            });
        }
        Some((root, t, ops))
    }

    /// Follows `ops` from the value at `root` of type `t`: the address and type reached.
    fn walk(&mut self, root: String, mut t: Ty, ops: Vec<Op>) -> Option<(String, Ty)> {
        let first = !ops.is_empty();
        let mut at = root.clone();
        for op in ops {
            let list = match self.module.core.types[t] {
                Type::List(item) => Some(item),
                _ => None,
            };
            at = match (op, list) {
                (Op::Key(key, site, add), _) => {
                    let at = self.entry(&at, t, &key, site, add);
                    t = self.map_types(t).1;
                    at
                }
                (Op::Field(i), _) => {
                    let field = self.module.layouts.get(t).alternatives[0][i];
                    t = field.ty;
                    self.field_at(&field, &at)
                }
                (Op::Item(i, site), Some(item)) => {
                    let at = self.item(&at, t, &i, site);
                    t = item;
                    at
                }
                (Op::Slice(a, b, site), Some(_)) => self.slice(&at, t, &a, &b, site),
                (Op::Item(i, site), None) => {
                    self.text(INDEX, Some(site), &[("ptr", &at), ("i64", &i)])
                }
                (Op::Slice(a, b, site), None) => {
                    let operands = [("ptr", at.as_str()), ("i64", &a), ("i64", &b)];
                    self.text(SLICE, Some(site), &operands)
                }
            };
        }
        self.module.ir(t)?;
        // A first field lies where its value does; its own name keeps it from passing for the
        // value's owner.
        if at == root && first {
            at = self.op(format!("getelementptr i8, ptr {at}, i64 0"));
        }
        Some((at, t))
    }

    /// The address of the field at `place` of the value at `at`, followed through its box.
    fn field_at(&mut self, place: &layout::Place, at: &str) -> String {
        let at = self.offset(at, place.offset);
        match place.boxed {
            true => self.load("ptr", &at),
            false => at,
        }
    }

    fn field(&mut self, place: &layout::Place, at: &str) -> String {
        let at = self.field_at(place, at);
        self.get(place.ty, &at)
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

    fn declare(&mut self, line: &str) {
        self.module.declare(line);
    }

    /// A new block of `bytes`; a `memory` trap when the heap refuses.
    fn alloc(&mut self, bytes: impl std::fmt::Display) -> String {
        self.declare(ALLOC);
        let block = self.op(format!("call ptr @paka_rt1_alloc(i64 {bytes})"));
        let refused = self.op(format!("icmp eq ptr {block}, null"));
        self.check(&refused, "memory", self.site);
        block
    }

    /// Returns a callee's nonzero status, its trap already recorded.
    fn pass(&mut self, status: &str) {
        let failed = self.op(format!("icmp ne i32 {status}, 0"));
        let (pass, ok) = (self.name("b"), self.name("b"));
        self.line(format!("br i1 {failed}, label %{pass}, label %{ok}"));
        self.start(&pass);
        self.line(format!("ret i32 {status}"));
        self.start(&ok);
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
             ret i32 1\n",
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

    fn stmt(&mut self, s: &Stmt) -> Option<()> {
        match s {
            Stmt::Let(l, e) => {
                let v = self.expr(e)??;
                self.bind_owned(*l, &v)?;
            }
            Stmt::Assign(place, op, e) => {
                let (root, t, mut ops) = self.operands(place)?;
                if let (None, Some(Op::Key(.., add))) = (op, ops.last_mut()) {
                    *add = true;
                }
                let mut v = self.expr(e)??;
                let (at, t) = self.walk(root, t, ops)?;
                if let Some(op) = op {
                    let old = self.get(t, &at);
                    v = match op {
                        Compound::Int(a, site) => self.int(a, *site, &old, &v),
                        Compound::Float(a) => self.float(a, &old, &v),
                        Compound::Join(site) => {
                            self.text(JOIN, Some(*site), &[("ptr", &old), ("ptr", &v)])
                        }
                    };
                }
                let v = self.owned(t, &v);
                self.drop_at(t, &at);
                self.put(t, &v, &at);
            }
            Stmt::Return(e) if self.update.is_none() => {
                let v = match e {
                    Some(e) => self.expr(e)?,
                    None => None,
                };
                self.give(e.as_ref(), v);
            }
            Stmt::Break | Stmt::Continue => {
                let (next, exit) = self.loops.last()?.clone();
                let to = if matches!(s, Stmt::Break) { exit } else { next };
                self.leave_loop();
                self.ended(format!("br label %{to}"));
            }
            Stmt::While(cond, body) => {
                let [head, inner, exit] = ["b"; 3].map(|s| self.name(s));
                self.line(format!("br label %{head}"));
                self.start(&head);
                let mark = self.owners.mark();
                let c = self.expr(cond)??;
                self.drop_temps(mark);
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
            Stmt::For(l, Walk::List(xs), body) => self.each(*l, xs, body)?,
            Stmt::Expr(e) => drop(self.expr(e)?),
            Stmt::Return(_) => return None,
        }
        Some(())
    }

    fn looped(&mut self, next: &str, exit: &str, body: &crate::core_form::Block) -> Option<()> {
        self.loops.push((next.into(), exit.into()));
        self.loop_body(body)?;
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
            ExprKind::Str(s) => self.literal(s),
            ExprKind::List(items) => self.list(e.ty?, items)?,
            ExprKind::Map(entries) => self.map(e.ty?, entries)?,
            ExprKind::Interp(parts, site) => self.interpolate(parts, *site)?,
            ExprKind::Place(place) => {
                let (at, t) = self.address(place)?;
                self.get(t, &at)
            }
            ExprKind::Record(_, fields) => self.construct(e.ty?, None, fields)?,
            ExprKind::Variant(tag, fields) => {
                self.construct(e.ty?, Some(self.module.alternative(tag)?), fields)?
            }
            ExprKind::Match(subject, arms) => return self.matched(e, subject, arms),
            ExprKind::FnValue(f) => self.module.fn_value(f, e.ty?).to_string(),
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
                    return self.held_binary(op, x, y).map(Some);
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
                // A function value is read before the arguments.
                let mut args = match callee {
                    Callee::Value(f) => vec![("i64", self.expr(f)??)],
                    _ => vec![],
                };
                // An `inout` place is reached once every argument is evaluated.
                let mut evaluated = vec![];
                for arg in given {
                    evaluated.push(match arg {
                        Arg::Value(a) => Ok((self.module.ir(a.ty?)?, self.expr(a)??, a.ty?)),
                        Arg::Inout(place) => Err(self.operands(place)?),
                    });
                }
                let mut tys = vec![];
                for arg in evaluated {
                    let (ty, v, t) = match arg {
                        Ok(value) => value,
                        Err((root, t, ops)) => {
                            let (at, t) = self.walk(root, t, ops)?;
                            ("ptr", at, t)
                        }
                    };
                    args.push((ty, v));
                    tys.push(t);
                }
                return match callee {
                    Callee::Fn(id) => self.call(*id, &args),
                    Callee::Builtin(row) => match ROWS.get(*row)?.1 {
                        row @ ("contains(xs: List<T>, item: T) -> Bool"
                        | "find(xs: List<T>, item: T) -> Option<Int>") => {
                            let Arg::Value(xs) = &given[0] else {
                                return None;
                            };
                            let i = self.position(xs.ty?, &args[0].1, &args[1].1);
                            Some(Some(match row.starts_with("find") {
                                true => self.found(e.ty?, &i),
                                false => self.op(format!("icmp sge i64 {i}, 0")),
                            }))
                        }
                        row => self.builtin(row, &args, &tys, e.ty, *site),
                    },
                    Callee::Value(f) => {
                        let t = f.ty?;
                        let Type::Fn(_, result) = &self.module.core.types[t] else {
                            return None;
                        };
                        self.module.dispatched.insert(t);
                        self.invoke(&format!("@v{t}"), *result, &args)
                    }
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
        let mark = self.owners.mark();
        let y = self.expr(y)??;
        self.drop_temps(mark);
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
        then: &crate::core_form::Block,
        otherwise: &Option<crate::core_form::Block>,
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
        branches: impl IntoIterator<Item = (String, Option<&'e crate::core_form::Block>, Binds)>,
    ) -> Option<Value> {
        let ty = match e.ty {
            Some(t) => Some(self.module.ir(t)?),
            None => None,
        };
        let merge = self.name("b");
        let mut incoming = vec![];
        for (label, block, binds) in branches {
            self.start(&label);
            let binds: Vec<_> = (binds.into_iter())
                .map(|(place, base, l)| (l, self.field(&place, &base)))
                .collect();
            let v = match block {
                Some(block) => self.scoped(&binds, block)?,
                None => None,
            };
            let v = v.unwrap_or_else(|| stand_in(ty.unwrap_or("i64")));
            incoming.push(format!("[ {v}, %{} ]", self.current()));
            self.line(format!("br label %{merge}"));
        }
        self.start(&merge);
        Some(ty.map(|ty| {
            let v = self.op(format!("phi {ty} {}", incoming.join(", ")));
            self.temp(e.ty.expect("a value has a type"), v)
        }))
    }

    /// A new record, or variant `alt`, of type `t` in a slot of its own; each field is stored as
    /// it is evaluated. One that owns is zeroed and owned first, so an exit while building drops
    /// what it holds so far.
    fn construct(&mut self, t: Ty, alt: Option<usize>, fields: &[Expr]) -> Option<String> {
        let value = self.name("%h");
        self.hold(&value, t);
        if self.module.layouts.owns(t) {
            let name = "@llvm.memset.p0.i64";
            self.declare(&format!("declare void {name}(ptr, i8, i64, i1)"));
            let size = self.size(t);
            self.line(format!(
                "call void {name}(ptr {value}, i8 0, i64 {size}, i1 false)"
            ));
            self.temp(t, value.clone());
        }
        if let Some(alt) = alt {
            self.store("i64", &alt.to_string(), &value);
        }
        let places = self.module.layouts.get(t).alternatives[alt.unwrap_or(0)].clone();
        for (field, place) in fields.iter().zip(places) {
            let v = self.expr(field)??;
            self.fill(&value, &place, &v);
        }
        Some(value)
    }

    /// Stores `v`, owned, as the field at `place` of the value at `value`.
    fn fill(&mut self, value: &str, place: &layout::Place, v: &str) {
        let v = self.owned(place.ty, v);
        self.place_into(value, place, &v);
    }

    /// Stores `v`, already owned, as the field at `place` of the value at `value`.
    fn place_into(&mut self, value: &str, place: &layout::Place, v: &str) {
        let at = self.offset(value, place.offset);
        if place.boxed {
            let block = self.alloc(self.size(place.ty));
            self.put(place.ty, v, &block);
            self.store("ptr", &block, &at);
        } else {
            self.put(place.ty, v, &at);
        }
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
                cases.push(format!(
                    "i64 {}, label %{label}",
                    self.module.alternative(tag)?
                ));
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
                let alt = self.module.alternative(tag)?;
                let places = self.module.layouts.get(t).alternatives[alt].clone();
                for (bind, place) in arm.binds.iter().zip(places) {
                    if let Some(l) = bind {
                        binds.push((place, at.clone(), *l));
                    }
                }
            }
            branches.push((label, Some(&arm.body), binds));
        }
        self.join(e, branches)
    }

    /// Calls a Paka function and passes a trap on.
    fn call(&mut self, id: usize, args: &[(&str, String)]) -> Option<Value> {
        let f = &self.module.core.fns[id];
        self.module.want(id);
        self.invoke(&format!("@p_{}", f.name), f.result, args)
    }

    /// Calls `callee`, whose result is of type `result`, and passes a trap on.
    fn invoke(
        &mut self,
        callee: &str,
        result: Option<Ty>,
        args: &[(&str, String)],
    ) -> Option<Value> {
        let mut list: Vec<String> = args.iter().map(|(ty, v)| format!("{ty} {v}")).collect();
        let out = match result {
            Some(t) => {
                self.module.ir(t)?;
                let r = self.name("%r");
                self.hold(&r, t);
                list.push(format!("ptr {r}"));
                Some((t, r))
            }
            None => None,
        };
        let status = self.op(format!("call i32 {callee}({})", list.join(", ")));
        self.pass(&status);
        Some(out.map(|(t, r)| {
            let v = self.get(t, &r);
            self.temp(t, v)
        }))
    }

    /// Builtin row `row` on `args`, of types `tys`, its result of type `result`.
    fn builtin(
        &mut self,
        row: &str,
        args: &[(&str, String)],
        tys: &[Ty],
        result: Option<Ty>,
        site: Site,
    ) -> Option<Value> {
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
            "len(x: String) -> Int" => self.chars(x[0]),
            "len(x: List<T>) -> Int" => self.length(x[0]),
            "len(x: Map<K, V>) -> Int" => self.map_len(x[0]),
            "is_empty(x: String) -> Bool"
            | "is_empty(x: List<T>) -> Bool"
            | "is_empty(x: Map<K, V>) -> Bool" => {
                let n = match row {
                    "is_empty(x: String) -> Bool" => self.chars(x[0]),
                    "is_empty(x: List<T>) -> Bool" => self.length(x[0]),
                    _ => self.map_len(x[0]),
                };
                self.op(format!("icmp eq i64 {n}, 0"))
            }
            _ => return self.generic(row, args, tys, result, site),
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
        self.declare(&format!("declare double {name}(double)"));
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
        self.declare(&format!("declare {{ i64, i1 }} {name}(i64, i64)"));
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

/// A literal leaves Paka here, into the IR.
#[allow(clippy::disallowed_methods)]
fn raw(n: Int) -> i64 {
    n.get()
}
