use super::{Int, Trap, is_number};

const TWO_52: f64 = 4503599627370496.0;
const TWO_63: f64 = 9223372036854775808.0;
const NAN: Float = Float(f64::from_bits(0x7FF8_0000_0000_0000));

/// An IEEE binary64: round to nearest even, no fused operations, no trap (M2-03 1.3).
/// `==` and the order are IEEE's: every compare with a NaN is false, except `!=`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Float(pub(super) f64);

impl Float {
    /// For literals in tests and back ends; not a Paka operation.
    pub fn from_bits(bits: u64) -> Float {
        Float(f64::from_bits(bits))
    }

    pub fn bits(self) -> u64 {
        self.0.to_bits()
    }

    pub(super) fn from_int(n: i64) -> Float {
        Float(n as f64)
    }

    pub fn add(self, other: Float) -> Float {
        Float(self.0 + other.0)
    }

    pub fn sub(self, other: Float) -> Float {
        Float(self.0 - other.0)
    }

    pub fn mul(self, other: Float) -> Float {
        Float(self.0 * other.0)
    }

    pub fn div(self, other: Float) -> Float {
        Float(self.0 / other.0)
    }

    pub fn neg(self) -> Float {
        Float(-self.0)
    }

    pub fn abs(self) -> Float {
        Float(self.0.abs())
    }

    pub fn sqrt(self) -> Float {
        Float(self.0.sqrt())
    }

    pub fn floor(self) -> Float {
        Float(self.0.floor())
    }

    pub fn ceil(self) -> Float {
        Float(self.0.ceil())
    }

    pub fn trunc(self) -> Float {
        Float(self.0.trunc())
    }

    /// Ties away from zero, exactly: `floor(a + 0.5)` is wrong for 0.49999999999999994.
    pub fn round(self) -> Float {
        let a = self.0;
        if a.abs() >= TWO_52 {
            return self;
        }
        let t = a.trunc();
        Float(match (a - t).abs() >= 0.5 {
            true if a > 0.0 => t + 1.0,
            true => t - 1.0,
            false => t,
        })
    }

    /// A NaN gives NaN; `-0.0` is below `0.0`.
    pub fn min(self, other: Float) -> Float {
        let (a, b) = (self.0, other.0);
        if a.is_nan() || b.is_nan() {
            NAN
        } else if a < b || a == b && a.is_sign_negative() {
            self
        } else {
            other
        }
    }

    pub fn max(self, other: Float) -> Float {
        let (a, b) = (self.0, other.0);
        if a.is_nan() || b.is_nan() {
            NAN
        } else if a > b || a == b && !a.is_sign_negative() {
            self
        } else {
            other
        }
    }

    /// `trunc` as an Int; NaN and values outside the Int range trap.
    pub fn to_int(self) -> Result<Int, Trap> {
        if (-TWO_63..TWO_63).contains(&self.0) {
            Ok(Int::new(self.0 as i64))
        } else {
            Err(Trap::BadConversion)
        }
    }

    /// `parse_float`: `-?` digits, optionally `.` digits; the nearest Float, ties to even.
    pub fn parse(text: &str) -> Option<Float> {
        is_number(text, true)
            .then(|| text.parse().ok())
            .flatten()
            .map(Float)
    }
}
