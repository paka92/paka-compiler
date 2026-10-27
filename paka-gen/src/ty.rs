//! The types of a generated program and the user types it declares.

use std::fmt;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    List(Box<Ty>),
    Map(Box<Ty>, Box<Ty>),
    Opt(Box<Ty>),
    Res(Box<Ty>, Box<Ty>),
    Rec(usize),
    Enum(usize),
    Fn(Vec<Ty>, Box<Ty>),
}

impl Ty {
    pub fn list(t: Ty) -> Ty {
        Ty::List(Box::new(t))
    }

    pub fn opt(t: Ty) -> Ty {
        Ty::Opt(Box::new(t))
    }

    /// Int, Float or String: what `<`, `min`, `max` and `sort` take.
    pub fn ordered(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float | Ty::Str)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ty::Int => write!(f, "Int"),
            Ty::Float => write!(f, "Float"),
            Ty::Bool => write!(f, "Bool"),
            Ty::Str => write!(f, "String"),
            Ty::List(t) => write!(f, "List<{t}>"),
            Ty::Map(k, v) => write!(f, "Map<{k}, {v}>"),
            Ty::Opt(t) => write!(f, "Option<{t}>"),
            Ty::Res(t, e) => write!(f, "Result<{t}, {e}>"),
            Ty::Rec(i) => write!(f, "R{i}"),
            Ty::Enum(i) => write!(f, "E{i}"),
            Ty::Fn(params, result) => {
                let params: Vec<_> = params.iter().map(Ty::to_string).collect();
                write!(f, "fn({}) -> {result}", params.join(", "))
            }
        }
    }
}

/// Field `j` is `x{j}`; the flag says it has a default.
pub struct Record {
    pub fields: Vec<(Ty, bool)>,
}

/// Variant `j` of enum `i` is `V{i}_{j}`, its fields `y0`, `y1`...
pub struct Enum {
    pub variants: Vec<Vec<Ty>>,
}

/// Function `i` is `f{i}`, its parameters `p0`, `p1`...
pub struct Func {
    pub params: Vec<(Ty, bool)>,
    pub result: Option<Ty>,
    /// A bound on the statements one call runs, callees included.
    pub cost: u64,
}

impl Func {
    pub fn inout(&self) -> bool {
        self.params.iter().any(|p| p.1)
    }
}
