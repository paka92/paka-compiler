//! The syntax tree of the M1 grammar (spec/m1.md section 4). Every node the checker reports on
//! carries the position of its first token.

use std::fmt;

/// (line, column), both from 1.
pub type Pos = (usize, usize);

#[derive(Debug, PartialEq)]
pub struct Name<'a> {
    pub text: &'a str,
    pub at: Pos,
}

#[derive(Debug, PartialEq)]
pub struct Program<'a> {
    pub defs: Vec<Def<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Def<'a> {
    Enum {
        name: Name<'a>,
        variants: Vec<Name<'a>>,
    },
    Fn {
        name: Name<'a>,
        param: Name<'a>,
        param_type: Type<'a>,
        result: Type<'a>,
        body: Expr<'a>,
    },
    App {
        name: Name<'a>,
        entries: Vec<Entry<'a>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct Type<'a> {
    pub name: Name<'a>,
    pub arg: Option<Box<Type<'a>>>,
}

impl fmt::Display for Type<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name.text)?;
        self.arg.as_ref().map_or(Ok(()), |arg| write!(f, "<{arg}>"))
    }
}

/// `at` is the first token of the expression; `len` is its length in bytes, up to the last token.
#[derive(Debug, PartialEq)]
pub struct Expr<'a> {
    pub at: Pos,
    pub len: usize,
    pub kind: ExprKind<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ExprKind<'a> {
    Str(&'a str),
    List(Vec<Expr<'a>>),
    /// An UPPER name with its arguments; a bare name has none, as `Exit()` is not in the grammar.
    Apply(&'a str, Vec<Expr<'a>>),
}

impl Def<'_> {
    pub fn name(&self) -> &Name<'_> {
        match self {
            Def::Enum { name, .. } | Def::Fn { name, .. } | Def::App { name, .. } => name,
        }
    }
}

/// The `Pos` is the position of the key.
#[derive(Debug, PartialEq)]
pub enum Entry<'a> {
    Start(Pos, Name<'a>),
    Update(Pos, Name<'a>),
    Grants(Pos, Vec<Name<'a>>),
}
