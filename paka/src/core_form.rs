//! The core form: what every back end runs. docs/core.md gives each node its type rule,
//! evaluation order and trap points; the headings there match the types here.

pub use paka_rt::effects::Grant;
use paka_rt::ops::{Float, Int};
use std::collections::BTreeSet;

/// Positions in the program's lists.
pub type Ty = usize;
pub type Site = usize;
pub type FnId = usize;
pub type RecordId = usize;
pub type EnumId = usize;
/// Parameters first, then each binding in order of appearance.
pub type Local = usize;

#[derive(Debug, Default, PartialEq)]
pub struct Program {
    /// Each file's own name and the line it starts on, in program order (M2-02 section 1.3).
    pub files: Vec<(String, usize)>,
    pub types: Vec<Type>,
    pub records: Vec<Record>,
    pub enums: Vec<Enum>,
    pub fns: Vec<Fn>,
    pub examples: Vec<Fn>,
    /// The examples that drive the app through a `Ui` (M5-06): only the interpreter runs them.
    pub ui_examples: Vec<Fn>,
    pub app: Option<App>,
    pub sites: Vec<TrapSite>,
    /// Each foreign effect, in declared order; an `Effect::Foreign` value's first field is its
    /// index here.
    pub foreign: Vec<Foreign>,
}

/// A function of a foreign declaration: its effect's name and the declaration's, its grant.
#[derive(Debug, PartialEq)]
pub struct Foreign {
    pub name: String,
    pub grant: String,
}

impl Program {
    /// The number of `t`, added on first use; equal types share one number.
    pub fn ty(&mut self, t: Type) -> Ty {
        match self.types.iter().position(|known| *known == t) {
            Some(at) => at,
            None => {
                self.types.push(t);
                self.types.len() - 1
            }
        }
    }

    /// Definition `def`: a function, or example `def - fns.len()`.
    pub fn def(&self, def: usize) -> &Fn {
        (self.fns.get(def)).unwrap_or_else(|| &self.examples[def - self.fns.len()])
    }

    /// The definition and site that compiled code's `def << 32 | offset` names: the offset counts
    /// from the definition's first site (M4-05 section 6).
    pub fn place(&self, at: u64) -> (&Fn, &TrapSite) {
        let f = self.def((at >> 32) as usize);
        (f, &self.sites[f.site + (at & 0xFFFF_FFFF) as usize])
    }
}

#[derive(Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    List(Ty),
    Map(Ty, Ty),
    Option(Ty),
    Result(Ty, Ty),
    Effect(Ty),
    /// Parameters, and the result if any.
    Fn(Vec<Ty>, Option<Ty>),
    Record(RecordId),
    Enum(EnumId),
    /// `View<M>`: an element tree whose messages are of type `M` (M5-01).
    View(Ty),
    /// The program's visual app running with no window (M5-06).
    Ui,
}

#[derive(Debug, PartialEq)]
pub struct Record {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<Variant>,
}

#[derive(Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

/// A function or an example. `site` is its name in the definition.
#[derive(Debug, PartialEq)]
pub struct Fn {
    pub name: String,
    pub site: Site,
    pub params: Vec<Param>,
    /// The type of every local: the parameters first, then each binding in numbering order,
    /// including the ones lowering adds.
    pub locals: Vec<Ty>,
    pub result: Option<Ty>,
    pub body: Block,
    /// Of an example: where each operand of its final `&&` chain is reported when false.
    pub claims: Vec<Site>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Param {
    pub ty: Ty,
    pub inout: bool,
}

#[derive(Debug, PartialEq)]
pub struct App {
    pub name: String,
    /// The state record literal with every default written in.
    pub state: Option<Expr>,
    /// A variant of the message enum, the type of `update`'s last parameter.
    pub start: Option<usize>,
    pub update: FnId,
    /// The view of a visual app.
    pub view: Option<FnId>,
    pub grants: BTreeSet<Grant>,
    /// The foreign declarations it is granted.
    pub foreign: BTreeSet<String>,
}

/// The variants of `Effect<M>`, in declared order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    Print,
    ReadFile,
    WriteFile,
    Exit,
    ExitWith,
    /// A foreign effect: its index in `Program::foreign`, then its arguments, `then` and `or`.
    Foreign,
}

/// Where a trap is reported; `line` counts across the files, the kind travels with the trap.
#[derive(Debug, PartialEq)]
pub struct TrapSite {
    pub line: usize,
    pub col: usize,
    /// The name of a hole.
    pub hole: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub value: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Let(Local, Expr),
    /// `place = e`, or `place op= e` with the operator.
    Assign(Place, Option<Compound>, Expr),
    Return(Option<Expr>),
    Break,
    Continue,
    For(Local, Walk, Block),
    While(Expr, Block),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Walk {
    List(Expr),
    /// `a..<b`
    Range(Expr, Expr),
}

/// `ty` is `None` for a node that gives no value.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub ty: Option<Ty>,
    pub kind: ExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(Int),
    Float(Float),
    Bool(bool),
    Str(String),
    /// The parts in order; the site is the opening quote.
    Interp(Vec<Expr>, Site),
    Hole(Site),
    Place(Place),
    /// Fields in declared order.
    Record(RecordId, Vec<Expr>),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Variant(Tag, Vec<Expr>),
    FnValue(FnRef),
    Unary(Un, Box<Expr>),
    Binary(Bin, Box<Expr>, Box<Expr>),
    Call(Callee, Vec<Arg>, Site),
    /// `else if` nests in the else block.
    If(Box<Expr>, Block, Option<Block>),
    Match(Box<Expr>, Vec<Arm>),
    /// A view's root: the one node its element lines give.
    Root(Block),
    /// A row of `builtins::ELEMENTS`, its arguments in table order with defaults written in, and
    /// its child block. The site is the element's name.
    Element(usize, Vec<Expr>, Option<Block>, Site),
    /// A hole where an element may stand: a placeholder node.
    Placeholder(Site),
    /// An element line of a view: adds its node to the child list being built.
    Child(Box<Child>),
}

/// `line` counts the element lines of a child block from 0 (M5-02 section 1.4); `looped` when a
/// `for` adds it. `node` is an element, a view call or a placeholder.
#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    pub line: usize,
    pub looped: bool,
    pub key: Option<Expr>,
    pub node: Expr,
}

/// A write or an `inout` argument is rooted at a `Local`; `Root::Value` never appears in one.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub root: Root,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Root {
    Local(Local),
    Value(Box<Expr>),
}

/// Each site is the step's `[`.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Field(usize),
    Item(Expr, Site),
    Key(Expr, Site),
    Slice(Expr, Expr, Site),
}

/// A variant of a user enum, `Option`, `Result` or `Effect`.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    User(EnumId, usize),
    Some,
    None,
    Ok,
    Err,
    Effect(Effect),
}

#[derive(Clone, Debug, PartialEq)]
pub enum FnRef {
    Fn(FnId),
    Variant(EnumId, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    Fn(FnId),
    /// An index into `builtins::ROWS`, chosen once by the checker; `ROWS.len() + k` is row `k` of
    /// `builtins::UI`. `ui_start` takes no argument, since a program holds one app.
    Builtin(usize),
    Value(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Value(Expr),
    Inout(Place),
}

/// The tags it decides; a one-tag arm binds each field to a local or to nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub tags: Vec<Tag>,
    pub binds: Vec<Option<Local>>,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Un {
    Neg(Site),
    NegFloat,
    Not,
}

/// The site, where an operator has one, is the operator.
#[derive(Clone, Debug, PartialEq)]
pub enum Bin {
    Int(Arith, Site),
    Float(FloatArith),
    Join(Site),
    Eq(Site),
    Ne(Site),
    Order(Cmp),
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FloatArith {
    Add,
    Sub,
    Mul,
    Div,
}

/// The operators `op=` allows: Int `+ - * / %`, Float `+ - * /`, String `+`.
#[derive(Clone, Debug, PartialEq)]
pub enum Compound {
    Int(Arith, Site),
    Float(FloatArith),
    Join(Site),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_types_share_one_number() {
        let mut p = Program {
            files: vec![("main.paka".into(), 1)],
            types: vec![],
            records: vec![],
            enums: vec![],
            fns: vec![],
            examples: vec![],
            ui_examples: vec![],
            foreign: vec![],
            app: None,
            sites: vec![],
        };
        let s = p.ty(Type::String);
        let list = p.ty(Type::List(s));
        assert_eq!((p.ty(Type::String), p.ty(Type::List(s))), (s, list));
        assert_eq!(p.types, [Type::String, Type::List(0)]);
    }
}
