//! The operations of docs/decisions/M2-03.md section 2, once, for the interpreter and the runtime.
//!
//! `Int` and `Float` expose the table's operations and nothing else; this is the one module of
//! the workspace that may do raw arithmetic and call `f64` maths methods.

#![allow(clippy::float_arithmetic, clippy::disallowed_methods)]
#![allow(clippy::should_implement_trait)]

mod fixed;
mod float;
mod int;
mod sort;
mod text;
pub mod vectors;

pub use float::Float;
pub use int::Int;
pub use sort::sorted;
pub use text::{Text, white};

/// The most bytes in a String, and the most items or entries in a List or Map.
pub const CAP: usize = 16_777_216;

/// The kinds of trap an operation can end in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    Overflow,
    DivideByZero,
    OutOfBounds,
    BadConversion,
    /// A String, List or Map past `CAP`.
    Memory,
}

impl Trap {
    const ALL: [Trap; 4] = [
        Trap::Overflow,
        Trap::DivideByZero,
        Trap::OutOfBounds,
        Trap::BadConversion,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Trap::Overflow => "overflow",
            Trap::DivideByZero => "divide_by_zero",
            Trap::OutOfBounds => "out_of_bounds",
            Trap::BadConversion => "bad_conversion",
            Trap::Memory => "memory",
        }
    }

    pub fn from_name(name: &str) -> Option<Trap> {
        Trap::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// Whether `text` is `-?` digits, and with `fraction` also optionally `.` digits (M2-03 1.5).
fn is_number(text: &str, fraction: bool) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let body = text.strip_prefix('-').unwrap_or(text);
    match body.split_once('.') {
        None => digits(body),
        Some((whole, part)) => fraction && digits(whole) && digits(part),
    }
}
