//! Name resolution: binds every use of a name to its definition and reports the first duplicate
//! or unknown name, with the nearest known name as the fix.

use crate::ast::{Pos, Program};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

type Res<T = ()> = Result<T, Diagnostic>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {
    Local(Pos),
    Def(usize),
    Variant(usize, usize),
    Field(usize, usize),
    Builtin(&'static str),
}

#[derive(Debug)]
pub struct Resolved {
    uses: BTreeMap<Pos, Binding>,
}

impl Resolved {
    pub fn get(&self, at: Pos) -> Option<Binding> {
        self.uses.get(&at).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Type,
    Variant,
    Fn,
    Grant,
    Other,
}

pub const BUILTINS: [(Kind, &str); 4] = [
    (Kind::Type, ""),
    (Kind::Variant, ""),
    (Kind::Grant, ""),
    (Kind::Fn, ""),
];

pub fn resolve<'a>(_path: &'a str, _program: &'a Program<'a>) -> Res<Resolved> {
    todo!("name resolution")
}

#[cfg(test)]
mod tests;
