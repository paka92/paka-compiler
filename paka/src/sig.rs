//! The signature table of a parsed and resolved program.

use crate::ast::{Def, Pos, Program};
use crate::diagnostic::Diagnostic;
use crate::resolve::Resolved;
use std::collections::BTreeMap;

mod hash;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ty(u32);

pub struct Types<'a> {
    names: Vec<&'a str>,
}

impl Types<'_> {
    pub fn show(&self, _ty: Ty) -> String {
        todo!()
    }
}

pub struct Param<'a> {
    pub name: &'a str,
    pub inout: bool,
    pub labelled: bool,
    pub ty: Ty,
}

pub struct Sig<'a> {
    pub params: Vec<Param<'a>>,
    pub result: Option<Ty>,
}

pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Ty,
    pub default: bool,
}

pub struct Variant<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
}

pub enum Item<'a> {
    Record(Vec<Field<'a>>),
    Enum(Vec<Variant<'a>>),
    Fn(Sig<'a>),
    Example,
    App {
        state: Option<Ty>,
        update: Option<usize>,
    },
}

pub struct Table<'a> {
    pub types: Types<'a>,
    pub defs: Vec<Result<Item<'a>, Diagnostic>>,
    pub fns: BTreeMap<&'static str, Vec<Sig<'static>>>,
    pub variants: BTreeMap<&'static str, Sig<'static>>,
}

impl<'a> Table<'a> {
    pub fn build(_path: &str, _program: &'a Program<'a>, _resolved: &Resolved) -> Table<'a> {
        todo!()
    }

    pub fn hash(&self, _def: usize) -> Option<u64> {
        todo!()
    }
}
