//! The syntax tree of the approved grammar (docs/decisions/M2-02.md section 2, with M2-03, M2-05,
//! M2-06 and M2-09). Every `Node` carries the position of its first token and its length in bytes up
//! to its last token; a parenthesised expression spans its parentheses.

use std::fmt;

// A new node enum must also be listed in `coverage::node_kinds!`.

/// (line, column), both from 1.
pub type Pos = (usize, usize);

#[derive(Debug, PartialEq)]
pub struct Name<'a> {
    pub text: &'a str,
    pub at: Pos,
}

#[derive(Debug, PartialEq)]
pub struct Node<K> {
    pub at: Pos,
    pub len: usize,
    pub kind: K,
}

pub type Expr<'a> = Node<ExprKind<'a>>;
pub type Stmt<'a> = Node<StmtKind<'a>>;
pub type Pat<'a> = Node<PatKind<'a>>;

#[derive(Debug, Default, PartialEq)]
pub struct Program<'a> {
    pub defs: Vec<Def<'a>>,
    /// Each file's first line, in program order; positions count lines across the files.
    pub lines: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub enum Def<'a> {
    Type {
        name: Name<'a>,
        fields: Vec<Field<'a>>,
    },
    Enum {
        name: Name<'a>,
        variants: Vec<Variant<'a>>,
    },
    Fn {
        name: Name<'a>,
        params: Vec<Param<'a>>,
        result: Option<Type<'a>>,
        body: Block<'a>,
    },
    Example {
        name: Name<'a>,
        body: Block<'a>,
    },
    App {
        name: Name<'a>,
        entries: Vec<Entry<'a>>,
    },
}

impl<'a> Def<'a> {
    pub fn name(&self) -> &Name<'a> {
        match self {
            Def::Type { name, .. }
            | Def::Enum { name, .. }
            | Def::Fn { name, .. }
            | Def::Example { name, .. }
            | Def::App { name, .. } => name,
        }
    }
}

/// A record field (with an optional default) or a variant field (never a default).
#[derive(Debug, PartialEq)]
pub struct Field<'a> {
    pub name: Name<'a>,
    pub ty: Type<'a>,
    pub default: Option<Expr<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct Variant<'a> {
    pub name: Name<'a>,
    pub fields: Vec<Field<'a>>,
}

/// `labelled` is true after the `;`.
#[derive(Debug, PartialEq)]
pub struct Param<'a> {
    pub name: Name<'a>,
    pub inout: bool,
    pub labelled: bool,
    pub ty: Type<'a>,
}

/// The `Pos` is the position of the key.
#[derive(Debug, PartialEq)]
pub enum Entry<'a> {
    State(Pos, Name<'a>),
    Start(Pos, Expr<'a>),
    Update(Pos, Name<'a>),
    Grants(Pos, Vec<Name<'a>>),
}

#[derive(Debug, PartialEq)]
pub enum Type<'a> {
    Name {
        name: Name<'a>,
        args: Vec<Type<'a>>,
    },
    /// `fn(A, B) -> R`; `at` is the `fn`.
    Fn {
        at: Pos,
        params: Vec<Type<'a>>,
        result: Option<Box<Type<'a>>>,
    },
}

impl fmt::Display for Type<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let list = |types: &[Type]| {
            let texts: Vec<_> = types.iter().map(Type::to_string).collect();
            texts.join(", ")
        };
        match self {
            Type::Name { name, args } if args.is_empty() => write!(f, "{}", name.text),
            Type::Name { name, args } => write!(f, "{}<{}>", name.text, list(args)),
            Type::Fn { params, result, .. } => {
                write!(f, "fn({})", list(params))?;
                result.as_ref().map_or(Ok(()), |r| write!(f, " -> {r}"))
            }
        }
    }
}

/// `close` is the position of the closing brace.
#[derive(Debug, PartialEq)]
pub struct Block<'a> {
    pub stmts: Vec<Stmt<'a>>,
    pub close: Pos,
}

#[derive(Debug, PartialEq)]
pub enum StmtKind<'a> {
    /// `let` or, with `var`, `var`.
    Let {
        var: bool,
        name: Name<'a>,
        ty: Option<Type<'a>>,
        value: Expr<'a>,
    },
    /// `_ = value`
    Discard(Expr<'a>),
    /// Any expression may stand left of the operator; the checker decides what is a place.
    Assign {
        target: Expr<'a>,
        op: AssignOp,
        value: Expr<'a>,
    },
    Return(Option<Expr<'a>>),
    Break,
    Continue,
    For {
        name: Name<'a>,
        list: Expr<'a>,
        body: Block<'a>,
    },
    While {
        cond: Expr<'a>,
        body: Block<'a>,
    },
    Expr(Expr<'a>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        use BinOp::*;
        match self {
            Or => "||",
            And => "&&",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ExprKind<'a> {
    /// The spelling, with a folded minus sign.
    Int(&'a str),
    Float(&'a str),
    Bool(bool),
    Str(Vec<StrPart<'a>>),
    /// The name after `??`.
    Hole(&'a str),
    /// A LOWER or UPPER name; a bare variant is a name too.
    Name(&'a str),
    Record(&'a str, Vec<Arg<'a>>),
    List(Vec<Expr<'a>>),
    /// `[:]` is an empty map.
    Map(Vec<(Expr<'a>, Expr<'a>)>),
    Unary(UnOp, Box<Expr<'a>>),
    /// The operator, the position of its token, then the operands.
    Binary(BinOp, Pos, Box<Expr<'a>>, Box<Expr<'a>>),
    /// `a..<b`, only after `in` and as an index.
    Range(Box<Expr<'a>>, Box<Expr<'a>>),
    Index(Box<Expr<'a>>, Box<Expr<'a>>),
    Field(Box<Expr<'a>>, Name<'a>),
    /// `e?`, with the position of the `?`.
    Try(Box<Expr<'a>>, Pos),
    Call(Box<Expr<'a>>, Vec<Arg<'a>>),
    /// `&place`, only as the value of an argument.
    Inout(Box<Expr<'a>>),
    /// `if` and its `else if` branches, then the `else`.
    If {
        branches: Vec<(Expr<'a>, Block<'a>)>,
        otherwise: Option<Block<'a>>,
    },
    Match {
        subject: Box<Expr<'a>>,
        arms: Vec<Arm<'a>>,
    },
}

#[derive(Debug, PartialEq)]
pub enum StrPart<'a> {
    /// Literal text with its escapes decoded.
    Text(String),
    Expr(Expr<'a>),
}

/// A call argument or a record field.
#[derive(Debug, PartialEq)]
pub struct Arg<'a> {
    pub label: Option<Name<'a>>,
    pub value: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct Arm<'a> {
    /// Alternatives joined by `|`.
    pub patterns: Vec<Pat<'a>>,
    pub body: ArmBody<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ArmBody<'a> {
    Block(Block<'a>),
    /// An assignment or an expression on the arm's line.
    Line(Stmt<'a>),
}

#[derive(Debug, PartialEq)]
pub enum PatKind<'a> {
    Wild,
    Int(&'a str),
    Str(String),
    Bool(bool),
    /// A name in a variant pattern.
    Bind(&'a str),
    Variant(&'a str, Vec<Pat<'a>>),
}
