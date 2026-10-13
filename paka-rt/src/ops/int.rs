use super::{Float, Trap, is_number};
use std::fmt;

/// A 64-bit two's-complement integer. Arithmetic traps instead of wrapping (M2-03 1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Int(i64);

fn exact(n: Option<i64>) -> Result<Int, Trap> {
    n.map(Int).ok_or(Trap::Overflow)
}

impl Int {
    pub const MIN: Int = Int(i64::MIN);
    pub const MAX: Int = Int(i64::MAX);

    pub fn new(n: i64) -> Int {
        Int(n)
    }

    pub fn get(self) -> i64 {
        self.0
    }

    pub fn add(self, other: Int) -> Result<Int, Trap> {
        exact(self.0.checked_add(other.0))
    }

    pub fn sub(self, other: Int) -> Result<Int, Trap> {
        exact(self.0.checked_sub(other.0))
    }

    pub fn mul(self, other: Int) -> Result<Int, Trap> {
        exact(self.0.checked_mul(other.0))
    }

    pub fn neg(self) -> Result<Int, Trap> {
        exact(self.0.checked_neg())
    }

    pub fn abs(self) -> Result<Int, Trap> {
        exact(self.0.checked_abs())
    }

    pub fn div(self, other: Int) -> Result<Int, Trap> {
        match other.0 {
            0 => Err(Trap::DivideByZero),
            _ => exact(self.0.checked_div(other.0)),
        }
    }

    pub fn rem(self, other: Int) -> Result<Int, Trap> {
        match other.0 {
            0 => Err(Trap::DivideByZero),
            _ => Ok(Int(self.0.wrapping_rem(other.0))),
        }
    }

    pub fn min(self, other: Int) -> Int {
        Ord::min(self, other)
    }

    pub fn max(self, other: Int) -> Int {
        Ord::max(self, other)
    }

    pub fn wrap_add(self, other: Int) -> Int {
        Int(self.0.wrapping_add(other.0))
    }

    pub fn wrap_sub(self, other: Int) -> Int {
        Int(self.0.wrapping_sub(other.0))
    }

    pub fn wrap_mul(self, other: Int) -> Int {
        Int(self.0.wrapping_mul(other.0))
    }

    pub fn bit_and(self, other: Int) -> Int {
        Int(self.0 & other.0)
    }

    pub fn bit_or(self, other: Int) -> Int {
        Int(self.0 | other.0)
    }

    pub fn bit_xor(self, other: Int) -> Int {
        Int(self.0 ^ other.0)
    }

    pub fn bit_not(self) -> Int {
        Int(!self.0)
    }

    pub fn shift_left(self, count: Int) -> Result<Int, Trap> {
        Ok(Int(self.0 << shift_count(count)?))
    }

    pub fn shift_right(self, count: Int) -> Result<Int, Trap> {
        Ok(Int(self.0 >> shift_count(count)?))
    }

    pub fn to_float(self) -> Float {
        Float::from_int(self.0)
    }

    /// `parse_int`: `-?` digits that fit; leading zeros are fine, nothing is trimmed.
    pub fn parse(text: &str) -> Option<Int> {
        is_number(text, false)
            .then(|| text.parse().ok())
            .flatten()
            .map(Int)
    }
}

fn shift_count(count: Int) -> Result<u32, Trap> {
    u32::try_from(count.0)
        .ok()
        .filter(|c| *c < 64)
        .ok_or(Trap::OutOfBounds)
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
