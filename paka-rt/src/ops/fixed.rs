//! `fixed`, by the integer-only algorithm of M2-03 2.6: no host formatter, no float operation.

use super::{Float, Int, Trap};

const LIMB: u128 = 1_000_000_000_000_000_000;

impl Float {
    /// The exact decimal value rounded to `digits` places, exact halves away from zero.
    pub fn fixed(self, digits: Int) -> Result<String, Trap> {
        let digits = usize::try_from(digits.get())
            .ok()
            .filter(|d| *d <= 20)
            .ok_or(Trap::OutOfBounds)?;
        let sign = if self.0.is_sign_negative() { "-" } else { "" };
        if self.0.is_nan() {
            return Ok("NaN".into());
        }
        if self.0.is_infinite() {
            return Ok(format!("{sign}inf"));
        }
        let (m, e) = split(self.0.to_bits());
        let (whole, part) = match u32::try_from(e) {
            Ok(e) => (whole(m, e), "0".repeat(digits)),
            Err(_) => {
                let q = format!("{:0>1$}", scaled(m, digits, -e), digits + 1);
                let (whole, part) = q.split_at(q.len() - digits);
                (whole.into(), part.into())
            }
        };
        let point = if digits == 0 { "" } else { "." };
        Ok(format!("{sign}{whole}{point}{part}"))
    }
}

/// `m` and `e` with `|x| == m * 2^e` for a finite `x`.
fn split(bits: u64) -> (u64, i32) {
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let fraction = bits & ((1 << 52) - 1);
    match exp {
        0 => (fraction, -1074),
        _ => (fraction | (1 << 52), exp - 1075),
    }
}

/// The digits of `m * 2^e`, kept in limbs of 18 digits, least significant first.
fn whole(m: u64, mut e: u32) -> String {
    let mut limbs = vec![u128::from(m)];
    while e > 0 {
        let step = e.min(59);
        e -= step;
        let mut carry = 0;
        for limb in &mut limbs {
            let v = (*limb << step) + carry;
            (*limb, carry) = (v % LIMB, v / LIMB);
        }
        if carry > 0 {
            limbs.push(carry);
        }
    }
    let mut limbs = limbs.iter().rev();
    let top = limbs.next().map(u128::to_string).unwrap_or_default();
    limbs.fold(top, |s, limb| format!("{s}{limb:018}"))
}

/// `m * 10^digits / 2^k` rounded to nearest, an exact half going up.
fn scaled(m: u64, digits: usize, k: i32) -> u128 {
    let n = u128::from(m) * 10u128.pow(digits as u32);
    if k >= 121 {
        return 0;
    }
    let (q, r) = (n >> k, n & ((1 << k) - 1));
    q + u128::from(2 * r >= (1 << k))
}
