//! The core form: what every back end runs. docs/core.md gives each node its type rule,
//! evaluation order and trap points; the headings there match the types here.

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

#[derive(Debug, PartialEq)]
pub struct Program {
    pub file: String,
    pub types: Vec<Type>,
    pub records: Vec<Record>,
    pub enums: Vec<Enum>,
    pub fns: Vec<Fn>,
    pub examples: Vec<Fn>,
    pub app: Option<App>,
    pub sites: Vec<TrapSite>,
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
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers the rest.
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
    pub result: Option<Ty>,
    pub body: Block,
}

#[derive(Debug, PartialEq)]
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
    pub start: usize,
    pub update: FnId,
    pub grants: BTreeSet<Grant>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grant {
    Console,
    ReadFiles,
    WriteFiles,
}

/// The variants of `Effect<M>`, in declared order.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers the rest.
pub enum Effect {
    Print,
    ReadFile,
    WriteFile,
    Exit,
    ExitWith,
}

/// Where a trap is reported; the file is the program's, the kind travels with the trap.
#[derive(Debug, PartialEq)]
pub struct TrapSite {
    pub line: usize,
    pub col: usize,
    /// The name of a hole.
    pub hole: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Stmt {
    Let(Local, Expr),
    /// `place = e`, or `place op= e` with the operator.
    Assign(Place, Option<Bin>, Expr),
    Return(Option<Expr>),
    Break,
    Continue,
    For(Local, Walk, Block),
    While(Expr, Block),
    Expr(Expr),
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Walk {
    List(Expr),
    /// `a..<b`
    Range(Expr, Expr),
}

/// `ty` is `None` for a node that gives no value.
#[derive(Debug, PartialEq)]
pub struct Expr {
    pub ty: Option<Ty>,
    pub kind: ExprKind,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers the rest.
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
}

/// Writes and `inout` need a `Local` root.
#[derive(Debug, PartialEq)]
pub struct Place {
    pub root: Root,
    pub steps: Vec<Step>,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Root {
    Local(Local),
    Value(Box<Expr>),
}

/// Each site is the step's `[`.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Step {
    Field(usize),
    Item(Expr, Site),
    Key(Expr, Site),
    Slice(Expr, Expr, Site),
}

/// A variant of a user enum, `Option`, `Result` or `Effect`.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers the rest.
pub enum Tag {
    User(EnumId, usize),
    Some,
    None,
    Ok,
    Err,
    Effect(Effect),
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum FnRef {
    Fn(FnId),
    Variant(EnumId, usize),
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Callee {
    Fn(FnId),
    /// The row of paka/src/builtins.rs.
    Builtin(&'static str),
    Value(Box<Expr>),
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Arg {
    Value(Expr),
    Inout(Place),
}

/// The tags it decides; a one-tag arm binds each field to a local or to nothing.
#[derive(Debug, PartialEq)]
pub struct Arm {
    pub tags: Vec<Tag>,
    pub binds: Vec<Option<Local>>,
    pub body: Block,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Un {
    Neg(Site),
    NegFloat,
    Not,
}

/// The site, where an operator has one, is the operator.
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Bin {
    Int(Arith, Site),
    Float(Arith),
    Join(Site),
    Eq(Site),
    Ne(Site),
    Order(Cmp),
    And,
    Or,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)] // M2-25 lowers these.
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
            file: "main.paka".into(),
            types: vec![],
            records: vec![],
            enums: vec![],
            fns: vec![],
            examples: vec![],
            app: None,
            sites: vec![],
        };
        let s = p.ty(Type::String);
        let list = p.ty(Type::List(s));
        assert_eq!((p.ty(Type::String), p.ty(Type::List(s))), (s, list));
        assert_eq!(p.types, [Type::String, Type::List(0)]);
    }
}
