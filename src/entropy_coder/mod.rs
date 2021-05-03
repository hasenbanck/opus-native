//! Implements the entropy coder (a range decoder / encoder).
pub(crate) use decoder::Decoder;
pub(crate) use encoder::Encoder;

mod decoder;
mod encoder;

/// The resolution of fractional-precision bit usage measurements, i.e., 3 => 1/8th bits.
const BITRES: u32 = 3;

/// Provides common functionality for the range encoder and decoder.
pub(crate) trait Tell {
    /// Mut return the total number of whole bits read or written.
    fn bits_total(&self) -> u32;
    /// Must return the number of values in the current range.
    fn range(&self) -> u32;

    /// Returns the number of bits "used" by the encoded or decoded symbols so far.
    ///
    /// This number can be computed in either the encoder or the decoder, and is
    /// suitable for making coding decisions.
    ///
    /// This will always be slightly larger than the exact value (e.g., all
    /// rounding error is in the positive direction).
    fn tell(&self) -> u32 {
        self.bits_total() - self.log(self.range())
    }

    /// Returns the number of bits "used" by the encoded or decoded symbols so far
    /// scaled by 2**BITRES.
    ///
    /// This same number can be computed in either the encoder or the decoder, and is
    /// suitable for making coding decisions.
    ///
    /// This will always be slightly larger than the exact value (e.g., all
    /// rounding error is in the positive direction).
    #[allow(clippy::as_conversions)]
    fn tell_frac(&self) -> u32 {
        // This is a faster version of the RFC tell_frac() version that takes
        // advantage of the low (1/8 bit) resolution to use just a linear function
        // followed by a lookup to determine the exact transition thresholds.
        let correction = [35733, 38967, 42495, 46340, 50535, 55109, 60097, 65535];
        let bits = self.bits_total() << BITRES;
        let range = self.range();
        let mut l = self.log(range);
        let r = range >> (l - 16);
        let mut b = (r >> 12) - 8;
        if r > correction[b as usize] {
            b += 1;
        }
        l = (l << 3) + b;
        bits - l
    }

    #[inline(always)]
    fn log(&self, x: u32) -> u32 {
        // FIXME use u32::BITS once stabilized
        32 - x.leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use super::*;

    struct TellImpl {
        bits_total: u32,
        range: u32,
    }

    impl Tell for TellImpl {
        #[inline]
        fn bits_total(&self) -> u32 {
            self.bits_total
        }

        #[inline]
        fn range(&self) -> u32 {
            self.range
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_tell() {
        assert_eq!(TellImpl { bits_total: 0x100, range: 0x2C934200 }.tell(), 0xE2);
        assert_eq!(TellImpl { bits_total: 0xA2, range: 0x26B3D280 }.tell(), 0x84);
        assert_eq!(TellImpl { bits_total: 0x6A3, range: 0x2B79000 }.tell(), 0x689);
        assert_eq!(TellImpl { bits_total: 0x20E, range: 0x347D1700 }.tell(), 0x1F0);
        assert_eq!(TellImpl { bits_total: 0x39A, range: 0x896DA00 }.tell(), 0x37E);
        assert_eq!(TellImpl { bits_total: 0x512, range: 0x1E08800 }.tell(), 0x4F9);
        assert_eq!(TellImpl { bits_total: 0x136, range: 0x473B3F00 }.tell(), 0x117);
        assert_eq!(TellImpl { bits_total: 0x4CB, range: 0x1EDAD600 }.tell(), 0x4AE);
        assert_eq!(TellImpl { bits_total: 0x679, range: 0x11653800 }.tell(), 0x65C);
    }

    #[test]
    #[rustfmt::skip]
    fn test_tell_frac() {
        assert_eq!(TellImpl { bits_total: 0x100, range: 0x2C934200 }.tell_frac(), 0x70D);
        assert_eq!(TellImpl { bits_total: 0xA2, range: 0x26B3D280 }.tell_frac(), 0x41E);
        assert_eq!(TellImpl { bits_total: 0x6A3, range: 0x2B79000 }.tell_frac(), 0x3445);
        assert_eq!(TellImpl { bits_total: 0x20E, range: 0x347D1700 }.tell_frac(), 0xF7B);
        assert_eq!(TellImpl { bits_total: 0x39A, range: 0x896DA00 }.tell_frac(), 0x1BF0);
        assert_eq!(TellImpl { bits_total: 0x512, range: 0x1E08800 }.tell_frac(), 0x27C1);
        assert_eq!(TellImpl { bits_total: 0x136, range: 0x473B3F00 }.tell_frac(), 0x8B7);
        assert_eq!(TellImpl { bits_total: 0x4CB, range: 0x1EDAD600 }.tell_frac(), 0x2569);
        assert_eq!(TellImpl { bits_total: 0x679, range: 0x11653800 }.tell_frac(), 0x32E0);
    }
}
