//! The syntax tree of the M1 grammar (spec/m1.md section 4). Every node the checker reports on
//! carries the position of its first token.

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

/// `at` is the first token of the expression.
#[derive(Debug, PartialEq)]
pub struct Expr<'a> {
    pub at: Pos,
    pub kind: ExprKind<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ExprKind<'a> {
    Str(&'a str),
    List(Vec<Expr<'a>>),
    /// An UPPER name with its arguments; a bare name has none, as `Exit()` is not in the grammar.
    Apply(&'a str, Vec<Expr<'a>>),
}

/// The `Pos` is the position of the key.
#[derive(Debug, PartialEq)]
pub enum Entry<'a> {
    Start(Pos, Name<'a>),
    Update(Pos, Name<'a>),
    Grants(Pos, Vec<Name<'a>>),
}
