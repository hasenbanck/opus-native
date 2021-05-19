use std::f32::consts::{LN_2, LOG2_E, PI};
use std::mem::size_of;
use std::ops::{Add, Mul, Sub};

/// The minimum number of bits required to store a positive integer in binary, or 0 for a non-positive integer.
#[inline(always)]
pub(crate) fn ilog(x: u32) -> u32 {
    (size_of::<u32>() * 8) as u32 - x.leading_zeros()
}

/// Fast version for log2.
#[inline(always)]
pub(crate) fn fast_log2(x: f32) -> f32 {
    x.ln() * LOG2_E
}

/// Fast version for exp2.
#[inline(always)]
pub(crate) fn fast_exp2(x: f32) -> f32 {
    (x * LN_2).exp()
}

/// Fast version for atan2.
#[allow(clippy::excessive_precision)]
#[inline(always)]
pub(crate) fn fast_atan2(x: f32, y: f32) -> f32 {
    const A: f32 = 0.43157974;
    const B: f32 = 0.67848403;
    const C: f32 = 0.08595542;
    const E: f32 = PI / 2.0;

    let x2 = x * x;
    let y2 = y * y;

    // For very small values, we don't care about the answer, so we can just return 0.
    if x2 + y2 < 1e-18 {
        return 0.0;
    }

    if x2 < y2 {
        let den = (y2 + B * x2) * (y2 + C * x2);
        -x * y * (y2 + A * x2) / den + (if y < 0.0 { -E } else { E })
    } else {
        let den = (x2 + B * y2) * (x2 + C * y2);
        x * y * (x2 + A * y2) / den + (if y < 0.0 { -E } else { E })
            - (if x * y < 0.0 { -E } else { E })
    }
}

/// This is a cos() approximation designed to be bit-exact on any platform. Bit exactness
/// with this approximation is important because it has an impact on the bit allocation.
#[inline(always)]
pub(crate) fn bitexact_cos(x: i16) -> i16 {
    let x2 = i32::from(x) * i32::from(x);
    let y = ((x2 + 4096) >> 13) as i16;
    1 + (32767 - y) + frac_mul16(y, -7651 + frac_mul16(y, 8277 + frac_mul16(-626, y)))
}

/// This is designed to be bit-exact on any platform.
#[inline(always)]
pub(crate) fn bitexact_log2tan(isin: i32, icos: i32) -> i32 {
    let ls = ilog(isin as u32) as i32;
    let lc = ilog(icos as u32) as i32;

    let icos = (icos << (15 - lc)) as i16;
    let isin = (isin << (15 - ls)) as i16;

    let a = i32::from(frac_mul16(isin, frac_mul16(isin, -2597) + 7932));
    let b = i32::from(frac_mul16(icos, frac_mul16(icos, -2597) + 7932));
    (ls - lc) * (1 << 11) + a - b
}

#[inline(always)]
fn frac_mul16(rhs: i16, lhs: i16) -> i16 {
    let x = i32::from(rhs) * i32::from(lhs);
    ((16384 + x) >> 15) as i16
}

/// Custom complex number implementation.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Complex {
    pub(crate) r: f32,
    pub(crate) i: f32,
}

impl std::ops::Sub<Complex> for Complex {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            r: self.r - rhs.r,
            i: self.i - rhs.i,
        }
    }
}

impl std::ops::Add<Complex> for Complex {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Complex) -> Self::Output {
        Self::Output {
            r: self.r + rhs.r,
            i: self.i + rhs.i,
        }
    }
}

impl std::ops::AddAssign for Complex {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.r += rhs.r;
        self.i += rhs.i;
    }
}

impl std::ops::Mul<Complex> for Complex {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        // (a+bi)(c+di) = (ac−bd) + (ad+bc)i
        let r = self.r * rhs.r - self.i * rhs.i;
        let i = self.r * rhs.i + self.i * rhs.r;
        Self::Output { r, i }
    }
}

impl std::ops::MulAssign<Complex> for Complex {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Complex) {
        let tmp = self.r;

        self.r *= rhs.r;
        self.r -= self.i * rhs.i;

        self.i *= rhs.r;
        self.i += tmp * rhs.i;
    }
}

impl std::ops::Mul<f32> for Complex {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        let r = self.r * rhs;
        let i = self.i * rhs;
        Self::Output { r, i }
    }
}

impl std::ops::MulAssign<f32> for Complex {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.r = self.r * rhs;
        self.i = self.i * rhs;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use std::f32::consts::LOG2_E;

    use super::*;

    // Multiple math related tests that just verify, if the current
    // Rust target passes all requirements.

    #[test]
    fn test_div() {
        (1..=327670).into_iter().for_each(|i| {
            let val = 1.0 / i as f32;
            let prod = val * i as f32;

            assert!(
                (prod - 1.0).abs() <= 0.00025,
                "div failed: 1/{}=`{}` (product = {})",
                i,
                val,
                prod
            );
        });
    }

    #[test]
    fn test_sqrt() {
        let mut i = 1;
        while i < 1000000000 {
            let k = i as f32;
            let val = k.sqrt();
            let ratio = val / k.sqrt();
            assert!(
                (ratio - 1.0).abs() <= 0.0005 && (val - k.sqrt()).abs() <= 2.0,
                "sqrt failed: sqrt({})='{}' (ratio = {})",
                i,
                val,
                ratio
            );
            i += 1 + (i >> 10);
        }
    }

    #[test]
    fn test_log2() {
        let mut x: f32 = 0.001;
        while x < 1677700.0 {
            let error = ((LOG2_E * x.ln()) - fast_log2(x)).abs();
            assert!(error <= 0.0009, "x = {}, error = {}", x, error);
            x += x / 8.0;
        }
    }

    #[test]
    fn test_exp2() {
        let mut x: f32 = -11.0;
        while x < 24.0 {
            let error = (x - LOG2_E * (fast_exp2(x)).ln()).abs();
            assert!(error <= 0.0002, "x = {}, error = {}", x, error);
            x += 0.0007;
        }
    }

    #[test]
    fn test_exp2log2() {
        let mut x: f32 = -11.0;
        while x < 24.0 {
            let error = (x - fast_log2(fast_exp2(x))).abs();
            assert!(error <= 0.001, "x = {}, error = {}", x, error);
            x += 0.0007;
        }
    }

    #[test]
    fn test_bitexact_cos() {
        let mut chk: i32 = 0;
        let mut max_d: i16 = 0;
        let mut last: i16 = 32767;
        let mut min_d: i16 = 32767;

        (64..=16320).into_iter().for_each(|i| {
            let q = bitexact_cos(i);
            chk ^= i32::from(q) * i32::from(i);
            let d = last - q;

            if d > max_d {
                max_d = d
            }
            if d < min_d {
                min_d = d
            }

            last = q;
        });

        assert_eq!(bitexact_cos(64), 32767);
        assert_eq!(bitexact_cos(16320), 200);
        assert_eq!(bitexact_cos(8192), 23171);
        assert_eq!(chk, 89408644);
        assert_eq!(max_d, 5);
        assert_eq!(min_d, 0);
    }

    #[test]
    fn test_bitexact_log2tan() {
        let mut chk: i32 = 0;
        let mut max_d: i32 = 0;
        let mut last: i32 = 15059;
        let mut min_d: i32 = 15059;

        (64..8193).into_iter().for_each(|i| {
            let mid = i32::from(bitexact_cos(i));
            let side = i32::from(bitexact_cos(16384 - i));
            let q = bitexact_log2tan(mid, side);
            chk ^= q * i32::from(i);
            let d = last - q;

            assert_eq!(q, -(bitexact_log2tan(side, mid)));

            if d > max_d {
                max_d = d
            }
            if d < min_d {
                min_d = d
            }

            last = q;
        });

        assert_eq!(chk, 15821257);
        assert_eq!(max_d, 61);
        assert_eq!(min_d, -2);
        assert_eq!(bitexact_log2tan(32767, 200), 15059);
        assert_eq!(bitexact_log2tan(30274, 12540), 2611);
        assert_eq!(bitexact_log2tan(23171, 23171), 0);
    }
}
