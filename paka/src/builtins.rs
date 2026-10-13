//! The built-in names with their signatures, in one table.

/// What a use of a name may stand for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Type,
    Variant,
    Fn,
    Grant,
    /// An `example` or an `app`: it holds a name but is never used by name.
    Other,
}

pub const ROWS: &[(Kind, &str)] = &[];

/// The name a row defines.
pub fn name(_row: &'static str) -> &'static str {
    todo!()
}
