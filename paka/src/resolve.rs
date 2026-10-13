//! Name resolution (docs/decisions/M2-02.md section 1.3, M2-14.md section 1.2). Binds every use of
//! a name to its definition and reports duplicate and unknown names, with the nearest known name as
//! the fix. No types are looked at, so a field read `x.f` and the labels of calls and records are
//! left to the checker.

use crate::ast::{Pos, Program};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

type Res<T = ()> = Result<T, Diagnostic>;

/// What a use of a name stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {
    /// A `let`, `var`, parameter, `for` name or pattern name, by the position of its name.
    Local(Pos),
    /// A `type`, `enum`, `fn`, `example` or `app`, by its index in `Program::defs`.
    Def(usize),
    /// A variant: its enum's index in `Program::defs`, then its index in the enum.
    Variant(usize, usize),
    /// A name of `BUILTINS`.
    Builtin(&'static str),
}

/// The outcome for a program whose declarations are sound. A name error in one definition never
/// hides or changes the result of another.
#[derive(Debug)]
pub struct Resolved {
    uses: BTreeMap<Pos, Binding>,
    /// The first name error of each definition, by index. An app's name errors are not here but in
    /// `entry_error`, so its result is always `Ok`.
    pub defs: Vec<Res>,
    entries: BTreeMap<Pos, Diagnostic>,
}

impl Resolved {
    pub fn get(&self, at: Pos) -> Option<Binding> {
        self.uses.get(&at).copied()
    }

    /// The name error of an app entry's value, by the position where the value starts: the name
    /// after `state:` or `update:`, the first token after `start:`, each name of `grants:`. The
    /// checker reports it at its turn in the order of M2-15 section 1.1, after its own key tests.
    pub fn entry_error(&self, at: Pos) -> Option<&Diagnostic> {
        self.entries.get(&at)
    }
}

/// What a name in the pool is: where a use may name it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Type,
    Variant,
    Fn,
    Grant,
    /// An `example` or an `app`: it holds a name but is never used by name.
    Other,
}

/// The built-in names: M2-14 section 1.1 (types, constructors), section 1.2 (63 functions) and
/// M2-15 section 1.3 (effects, grants).
pub const BUILTINS: [(Kind, &str); 4] = [
    (
        Kind::Type,
        "Int Float Bool String List Map Option Result Effect",
    ),
    (
        Kind::Variant,
        "Some None Ok Err Print ReadFile WriteFile Exit ExitWith",
    ),
    (Kind::Grant, "Console ReadFiles WriteFiles"),
    (
        Kind::Fn,
        "min max abs to_float to_int sqrt floor ceil trunc round fixed
         wrap_add wrap_sub wrap_mul bit_and bit_or bit_xor bit_not shift_left shift_right
         len is_empty contains find parse_int parse_float code_at from_code
         split chars lines words join repeat pad_left pad_right
         trim trim_start trim_end starts_with ends_with replace upper lower
         append extend pop insert remove_at swap reverse sort sort_by list_of
         get has remove keys values unwrap_or ok_or assert unreachable",
    ),
];

/// Declaration errors come first, as `Err`; then each definition is resolved on its own.
pub fn resolve<'a>(_path: &'a str, _program: &'a Program<'a>) -> Res<Resolved> {
    todo!("name resolution")
}

#[cfg(test)]
mod tests;
