//! Evaluation of the core form, node by node (docs/core.md sections 2 to 6).

use super::builtin;
use super::value::{Entries, FnKey, Items, Key, Value, equals, number, position};
use crate::core_form::{
    App, Arg, Arith, Arm, Bin, Block, Callee, Cmp, Compound, Expr, ExprKind, FloatArith, FnId,
    FnRef, Local, Place, Program, Root, Site, Step, Stmt, Un, Walk,
};
use paka_rt::ops::{self, Float, Int, Text};
use std::cmp::Ordering;
use std::fmt::Write;
use std::mem;

/// The call depth no call may pass; the handler is at depth 1.
const DEPTH: usize = 10_000;
/// The most bytes in a String, and the most items or entries in a List or Map.
pub const CAP: usize = 16_777_216;
/// Frame names a trap keeps: the report shows eight, one more says there were others.
const KEPT: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Overflow,
    DivideByZero,
    OutOfBounds,
    BadConversion,
    Assert,
    Hole,
    Stack,
    Memory,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Overflow => "overflow",
            Kind::DivideByZero => "divide_by_zero",
            Kind::OutOfBounds => "out_of_bounds",
            Kind::BadConversion => "bad_conversion",
            Kind::Assert => "assert",
            Kind::Hole => "hole",
            Kind::Stack => "stack",
            Kind::Memory => "memory",
        }
    }
}

impl From<ops::Trap> for Kind {
    fn from(trap: ops::Trap) -> Kind {
        match trap {
            ops::Trap::Overflow => Kind::Overflow,
            ops::Trap::DivideByZero => Kind::DivideByZero,
            ops::Trap::OutOfBounds => Kind::OutOfBounds,
            ops::Trap::BadConversion => Kind::BadConversion,
        }
    }
}

/// A trap travelling out of the program: each frame it leaves adds its name.
#[derive(Debug)]
pub struct Trap {
    pub kind: Kind,
    pub site: Site,
    pub detail: String,
    /// Innermost first.
    pub frames: Vec<String>,
}

impl Trap {
    pub fn new(kind: impl Into<Kind>, site: Site, detail: String) -> Trap {
        let kind = kind.into();
        Trap {
            kind,
            site,
            detail,
            frames: vec![],
        }
    }

    pub fn bare(kind: Kind, site: Site) -> Trap {
        Trap::new(kind, site, String::new())
    }

    fn leave(&mut self, name: &str) {
        if self.frames.len() < KEPT {
            self.frames.push(name.into());
        }
    }
}

/// Why an evaluation did not give its value.
pub enum Stop {
    Trap(Box<Trap>),
    Return(Value),
    Break,
    Continue,
}

impl From<Trap> for Stop {
    fn from(trap: Trap) -> Stop {
        Stop::Trap(Box::new(trap))
    }
}

impl Stop {
    fn into_trap(self) -> Trap {
        match self {
            Stop::Trap(trap) => *trap,
            _ => unreachable!("a trap leaves every function"),
        }
    }
}

type Done<T = Value> = Result<T, Stop>;

/// The locals of one call.
pub struct Frame {
    locals: Vec<Value>,
    params: usize,
    /// The function's name in its definition, where a copy that nests too deep is reported.
    site: Site,
}

/// A step of a place with its operands evaluated.
enum Idx {
    Field(usize),
    Item(Int, Site),
    Key(Key, Site),
    Slice(Int, Int, Site),
}

/// An argument evaluated, or a place the call takes over for its length: the value moves into
/// the callee and back, so a read-only list is not copied and an `inout` is written in place.
enum Given {
    Own(Value),
    Borrow(Local, Vec<Idx>),
}

/// A borrowed place: the argument's position, its local and its path.
type Loan = (usize, Local, Vec<Idx>);

enum Target {
    User(FnId),
    Ctor(usize),
    Builtin(usize),
}

pub struct Machine<'a> {
    core: &'a Program,
    depth: usize,
}

impl<'a> Machine<'a> {
    pub fn new(core: &'a Program) -> Machine<'a> {
        Machine { core, depth: 0 }
    }

    /// The state built once, then the handler's answer to the start message.
    pub fn handle(&mut self, app: &App) -> Result<Value, Trap> {
        let site = self.core.fns[app.update].site;
        let mut fr = Frame {
            locals: vec![],
            params: 0,
            site,
        };
        let mut args = vec![];
        if let Some(state) = &app.state {
            args.push(self.eval(&mut fr, state).map_err(Stop::into_trap)?);
        }
        args.push(Value::Variant(app.start, Items::default()));
        let (answer, _) = self
            .invoke(app.update, args, site)
            .map_err(Stop::into_trap)?;
        Ok(answer)
    }

    /// Runs function `id` on its parameters; gives its value and its final locals, whose first
    /// ones are the parameters.
    fn invoke(
        &mut self,
        id: FnId,
        mut locals: Vec<Value>,
        site: Site,
    ) -> Done<(Value, Vec<Value>)> {
        if self.depth >= DEPTH {
            return Err(Trap::bare(Kind::Stack, site).into());
        }
        let f = &self.core.fns[id];
        locals.resize_with(f.locals.len(), Value::default);
        let mut fr = Frame {
            locals,
            params: f.params.len(),
            site: f.site,
        };
        self.depth += 1;
        let done = self.body(&mut fr, &f.body);
        self.depth -= 1;
        match done {
            Ok(value) | Err(Stop::Return(value)) => Ok((value, fr.locals)),
            Err(Stop::Trap(mut trap)) => {
                trap.leave(&f.name);
                Err(Stop::Trap(trap))
            }
            Err(_) => unreachable!("a loop holds its break"),
        }
    }

    /// A function's block: a local it ends in moves out instead of being copied.
    fn body(&mut self, fr: &mut Frame, b: &Block) -> Done {
        for s in &b.stmts {
            self.stmt(fr, s)?;
        }
        match &b.value {
            Some(e) => self.result(fr, e),
            None => Ok(Value::Unit),
        }
    }

    fn block(&mut self, fr: &mut Frame, b: &Block) -> Done {
        for s in &b.stmts {
            self.stmt(fr, s)?;
        }
        match &b.value {
            Some(e) => self.eval(fr, e),
            None => Ok(Value::Unit),
        }
    }

    /// The value a function leaves with: a local that is not a parameter moves.
    fn result(&mut self, fr: &mut Frame, e: &Expr) -> Done {
        match &e.kind {
            ExprKind::Place(Place {
                root: Root::Local(l),
                steps,
            }) if steps.is_empty() && *l >= fr.params => Ok(mem::take(&mut fr.locals[*l])),
            _ => self.eval(fr, e),
        }
    }

    fn stmt(&mut self, fr: &mut Frame, s: &Stmt) -> Done<()> {
        match s {
            Stmt::Let(l, e) => fr.locals[*l] = self.eval(fr, e)?,
            Stmt::Assign(place, op, e) => self.assign(fr, place, op.as_ref(), e)?,
            Stmt::Return(e) => {
                let value = match e {
                    Some(e) => self.result(fr, e)?,
                    None => Value::Unit,
                };
                return Err(Stop::Return(value));
            }
            Stmt::Break => return Err(Stop::Break),
            Stmt::Continue => return Err(Stop::Continue),
            Stmt::For(x, walk, body) => self.each(fr, *x, walk, body)?,
            Stmt::While(c, body) => while self.truth(fr, c)? && !self.run_body(fr, body)? {},
            Stmt::Expr(e) => drop(self.eval(fr, e)?),
        }
        Ok(())
    }

    fn each(&mut self, fr: &mut Frame, x: Local, walk: &Walk, body: &Block) -> Done<()> {
        match walk {
            Walk::List(e) => {
                let Value::List(mut items) = self.eval(fr, e)? else {
                    unreachable!("a checked List");
                };
                for item in mem::take(&mut items.0) {
                    fr.locals[x] = item;
                    if self.run_body(fr, body)? {
                        break;
                    }
                }
            }
            Walk::Range(a, b) => {
                let (mut i, end) = (self.eval(fr, a)?.int(), self.eval(fr, b)?.int());
                while i < end {
                    fr.locals[x] = Value::Int(i);
                    if self.run_body(fr, body)? {
                        break;
                    }
                    i = i.wrap_add(Int::new(1));
                }
            }
        }
        Ok(())
    }

    /// Runs a loop body once; whether the loop is over.
    fn run_body(&mut self, fr: &mut Frame, body: &Block) -> Done<bool> {
        match self.block(fr, body) {
            Ok(_) | Err(Stop::Continue) => Ok(false),
            Err(Stop::Break) => Ok(true),
            Err(stop) => Err(stop),
        }
    }

    fn truth(&mut self, fr: &mut Frame, e: &Expr) -> Done<bool> {
        Ok(self.eval(fr, e)?.bool())
    }

    pub fn eval(&mut self, fr: &mut Frame, e: &Expr) -> Done {
        Ok(match &e.kind {
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Float(x) => Value::Float(*x),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Str(s) => Value::Str(Text::from(s.as_str())),
            ExprKind::Interp(parts, site) => self.interpolate(fr, parts, *site)?,
            ExprKind::Hole(site) => return Err(self.hole(*site).into()),
            ExprKind::Place(p) => self.read(fr, p)?,
            ExprKind::Record(_, fields) => Value::Record(Items(self.all(fr, fields)?)),
            ExprKind::List(items) => Value::List(Items(self.all(fr, items)?)),
            ExprKind::Map(pairs) => self.map(fr, pairs)?,
            ExprKind::Variant(tag, fields) => {
                Value::Variant(number(tag), Items(self.all(fr, fields)?))
            }
            ExprKind::FnValue(FnRef::Fn(id)) => Value::Fn(FnKey::Fn(*id)),
            ExprKind::FnValue(FnRef::Variant(e, v)) => Value::Fn(FnKey::Ctor(*e, *v)),
            ExprKind::Unary(op, x) => self.unary(fr, op, x)?,
            ExprKind::Binary(op, a, b) => self.binary(fr, op, a, b)?,
            ExprKind::Call(callee, args, site) => self.call(fr, callee, args, *site)?,
            ExprKind::If(c, then, other) => match (self.truth(fr, c)?, other) {
                (true, _) => self.block(fr, then)?,
                (false, Some(other)) => self.block(fr, other)?,
                (false, None) => Value::Unit,
            },
            ExprKind::Match(subject, arms) => self.matching(fr, subject, arms)?,
        })
    }

    fn all(&mut self, fr: &mut Frame, es: &[Expr]) -> Done<Vec<Value>> {
        let mut values = Vec::with_capacity(es.len());
        for e in es {
            values.push(self.eval(fr, e)?);
        }
        Ok(values)
    }

    fn map(&mut self, fr: &mut Frame, pairs: &[(Expr, Expr)]) -> Done {
        let mut entries = Entries::default();
        for (k, v) in pairs {
            let key = self.eval(fr, k)?.key();
            entries.0.insert(key, self.eval(fr, v)?);
        }
        Ok(Value::Map(entries))
    }

    fn hole(&self, site: Site) -> Trap {
        let name = self.core.sites[site].hole.as_deref().unwrap_or_default();
        Trap::new(Kind::Hole, site, format!("??{name}"))
    }

    fn interpolate(&mut self, fr: &mut Frame, parts: &[Expr], site: Site) -> Done {
        let mut text = String::new();
        for part in parts {
            let shown = match self.eval(fr, part)? {
                Value::Int(n) => write!(text, "{n}"),
                Value::Bool(b) => write!(text, "{b}"),
                Value::Str(s) => {
                    text.push_str(s.as_str());
                    Ok(())
                }
                _ => unreachable!("a checked part"),
            };
            shown.expect("writing to a String");
            if text.len() > CAP {
                return Err(Trap::bare(Kind::Memory, site).into());
            }
        }
        Ok(Value::Str(text.into()))
    }

    fn unary(&mut self, fr: &mut Frame, op: &Un, x: &Expr) -> Done {
        let x = self.eval(fr, x)?;
        Ok(match op {
            Un::Neg(site) => {
                let n = x.int();
                let neg = n.neg();
                Value::Int(neg.map_err(|k| Trap::new(k, *site, format!("-({n})")))?)
            }
            Un::NegFloat => Value::Float(x.float().neg()),
            Un::Not => Value::Bool(!x.bool()),
        })
    }

    fn binary(&mut self, fr: &mut Frame, op: &Bin, a: &Expr, b: &Expr) -> Done {
        match op {
            Bin::And => return Ok(Value::Bool(self.truth(fr, a)? && self.truth(fr, b)?)),
            Bin::Or => return Ok(Value::Bool(self.truth(fr, a)? || self.truth(fr, b)?)),
            _ => {}
        }
        let (x, y) = (self.eval(fr, a)?, self.eval(fr, b)?);
        Ok(match op {
            Bin::Int(arith, site) => Value::Int(int_op(arith, *site, x.int(), y.int())?),
            Bin::Float(arith) => Value::Float(float_op(arith, x.float(), y.float())),
            Bin::Join(site) => {
                let (Value::Str(mut s), Value::Str(t)) = (x, y) else {
                    unreachable!("checked Strings");
                };
                append(*site, &mut s, &t)?;
                Value::Str(s)
            }
            Bin::Eq(site) | Bin::Ne(site) => {
                let same = equals(&x, &y).ok_or_else(|| Trap::bare(Kind::Stack, *site))?;
                Value::Bool(same == matches!(op, Bin::Eq(_)))
            }
            Bin::Order(cmp) => Value::Bool(compare(cmp, &x, &y)),
            Bin::And | Bin::Or => unreachable!("decided above"),
        })
    }

    fn matching(&mut self, fr: &mut Frame, subject: &Expr, arms: &[Arm]) -> Done {
        let picked = match &subject.kind {
            ExprKind::Place(p) => self.peek(fr, p, |v| pick(arms, v))?,
            _ => pick(arms, &self.eval(fr, subject)?),
        };
        let (arm, binds) = picked.ok_or_else(|| Trap::bare(Kind::Stack, fr.site))?;
        for (l, v) in binds {
            fr.locals[l] = v;
        }
        self.block(fr, &arm.body)
    }

    /// The operands of a place's steps, in written order.
    fn path(&mut self, fr: &mut Frame, steps: &[Step]) -> Done<Vec<Idx>> {
        let mut path = Vec::with_capacity(steps.len());
        for step in steps {
            path.push(match step {
                Step::Field(n) => Idx::Field(*n),
                Step::Item(i, site) => Idx::Item(self.eval(fr, i)?.int(), *site),
                Step::Key(k, site) => Idx::Key(self.eval(fr, k)?.key(), *site),
                Step::Slice(a, b, site) => {
                    let (a, b) = (self.eval(fr, a)?.int(), self.eval(fr, b)?.int());
                    Idx::Slice(a, b, *site)
                }
            });
        }
        Ok(path)
    }

    /// Runs `f` on the value at `p`, without copying it.
    fn peek<R>(&mut self, fr: &mut Frame, p: &Place, f: impl FnOnce(&Value) -> R) -> Done<R> {
        let computed = match &p.root {
            Root::Value(e) => Some(self.eval(fr, e)?),
            Root::Local(_) => None,
        };
        let path = self.path(fr, &p.steps)?;
        let root = match (&p.root, &computed) {
            (Root::Local(l), _) => &fr.locals[*l],
            (_, Some(v)) => v,
            _ => unreachable!("a root"),
        };
        Ok(walk(root, &path, fr.site, f)?)
    }

    fn read(&mut self, fr: &mut Frame, p: &Place) -> Done {
        let copy = self.peek(fr, p, Value::copy)?;
        Ok(copy.ok_or_else(|| Trap::bare(Kind::Stack, fr.site))?)
    }

    fn assign(
        &mut self,
        fr: &mut Frame,
        place: &Place,
        op: Option<&Compound>,
        e: &Expr,
    ) -> Done<()> {
        let Root::Local(l) = &place.root else {
            unreachable!("a written place is rooted at a local");
        };
        let path = self.path(fr, &place.steps)?;
        let value = self.eval(fr, e)?;
        let root = &mut fr.locals[*l];
        match op {
            None => store(root, path, value, CAP)?,
            Some(op) => compound(walk_mut(root, &path)?, op, value)?,
        }
        Ok(())
    }

    fn call(&mut self, fr: &mut Frame, callee: &Callee, args: &[Arg], site: Site) -> Done {
        let target = match callee {
            Callee::Fn(id) => Target::User(*id),
            Callee::Builtin(row) => Target::Builtin(*row),
            Callee::Value(e) => match self.eval(fr, e)? {
                Value::Fn(FnKey::Fn(id)) => Target::User(id),
                Value::Fn(FnKey::Ctor(_, v)) => Target::Ctor(v),
                _ => unreachable!("a checked function value"),
            },
        };
        let lend = !matches!(target, Target::Ctor(_));
        let mut given: Vec<Given> = Vec::with_capacity(args.len());
        for arg in args {
            let g = self.given(fr, arg, lend, &given)?;
            given.push(g);
        }
        let (mut params, loans) = take_args(fr, given)?;
        let value = match target {
            Target::Ctor(v) => return Ok(Value::Variant(v, Items(params))),
            Target::Builtin(row) => builtin::call(row, &mut params, site)?,
            Target::User(id) => {
                let (value, locals) = self.invoke(id, params, site)?;
                params = locals;
                value
            }
        };
        give_back(fr, loans, &mut params);
        Ok(value)
    }

    fn given(&mut self, fr: &mut Frame, arg: &Arg, lend: bool, earlier: &[Given]) -> Done<Given> {
        match arg {
            Arg::Inout(p) => {
                let Root::Local(l) = &p.root else {
                    unreachable!("an inout place is rooted at a local");
                };
                Ok(Given::Borrow(*l, self.path(fr, &p.steps)?))
            }
            Arg::Value(e) => {
                if let ExprKind::Place(Place {
                    root: Root::Local(l),
                    steps,
                }) = &e.kind
                {
                    let taken = |g: &Given| matches!(g, Given::Borrow(m, _) if m == l);
                    let heavy = !matches!(
                        fr.locals[*l],
                        Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Fn(_)
                    );
                    if lend && steps.is_empty() && heavy && !earlier.iter().any(taken) {
                        return Ok(Given::Borrow(*l, vec![]));
                    }
                }
                Ok(Given::Own(self.eval(fr, e)?))
            }
        }
    }
}

/// Walks the `inout` places in order, with their checks, and moves what they hold into the
/// parameters.
fn take_args(fr: &mut Frame, given: Vec<Given>) -> Result<(Vec<Value>, Vec<Loan>), Trap> {
    let mut params = Vec::with_capacity(given.len());
    let mut loans = vec![];
    for (i, g) in given.into_iter().enumerate() {
        params.push(match g {
            Given::Own(v) => v,
            Given::Borrow(l, path) => {
                let v = mem::take(walk_mut(&mut fr.locals[l], &path)?);
                loans.push((i, l, path));
                v
            }
        });
    }
    Ok((params, loans))
}

fn give_back(fr: &mut Frame, loans: Vec<Loan>, params: &mut [Value]) {
    for (i, l, path) in loans {
        let slot = walk_mut(&mut fr.locals[l], &path).expect("the place was walked");
        *slot = mem::take(&mut params[i]);
    }
}

/// The arm for the subject's variant and the fields it binds, copied.
fn pick<'p>(arms: &'p [Arm], subject: &Value) -> Option<(&'p Arm, Vec<(Local, Value)>)> {
    let Value::Variant(n, fields) = subject else {
        unreachable!("a checked subject");
    };
    let arm = (arms.iter().find(|a| a.tags.iter().any(|t| number(t) == *n)))
        .expect("an arm for every variant");
    let mut binds = vec![];
    for (bound, field) in arm.binds.iter().zip(&fields.0) {
        if let Some(l) = bound {
            binds.push((*l, field.copy()?));
        }
    }
    Some((arm, binds))
}

fn out_of_bounds(site: Site, detail: String) -> Trap {
    Trap::new(Kind::OutOfBounds, site, detail)
}

fn bound(i: Int, len: usize, site: Site) -> Result<usize, Trap> {
    position(i)
        .filter(|n| *n < len)
        .ok_or_else(|| out_of_bounds(site, format!("index {i}, length {len}")))
}

fn missing(key: &Key, site: Site) -> Trap {
    out_of_bounds(site, format!("key {}", key.show()))
}

/// Runs `f` on the value at the end of `path`; a slice or a String item is a fresh value.
fn walk<R>(v: &Value, path: &[Idx], home: Site, f: impl FnOnce(&Value) -> R) -> Result<R, Trap> {
    let Some((step, rest)) = path.split_first() else {
        return Ok(f(v));
    };
    match (step, v) {
        (Idx::Field(n), Value::Record(fields)) => walk(&fields.0[*n], rest, home, f),
        (Idx::Item(i, site), Value::List(items)) => {
            walk(&items.0[bound(*i, items.0.len(), *site)?], rest, home, f)
        }
        (Idx::Item(i, site), Value::Str(t)) => {
            let item = t.index(*i);
            let item =
                item.map_err(|k| Trap::new(k, *site, format!("index {i}, length {}", t.len())))?;
            walk(&Value::Str(item), rest, home, f)
        }
        (Idx::Key(k, site), Value::Map(m)) => {
            walk(m.0.get(k).ok_or_else(|| missing(k, *site))?, rest, home, f)
        }
        (Idx::Slice(a, b, site), Value::List(items)) => {
            let len = items.0.len();
            let (from, to) = match (position(*a), position(*b)) {
                (Some(x), Some(y)) if x <= y && y <= len => (x, y),
                _ => return Err(slice_error(*a, *b, len, *site)),
            };
            let copies = items.0[from..to].iter().map(Value::copy);
            let part = copies.collect::<Option<_>>();
            let part = part.ok_or_else(|| Trap::bare(Kind::Stack, home))?;
            walk(&Value::List(Items(part)), rest, home, f)
        }
        (Idx::Slice(a, b, site), Value::Str(t)) => {
            let part = t
                .slice(*a, *b)
                .map_err(|_| slice_error(*a, *b, t.len(), *site))?;
            walk(&Value::Str(part), rest, home, f)
        }
        _ => unreachable!("a checked place"),
    }
}

fn slice_error(a: Int, b: Int, len: impl std::fmt::Display, site: Site) -> Trap {
    out_of_bounds(site, format!("slice {a}..<{b}, length {len}"))
}

/// The value at the end of `path`, which must exist.
fn walk_mut<'v>(root: &'v mut Value, path: &[Idx]) -> Result<&'v mut Value, Trap> {
    path.iter().try_fold(root, |v, step| match (step, v) {
        (Idx::Field(n), Value::Record(fields)) => Ok(&mut fields.0[*n]),
        (Idx::Item(i, site), Value::List(items)) => {
            let n = bound(*i, items.0.len(), *site)?;
            Ok(&mut items.0[n])
        }
        (Idx::Key(k, site), Value::Map(m)) => m.0.get_mut(k).ok_or_else(|| missing(k, *site)),
        _ => unreachable!("a checked place"),
    })
}

/// Writes `value` at `path`; a last key step adds a missing key, unless the map holds `cap`.
fn store(root: &mut Value, mut path: Vec<Idx>, value: Value, cap: usize) -> Result<(), Trap> {
    if !matches!(path.last(), Some(Idx::Key(..))) {
        *walk_mut(root, &path)? = value;
        return Ok(());
    }
    let Some(Idx::Key(key, site)) = path.pop() else {
        unreachable!("a key step");
    };
    let Value::Map(map) = walk_mut(root, &path)? else {
        unreachable!("a checked Map");
    };
    if map.0.len() >= cap && !map.0.contains_key(&key) {
        return Err(Trap::bare(Kind::Memory, site));
    }
    map.0.insert(key, value);
    Ok(())
}

/// `slot op= value`, in place.
fn compound(slot: &mut Value, op: &Compound, value: Value) -> Result<(), Trap> {
    match op {
        Compound::Int(arith, site) => {
            *slot = Value::Int(int_op(arith, *site, slot.int(), value.int())?)
        }
        Compound::Float(arith) => {
            *slot = Value::Float(float_op(arith, slot.float(), value.float()))
        }
        Compound::Join(site) => {
            let (Value::Str(s), Value::Str(t)) = (slot, &value) else {
                unreachable!("checked Strings");
            };
            append(*site, s, t)?;
        }
    }
    Ok(())
}

fn append(site: Site, s: &mut Text, t: &Text) -> Result<(), Trap> {
    if s.as_str().len() + t.as_str().len() > CAP {
        return Err(Trap::bare(Kind::Memory, site));
    }
    s.push(t);
    Ok(())
}

fn int_op(op: &Arith, site: Site, a: Int, b: Int) -> Result<Int, Trap> {
    let (done, sign) = match op {
        Arith::Add => (a.add(b), '+'),
        Arith::Sub => (a.sub(b), '-'),
        Arith::Mul => (a.mul(b), '*'),
        Arith::Div => (a.div(b), '/'),
        Arith::Rem => (a.rem(b), '%'),
    };
    done.map_err(|k| Trap::new(k, site, format!("{a} {sign} {b}")))
}

fn float_op(op: &FloatArith, a: Float, b: Float) -> Float {
    match op {
        FloatArith::Add => a.add(b),
        FloatArith::Sub => a.sub(b),
        FloatArith::Mul => a.mul(b),
        FloatArith::Div => a.div(b),
    }
}

fn compare(cmp: &Cmp, a: &Value, b: &Value) -> bool {
    let order = match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Str(x), Value::Str(y)) => x.partial_cmp(y),
        _ => unreachable!("a checked order"),
    };
    match cmp {
        Cmp::Lt => order == Some(Ordering::Less),
        Cmp::Le => matches!(order, Some(Ordering::Less | Ordering::Equal)),
        Cmp::Gt => order == Some(Ordering::Greater),
        Cmp::Ge => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn a_new_key_past_the_cap_traps_memory() {
        let key = |n| Key::Int(Int::new(n));
        let entries = BTreeMap::from([(key(1), Value::Unit), (key(2), Value::Unit)]);
        let mut map = Value::Map(Entries(entries));
        let at = |n| vec![Idx::Key(key(n), 0)];
        assert!(store(&mut map, at(1), Value::Bool(true), 2).is_ok());
        let trap = store(&mut map, at(3), Value::Unit, 2).unwrap_err();
        assert_eq!(trap.kind, Kind::Memory);
        assert!(store(&mut map, at(3), Value::Unit, 3).is_ok());
    }
}
