//! The statement and expression checker: every body is checked against the signature table
//! alone, never against another definition's body (docs/decisions/M2-02.md, M2-09.md, M2-12.md).

use crate::ast::{Pos, Program};
use crate::diagnostic::Diagnostic;
use crate::resolve::Resolved;
use crate::sig::{Table, Ty};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// The result of checking a program's bodies, one entry per definition.
pub struct Bodies {
    /// The first error of each definition, or none.
    pub defs: Vec<Option<Diagnostic>>,
    /// The type of every expression met, by (position, length in bytes); partial after an error.
    pub types: Vec<BTreeMap<(Pos, usize), Ty>>,
}

pub fn check<'a>(
    _path: &str,
    _program: &'a Program<'a>,
    _resolved: &Resolved,
    _table: &mut Table<'a>,
) -> Bodies {
    todo!()
}
