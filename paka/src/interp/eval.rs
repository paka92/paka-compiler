//! Evaluation of the core form, node by node (docs/core.md sections 2 to 6).

use super::show::show;
use super::value::{Entries, FnKey, Items, Key, Message, Value, equals, number, position};
use super::{RUN_THREAD, STACK, builtin, generic};
use crate::builtins::{ELEMENTS, ROWS};
use crate::core_form::Fn as Func;
use crate::core_form::{
    App, Arg, Arith, Arm, Bin, Block, Callee, Child, Cmp, Compound, Expr, ExprKind, FloatArith,
    FnId, FnRef, Local, Place, Program, Root, Site, Step, Stmt, Ty, Un, Walk,
};
pub use overlap::Detector;
use overlap::{held_in_register, spans};
use paka_rt::ops::{self, Float, Int, Text};
use paka_rt::route::Event;
use paka_rt::tree::{self, Edit, Node};
use std::cmp::Ordering;
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};
use std::{mem, panic, thread};
pub use ui::{Left, Ui};

mod overlap;
mod ui;

/// The call depth no call may pass; the handler is at depth 1.
const DEPTH: usize = 10_000;
/// The most expressions and blocks under evaluation on one stack segment. A level takes under
/// 5 KiB in a debug build, so a segment of `STACK` bytes holds it with a wide margin.
const BUDGET: usize = 1_000;
pub use paka_rt::ops::CAP;
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
            ops::Trap::Memory => Kind::Memory,
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
    /// A fault of the interpreter itself, with its reason; the run ends with status 4.
    Internal(String),
    Return(Value),
    Break,
    Continue,
    /// A UI example's app stopped with this line: an effect it does not grant, or start fields
    /// with no arguments to fill them.
    Ended(String),
    /// The interrupt was set: the run is past its time limit (docs/decisions/M4-04.md).
    Time,
}

impl From<Trap> for Stop {
    fn from(trap: Trap) -> Stop {
        Stop::Trap(Box::new(trap))
    }
}

pub type Done<T = Value> = Result<T, Stop>;

/// Both sides of a failed comparison and their type.
pub type Sides = (Value, Value, Ty);

/// Why an example is false: its `&&` operand, counted from 0, and for a comparison its sides.
pub struct Fail {
    pub claim: usize,
    pub sides: Option<Sides>,
}

impl Fail {
    /// A `return` ended the example: the first claim stands for it.
    fn unplaced() -> Fail {
        Fail {
            claim: 0,
            sides: None,
        }
    }
}

/// The operands of a `&&` chain in written order.
fn operands<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match &e.kind {
        ExprKind::Binary(Bin::And, a, b) => {
            operands(a, out);
            operands(b, out);
        }
        _ => out.push(e),
    }
}

/// The locals of one call.
pub struct Frame {
    locals: Vec<Value>,
    params: usize,
    /// The function's name in its definition, where a copy that nests too deep is reported.
    site: Site,
}

/// What a place holds: a value in storage, or a fresh one made by reading it (a slice, a String
/// item), which a caller that wants a copy can keep as it is.
enum Seen<'v> {
    Held(&'v Value),
    Made(Value),
}

impl Seen<'_> {
    fn value(&self) -> &Value {
        match self {
            Seen::Held(v) => v,
            Seen::Made(v) => v,
        }
    }

    /// An owned copy, or `None` when it nests too deep.
    fn copy(self) -> Option<Value> {
        match self {
            Seen::Held(v) => v.copy(),
            Seen::Made(v) => Some(v),
        }
    }
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
    /// A row of `builtins::UI`.
    Ui(usize),
}

/// The children of one child block, in the order their lines ran.
type Kids = Vec<(tree::Step, Node<Message>)>;

pub struct Machine<'a> {
    core: &'a Program,
    depth: usize,
    levels: usize,
    /// The child lists being built, innermost last.
    kids: Vec<Kids>,
    /// Of a UI example: the first frame that held an error node, as the call that drew it and
    /// the node's line.
    pub broken: Option<(Site, String)>,
    /// Of a UI example, when kept: each action's events from its start and what it left.
    pub ui_log: Option<Vec<(Vec<Event>, Left)>>,
    /// The run-time overlap detector, when the run asks for it.
    pub detector: Option<Detector>,
    /// Polled at each call and loop turn, when the run has a time limit.
    pub interrupt: Option<Arc<AtomicBool>>,
}

impl<'a> Machine<'a> {
    pub fn new(core: &'a Program) -> Machine<'a> {
        Machine {
            core,
            depth: 0,
            levels: 0,
            kids: vec![],
            broken: None,
            ui_log: None,
            detector: None,
            interrupt: None,
        }
    }

    /// The tree view `id` draws from `args`.
    pub fn draw(&mut self, id: FnId, args: Vec<Value>) -> Done<Node<Message>> {
        let site = self.core.fns[id].site;
        self.depth = 0;
        match self.invoke(id, args, site)?.0 {
            Value::Node(node) => Ok(*node),
            _ => unreachable!("a view gives a node"),
        }
    }

    /// What function `id` gives for `args`, called at depth 1 like the handler.
    pub fn run(&mut self, id: FnId, args: Vec<Value>) -> Done {
        let site = self.core.fns[id].site;
        Ok(self.invoke(id, args, site)?.0)
    }

    /// The state, built once from the defaults the core form writes in; `None` without a `state:`.
    pub fn state(&mut self, app: &App) -> Done<Option<Value>> {
        let Some(state) = &app.state else {
            return Ok(None);
        };
        let site = self.core.fns[app.update].site;
        let mut fr = Frame {
            locals: vec![],
            params: 0,
            site,
        };
        self.eval(&mut fr, state).map(Some)
    }

    /// The handler's effects for one message, and the state it leaves; it ends in a trap or an
    /// internal fault.
    pub fn update(
        &mut self,
        app: &App,
        state: Option<Value>,
        message: Value,
    ) -> Done<(Value, Option<Value>)> {
        let site = self.core.fns[app.update].site;
        let Some(state) = state else {
            let (effects, _) = self.invoke(app.update, vec![message], site)?;
            return Ok((effects, None));
        };
        let (effects, locals) = self.invoke(app.update, vec![state, message], site)?;
        Ok((effects, locals.into_iter().next()))
    }

    /// The message an answer's function makes of its text, called at depth 1 like the handler.
    pub fn answer(&mut self, f: FnKey, arg: Value) -> Done {
        match f {
            FnKey::Ctor(_, variant) => Ok(Value::Variant(variant, Items(vec![arg]))),
            FnKey::Fn(id) => self.run(id, vec![arg]),
        }
    }

    /// Runs an example at depth 1: `None` when its Bool is true, else why it is false.
    pub fn example(&mut self, f: &Func) -> Done<Option<Fail>> {
        let mut fr = Frame {
            locals: Vec::new(),
            params: 0,
            site: f.site,
        };
        fr.locals.resize_with(f.locals.len(), Value::default);
        self.depth = 1;
        match self.verdict(&mut fr, &f.body) {
            Err(Stop::Return(held)) => Ok((!held.bool()).then(Fail::unplaced)),
            Err(Stop::Trap(mut trap)) => {
                trap.leave(&format!("example {}", f.name));
                Err(Stop::Trap(trap))
            }
            done => done,
        }
    }

    /// The statements, then the operands of the final `&&` chain until one is false.
    fn verdict(&mut self, fr: &mut Frame, b: &Block) -> Done<Option<Fail>> {
        for s in &b.stmts {
            self.stmt(fr, s)?;
        }
        let Some(last) = &b.value else {
            unreachable!("an example ends in a value");
        };
        let mut claims = vec![];
        operands(last, &mut claims);
        for (claim, e) in claims.into_iter().enumerate() {
            let (held, sides) = self.claim(fr, e)?;
            if !held {
                return Ok(Some(Fail { claim, sides }));
            }
        }
        Ok(None)
    }

    /// Whether the operand holds; for a comparison, also its two sides and their type.
    fn claim(&mut self, fr: &mut Frame, e: &Expr) -> Done<(bool, Option<Sides>)> {
        let ExprKind::Binary(op @ (Bin::Eq(_) | Bin::Ne(_) | Bin::Order(_)), a, b) = &e.kind else {
            return Ok((self.truth(fr, e)?, None));
        };
        let (x, y) = (self.eval(fr, a)?, self.eval(fr, b)?);
        let held = match op {
            Bin::Eq(site) | Bin::Ne(site) => {
                let same = equals(&x, &y).ok_or_else(|| Trap::bare(Kind::Stack, *site))?;
                same == matches!(op, Bin::Eq(_))
            }
            Bin::Order(cmp) => compare(cmp, &x, &y),
            _ => unreachable!("a comparison"),
        };
        let ty = a.ty.expect("a typed operand");
        Ok((held, Some((x, y, ty))))
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
        self.poll()?;
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
            Err(stop @ (Stop::Internal(_) | Stop::Time)) => Err(stop),
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
        self.deeper(|m| m.statements(fr, b))
    }

    /// Runs `f` as one more level of evaluation. Past `BUDGET` levels on this stack segment, `f`
    /// goes on a fresh segment: a scoped thread with a stack of its own, joined at once.
    fn deeper(&mut self, f: impl FnOnce(&mut Self) -> Done + Send) -> Done {
        if self.levels < BUDGET {
            self.levels += 1;
            let done = f(self);
            self.levels -= 1;
            return done;
        }
        let here = mem::take(&mut self.levels);
        let done = thread::scope(|scope| {
            let host = thread::Builder::new().name(RUN_THREAD.into());
            let run = host
                .stack_size(STACK)
                .spawn_scoped(scope, || self.deeper(f));
            match run {
                Ok(run) => run.join().unwrap_or_else(|p| panic::resume_unwind(p)),
                Err(e) => Err(Stop::Internal(e.to_string())),
            }
        });
        self.levels = here;
        done
    }

    fn statements(&mut self, fr: &mut Frame, b: &Block) -> Done {
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
            Stmt::Expr(e) => self.lend(fr, e, |_| ())?,
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
        self.poll()?;
        match self.block(fr, body) {
            Ok(_) | Err(Stop::Continue) => Ok(false),
            Err(Stop::Break) => Ok(true),
            Err(stop) => Err(stop),
        }
    }

    fn poll(&self) -> Done<()> {
        match &self.interrupt {
            Some(flag) if flag.load(atomic::Ordering::Relaxed) => Err(Stop::Time),
            _ => Ok(()),
        }
    }

    fn truth(&mut self, fr: &mut Frame, e: &Expr) -> Done<bool> {
        Ok(self.eval(fr, e)?.bool())
    }

    pub fn eval(&mut self, fr: &mut Frame, e: &Expr) -> Done {
        self.deeper(|m| m.node(fr, e))
    }

    /// One arm each, so that this frame stays small: it is on the stack once per nesting level.
    fn node(&mut self, fr: &mut Frame, e: &Expr) -> Done {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Float(x) => Ok(Value::Float(*x)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Str(s) => Ok(Value::Str(Text::from(s.as_str()))),
            ExprKind::Interp(parts, site) => self.interpolate(fr, parts, *site),
            ExprKind::Hole(site) => Err(self.hole(*site).into()),
            ExprKind::Place(p) => self.read(fr, p),
            ExprKind::Record(_, fields) => self.items(fr, fields, Value::Record),
            ExprKind::List(items) => self.items(fr, items, Value::List),
            ExprKind::Map(pairs) => self.map(fr, pairs),
            ExprKind::Variant(tag, fields) => {
                self.items(fr, fields, |items| Value::Variant(number(tag), items))
            }
            ExprKind::FnValue(FnRef::Fn(id)) => Ok(Value::Fn(FnKey::Fn(*id))),
            ExprKind::FnValue(FnRef::Variant(e, v)) => Ok(Value::Fn(FnKey::Ctor(*e, *v))),
            ExprKind::Unary(op, x) => self.unary(fr, op, x),
            ExprKind::Binary(op, a, b) => self.binary(fr, op, a, b),
            ExprKind::Call(callee, args, site) => self.call(fr, callee, args, *site),
            ExprKind::If(c, then, other) => self.branch(fr, c, then, other.as_ref()),
            ExprKind::Match(subject, arms) => self.matching(fr, subject, arms),
            ExprKind::Root(lines) => {
                let mut kids = self.lines(fr, lines)?.into_iter();
                let (_, root) = kids.next().expect("a view's one root");
                Ok(Value::Node(Box::new(root)))
            }
            ExprKind::Element(row, args, children, site) => {
                self.element(fr, *row, args, children.as_ref(), *site)
            }
            ExprKind::Placeholder(site) => {
                let name = self.core.sites[*site].hole.clone().unwrap_or_default();
                Ok(Value::Node(Box::new(Node::Hole(name))))
            }
            ExprKind::Child(child) => self.child(fr, child),
        }
    }

    /// The nodes the element lines of `b` add.
    fn lines(&mut self, fr: &mut Frame, b: &Block) -> Done<Kids> {
        self.kids.push(vec![]);
        let done = self.block(fr, b);
        let kids = self.kids.pop().expect("the list just pushed");
        done.map(|_| kids)
    }

    fn element(
        &mut self,
        fr: &mut Frame,
        row: usize,
        args: &[Expr],
        children: Option<&Block>,
        site: Site,
    ) -> Done {
        let mut values = self.all(fr, args)?.into_iter();
        let mut next = || values.next().expect("a checked argument");
        let name = ELEMENTS[row].name;
        let node = match name {
            "Text" => Node::Text(text(next())),
            "Button" | "Input" => {
                let shown = text(next());
                let value = next();
                let text = show(self.core, args[1].ty.expect("a typed message"), &value);
                let message = Message { value, text };
                match name {
                    "Button" => Node::Button {
                        label: shown,
                        on: message,
                    },
                    _ => Node::Input {
                        text: shown,
                        on_change: message,
                        edit: Edit::default(),
                    },
                }
            }
            _ => {
                let gap = position(next().int()).ok_or_else(|| Trap::bare(Kind::Assert, site))?;
                let lines = children.expect("a child block");
                let children = tree::unique(name, self.lines(fr, lines)?);
                let gap = gap as u64;
                match name {
                    "Row" => Node::Row { gap, children },
                    _ => Node::Col { gap, children },
                }
            }
        };
        Ok(Value::Node(Box::new(node)))
    }

    /// Adds the line's node to the child list being built, under its key or its place.
    fn child(&mut self, fr: &mut Frame, c: &Child) -> Done {
        let Value::Node(node) = self.eval(fr, &c.node)? else {
            unreachable!("a node");
        };
        let key = match &c.key {
            Some(k) => Some(self.eval(fr, k)?.key()),
            None => None,
        };
        let kids = self.kids.last_mut().expect("a child list being built");
        let step = match key {
            Some(key) => step(key),
            None if c.looped && matches!(*node, Node::Hole(_)) => {
                let made = kids
                    .iter()
                    .filter(|(s, _)| matches!(s, tree::Step::LoopHole(l, _) if *l == c.line));
                tree::Step::LoopHole(c.line, made.count())
            }
            None => tree::Step::Place(c.line),
        };
        kids.push((step, *node));
        Ok(Value::Unit)
    }

    fn items(&mut self, fr: &mut Frame, es: &[Expr], make: impl FnOnce(Items) -> Value) -> Done {
        Ok(make(Items(self.all(fr, es)?)))
    }

    fn branch(&mut self, fr: &mut Frame, cond: &Expr, then: &Block, other: Option<&Block>) -> Done {
        match (self.truth(fr, cond)?, other) {
            (true, _) => self.block(fr, then),
            (false, Some(other)) => self.block(fr, other),
            (false, None) => Ok(Value::Unit),
        }
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
            self.lend(fr, part, |v| match v {
                Value::Int(n) => write!(text, "{n}").expect("writing to a String"),
                Value::Bool(b) => write!(text, "{b}").expect("writing to a String"),
                Value::Str(s) => text.push_str(s.as_str()),
                _ => unreachable!("a checked part"),
            })?;
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
            Bin::And => Ok(Value::Bool(self.truth(fr, a)? && self.truth(fr, b)?)),
            Bin::Or => Ok(Value::Bool(self.truth(fr, a)? || self.truth(fr, b)?)),
            Bin::Join(site) => self.join(fr, *site, a, b),
            Bin::Eq(site) | Bin::Ne(site) => self.same(fr, *site, matches!(op, Bin::Eq(_)), a, b),
            Bin::Order(cmp) => self.pair(fr, a, b, |x, y| Value::Bool(compare(cmp, x, y))),
            Bin::Int(arith, site) => self.numbers(fr, a, b, |x, y| {
                Ok(Value::Int(int_op(arith, *site, x.int(), y.int())?))
            }),
            Bin::Float(arith) => self.numbers(fr, a, b, |x, y| {
                Ok(Value::Float(float_op(arith, x.float(), y.float())))
            }),
        }
    }

    fn numbers(
        &mut self,
        fr: &mut Frame,
        a: &Expr,
        b: &Expr,
        f: impl FnOnce(Value, Value) -> Done,
    ) -> Done {
        let (x, y) = (self.eval(fr, a)?, self.eval(fr, b)?);
        f(x, y)
    }

    fn same(&mut self, fr: &mut Frame, site: Site, eq: bool, a: &Expr, b: &Expr) -> Done {
        let same = self.pair(fr, a, b, equals)?;
        let same = same.ok_or_else(|| Trap::bare(Kind::Stack, site))?;
        Ok(Value::Bool(same == eq))
    }

    fn join(&mut self, fr: &mut Frame, site: Site, a: &Expr, b: &Expr) -> Done {
        let Value::Str(mut s) = self.eval(fr, a)? else {
            unreachable!("a checked String");
        };
        self.lend(fr, b, |t| match t {
            Value::Str(t) => append(site, &mut s, t),
            _ => unreachable!("a checked String"),
        })??;
        Ok(Value::Str(s))
    }

    /// Runs `f` on the value of `e`, which stays in place when `e` is a place.
    fn lend<R>(&mut self, fr: &mut Frame, e: &Expr, f: impl FnOnce(&Value) -> R) -> Done<R> {
        match &e.kind {
            ExprKind::Place(p) => self.peek(fr, p, f),
            _ => Ok(f(&self.eval(fr, e)?)),
        }
    }

    /// Runs `f` on both operands in written order, lending the right one, and the left one when
    /// the right one is a literal; the other is copied, since evaluating the right one may change
    /// the left one's place.
    fn pair<R>(
        &mut self,
        fr: &mut Frame,
        a: &Expr,
        b: &Expr,
        f: impl FnOnce(&Value, &Value) -> R,
    ) -> Done<R> {
        if let ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Str(_) = b.kind
        {
            let y = self.eval(fr, b)?;
            return self.lend(fr, a, |x| f(x, &y));
        }
        let x = self.eval(fr, a)?;
        self.lend(fr, b, |y| f(&x, y))
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
        self.look(fr, p, |seen| f(seen.value()))
    }

    fn look<R>(&mut self, fr: &mut Frame, p: &Place, f: impl FnOnce(Seen) -> R) -> Done<R> {
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
        let copy = self.look(fr, p, |seen| seen.copy())?;
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
        let written = self.detector.is_some().then(|| spans(&path));
        let root = &mut fr.locals[*l];
        match op {
            None => store(root, path, value, CAP)?,
            Some(op) => {
                let slot = walk_mut(root, &path).map_err(|t| name_idiom(t, op, path.last()))?;
                compound(slot, op, value)?
            }
        }
        if let (Some(d), Some(written)) = (&mut self.detector, written) {
            d.wrote(self.depth, *l, &written);
        }
        Ok(())
    }

    fn call(&mut self, fr: &mut Frame, callee: &Callee, args: &[Arg], site: Site) -> Done {
        let target = match callee {
            Callee::Fn(id) => Target::User(*id),
            Callee::Builtin(row) => match row.checked_sub(ROWS.len()) {
                Some(k) => Target::Ui(k),
                None => Target::Builtin(*row),
            },
            Callee::Value(e) => match self.eval(fr, e)? {
                Value::Fn(FnKey::Fn(id)) => Target::User(id),
                Value::Fn(FnKey::Ctor(_, v)) => Target::Ctor(v),
                _ => unreachable!("a checked function value"),
            },
        };
        let lend = |i| match target {
            Target::Ctor(_) => false,
            Target::Builtin(row) => !generic::keeps(row, i),
            Target::User(_) | Target::Ui(_) => true,
        };
        let base = self.detector.as_ref().map_or(0, Detector::marks);
        let given = self.operands(fr, args, lend, site);
        if let Some(d) = &mut self.detector {
            let seen = given.as_ref().map_or(&[][..], Vec::as_slice);
            if d.start(self.depth, site, args, seen, base) {
                return Err(Stop::Internal("a call takes overlapping places".into()));
            }
        }
        let (mut params, loans) = take_args(fr, given?)?;
        // A trap skips `give_back`; the loaned slots stay `Unit`, which nothing can see once the run ends.
        let value = match target {
            Target::Ctor(v) => return Ok(Value::Variant(v, Items(params))),
            Target::Builtin(row) if generic::calls_back(row) => self.sort_by(&mut params, site)?,
            Target::Builtin(row) => builtin::call(row, &mut params, site)?,
            Target::Ui(k) => self.ui(k, &mut params, site)?,
            Target::User(id) => {
                let (value, locals) = self.invoke(id, params, site)?;
                params = locals;
                value
            }
        };
        give_back(fr, loans, &mut params);
        Ok(value)
    }

    /// `sort_by(&xs, before)`: M2-14 section 1.13's procedure, `before` called through its id. An
    /// item moves into the call and back, since `before` only reads it.
    fn sort_by(&mut self, args: &mut [Value], site: Site) -> Done {
        let [Value::List(xs), Value::Fn(FnKey::Fn(before))] = args else {
            unreachable!("a checked sort_by");
        };
        let mut frame = Vec::new();
        let mut ask = |right: &mut Value, left: &mut Value| -> Done<bool> {
            frame.extend([mem::take(right), mem::take(left)]);
            let (answer, mut kept) = self.invoke(*before, mem::take(&mut frame), site)?;
            (*right, *left) = (mem::take(&mut kept[0]), mem::take(&mut kept[1]));
            kept.clear();
            frame = kept;
            Ok(answer.bool())
        };
        generic::merge_sort(&mut xs.0, &mut ask)?;
        Ok(Value::Unit)
    }

    /// The arguments in written order.
    fn operands(
        &mut self,
        fr: &mut Frame,
        args: &[Arg],
        lend: impl Fn(usize) -> bool,
        site: Site,
    ) -> Done<Vec<Given>> {
        let mut given: Vec<Given> = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let g = self.given(fr, arg, lend(i), &given, site)?;
            given.push(g);
        }
        Ok(given)
    }

    fn given(
        &mut self,
        fr: &mut Frame,
        arg: &Arg,
        lend: bool,
        earlier: &[Given],
        site: Site,
    ) -> Done<Given> {
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
                    let heavy = !held_in_register(&fr.locals[*l]);
                    if lend && steps.is_empty() && heavy && !earlier.iter().any(taken) {
                        if let Some(d) = &mut self.detector {
                            d.read(self.depth, *l, &[], &fr.locals[*l], site);
                        }
                        return Ok(Given::Borrow(*l, vec![]));
                    }
                    if self.detector.is_some() {
                        return self.watched(fr, *l, steps, site);
                    }
                }
                Ok(Given::Own(self.eval(fr, e)?))
            }
        }
    }

    /// A read-only place argument read as `read` reads it, noted for the detector.
    fn watched(&mut self, fr: &mut Frame, l: Local, steps: &[Step], site: Site) -> Done<Given> {
        let path = self.path(fr, steps)?;
        let copy = walk(&fr.locals[l], &path, fr.site, |seen| seen.copy())?;
        let value = copy.ok_or_else(|| Trap::bare(Kind::Stack, fr.site))?;
        if let Some(d) = &mut self.detector {
            d.read(self.depth, l, &path, &value, site);
        }
        Ok(Given::Own(value))
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

pub(super) fn out_of_bounds(site: Site, detail: String) -> Trap {
    Trap::new(Kind::OutOfBounds, site, detail)
}

pub(super) fn bound(i: Int, len: usize, site: Site) -> Result<usize, Trap> {
    position(i)
        .filter(|n| *n < len)
        .ok_or_else(|| out_of_bounds(site, format!("index {i}, length {len}")))
}

fn missing(key: &Key, site: Site) -> Trap {
    out_of_bounds(site, format!("key {}", key.show()))
}

/// An Int `+=` on a missing last key names the count idiom (M2-14 section 1.11).
fn name_idiom(mut trap: Trap, op: &Compound, last: Option<&Idx>) -> Trap {
    let counts = matches!(op, Compound::Int(Arith::Add, _));
    if counts && matches!(last, Some(Idx::Key(_, site)) if *site == trap.site) {
        trap.detail += "; write m[k] = unwrap_or(get(m, k), 0) + 1";
    }
    trap
}

/// Runs `f` on the value at the end of `path`.
fn walk<R>(v: &Value, path: &[Idx], home: Site, f: impl FnOnce(Seen) -> R) -> Result<R, Trap> {
    let Some((step, rest)) = path.split_first() else {
        return Ok(f(Seen::Held(v)));
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
            made(Value::Str(item), rest, home, f)
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
            made(Value::List(Items(part)), rest, home, f)
        }
        (Idx::Slice(a, b, site), Value::Str(t)) => {
            let part = t
                .slice(*a, *b)
                .map_err(|_| slice_error(*a, *b, t.len(), *site))?;
            made(Value::Str(part), rest, home, f)
        }
        _ => unreachable!("a checked place"),
    }
}

/// `walk` on a fresh value, which `f` gets whole when no step is left.
fn made<R>(v: Value, rest: &[Idx], home: Site, f: impl FnOnce(Seen) -> R) -> Result<R, Trap> {
    if rest.is_empty() {
        return Ok(f(Seen::Made(v)));
    }
    walk(&v, rest, home, f)
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

pub(super) fn compare(cmp: &Cmp, a: &Value, b: &Value) -> bool {
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

/// A key as a step of the tree, where it leaves Paka.
#[allow(clippy::disallowed_methods)]
fn step(key: Key) -> tree::Step {
    match key {
        Key::Int(n) => tree::Step::Int(n.get()),
        Key::Str(t) => tree::Step::Str(t.as_str().into()),
    }
}

fn text(v: Value) -> String {
    match v {
        Value::Str(t) => t.as_str().into(),
        _ => unreachable!("a checked String"),
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
