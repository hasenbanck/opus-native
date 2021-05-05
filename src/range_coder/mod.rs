//! Implements the range coder.
use std::mem::size_of;

#[cfg(feature = "decoder")]
pub(crate) use decoder::RangeDecoder;
#[cfg(feature = "encoder")]
pub(crate) use encoder::RangeEncoder;

#[cfg(feature = "decoder")]
mod decoder;
#[cfg(feature = "encoder")]
mod encoder;

/// The number of bits to use for the range-coded part of unsigned integers.
const UINT_BITS: u32 = 8;
/// The resolution of fractional-precision bit usage measurements, i.e., 3 => 1/8th bits.
const BITRES: u32 = 3;
/// Must be at least 32 bits, but if you have fast arithmetic on a larger type,
/// you can speed up the decoder by using it here.
const WINDOW_SIZE: u32 = 32;
/// The number of bits to output at a time.
const SYM_BITS: u32 = 8;
/// The total number of bits in each of the state registers.
const CODE_BITS: u32 = 32;
/// The maximum symbol value.
const SYM_MAX: u32 = (1 << SYM_BITS) - 1;
/// Bits to shift by to move a symbol into the high-order position.
const CODE_SHIFT: u32 = CODE_BITS - SYM_BITS - 1;
/// Carry bit of the high-order range symbol.
const CODE_TOP: u32 = 1 << (CODE_BITS - 1);
/// Low-order bit of the high-order range symbol.
const CODE_BOT: u32 = CODE_TOP >> SYM_BITS;
/// The number of bits available for the last, partial symbol in the code field.
const CODE_EXTRA: u32 = (CODE_BITS - 2) % SYM_BITS + 1;

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
        (size_of::<u32>() * 8) as u32 - x.leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use std::f64::consts::LOG2_E;

    use nanorand::RNG;

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

    fn ldexp(x: f64, exp: f64) -> f64 {
        x * 2.0f64.powf(exp)
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

    #[test]
    #[rustfmt::skip]
    fn test_tell_frac_limits() {
        assert_eq!(TellImpl { bits_total: 0x18, range: 0x800000 }.tell(), 0x0);
        assert_eq!(TellImpl { bits_total: u32::MAX, range: 0x800000 }.tell(), 0xFFFFFFE7);
        assert_eq!(TellImpl { bits_total: 0x20, range: u32::MAX }.tell(), 0x0);
        assert_eq!(TellImpl { bits_total: u32::MAX, range: u32::MAX }.tell(), 0xFFFFFFDF);
    }

    #[test]
    fn test_encoder_uint_bits() {
        let mut entropy: f64 = 0.0;
        let mut nbits: u32;
        let mut nbits2: u32;

        let mut buffer = vec![0_u8; 10 * 1024 * 1024];
        let mut enc = RangeEncoder::new(&mut buffer);

        for ft in 2..1024 {
            for i in 0..ft {
                entropy += f64::ln(ft as f64) * LOG2_E;
                enc.encode_uint(i, ft).unwrap();
            }
        }

        for ftb in 1..16 {
            for i in 0..(1 << ftb) {
                entropy += ftb as f64;
                nbits = enc.tell();

                enc.encode_bits(i, ftb).unwrap();
                nbits2 = enc.tell();
                assert_eq!(
                    nbits2 - nbits,
                    ftb,
                    "Used {} bits to encode {} bits directly.",
                    nbits2 - nbits,
                    ftb
                );
            }
        }

        nbits = enc.tell_frac();
        enc.done().unwrap();

        println!(
            "Encoded {:.2} bits of entropy to {:.2} bits ({:.3}% wasted).",
            entropy,
            ldexp(nbits as f64, -3.0),
            100.0 * (nbits as f64 - ldexp(entropy, 3.0)) / nbits as f64
        );
        println!("Packed to {} bytes.", enc.range_bytes());

        drop(enc);
        let mut dec = RangeDecoder::new(&buffer);

        for ft in 2..1024 {
            for i in 0..ft {
                let sym = dec.decode_uint(ft);
                assert_eq!(
                    sym, i,
                    "Decoded {} instead of {} with ft of {}.",
                    sym, i, ft
                );
            }
        }

        for ftb in 1..16 {
            for i in 0..(1 << ftb) {
                let sym = dec.decode_bits(ftb);
                assert_eq!(
                    sym, i,
                    "Decoded {} instead of {} with ftb of {}.",
                    sym, i, ftb
                );
            }
        }

        nbits2 = dec.tell_frac();
        assert_eq!(
            nbits,
            nbits2,
            "Reported number of bits used was {:.2}, should be {:.2}.",
            ldexp(nbits2 as f64, -3.0),
            ldexp(nbits as f64, -3.0)
        );
    }

    /// Testing an encoder bust prefers range coder data over raw bits.
    /// This isn't a general guarantee, will only work for data that is buffered in
    /// the encoder state and not yet stored in the user buffer, and should never
    /// get used in practice.
    /// It's mostly here for code coverage completeness.
    #[test]
    fn test_encoder_prefer_range_coder_data() {
        // Start with a 16-bit buffer.
        let mut buffer = vec![0_u8; 2];
        let mut enc = RangeEncoder::new(&mut buffer);
        // Write 7 raw bits.
        enc.encode_bits(0x55, 7).unwrap();
        // Write 12.3 bits of range coder data.
        enc.encode_uint(1, 2).unwrap();
        enc.encode_uint(1, 3).unwrap();
        enc.encode_uint(1, 4).unwrap();
        enc.encode_uint(1, 5).unwrap();
        enc.encode_uint(2, 6).unwrap();
        enc.encode_uint(6, 7).unwrap();
        enc.done().unwrap();

        drop(enc);
        let mut dec = RangeDecoder::new(&buffer);

        // The raw bits should have been overwritten by the range coder data.
        assert_eq!(dec.decode_bits(7), 0x05);
        // And all the range coder data should have been encoded correctly.
        assert_eq!(dec.decode_uint(2), 1);
        assert_eq!(dec.decode_uint(3), 1);
        assert_eq!(dec.decode_uint(4), 1);
        assert_eq!(dec.decode_uint(5), 1);
        assert_eq!(dec.decode_uint(6), 2);
        assert_eq!(dec.decode_uint(7), 6);
    }

    const DATA_SIZE: usize = 10000;

    #[test]
    fn test_encoder_random() {
        let seed = 42;
        let mut rnd = nanorand::WyRand::new_seed(seed);
        let mut buffer = vec![0_u8; DATA_SIZE];

        for _ in 0..1024 {
            let ft = rnd.generate_range::<u32>(2, 1024);
            let sz = rnd.generate_range::<usize>(128, 512);

            let mut data = vec![0_u32; sz];
            let mut tell = vec![0_u32; sz + 1];

            let mut enc = RangeEncoder::new(&mut buffer);
            let zeros = rnd.generate_range::<u32>(0, 14) == 0;
            tell[0] = enc.tell_frac();
            for j in 0..sz {
                if zeros {
                    data[j] = 0;
                } else {
                    data[j] = rnd.generate_range(0, ft);
                }
                enc.encode_uint(data[j], ft).unwrap();
                tell[j + 1] = enc.tell_frac();
            }
            if rnd.generate_range::<u32>(0, 2) == 0 {
                while enc.tell() % 8 != 0 {
                    enc.encode_uint(rnd.generate_range::<u32>(0, 2), 2).unwrap();
                }
            }
            let tell_bits = enc.tell();
            enc.done().unwrap();

            assert_eq!(
                tell_bits,
                enc.tell(),
                "tell() changed after done(): {} instead of {}",
                enc.tell(),
                tell_bits,
            );

            assert!(
                (tell_bits + 7) / 8 >= enc.range_bytes() as u32,
                "tell() lied, there's {} bytes instead of {}",
                enc.range_bytes(),
                (tell_bits + 7) / 8,
            );

            drop(enc);
            let mut dec = RangeDecoder::new(&buffer);

            assert_eq!(
                dec.tell_frac(),
                tell[0],
                "Tell mismatch between encoder and decoder at symbol {}: {} instead of {}.",
                0,
                dec.tell_frac(),
                tell[0]
            );

            for j in 0..sz {
                let sym = dec.decode_uint(ft);
                assert_eq!(
                    sym, data[j],
                    "Decoded {} instead of {} with ft of {} at position {} of {}",
                    sym, data[j], ft, j, sz
                );
                assert_eq!(
                    dec.tell_frac(),
                    tell[j + 1],
                    "Tell mismatch between encoder and decoder at symbol {}: {} instead of {}",
                    j + 1,
                    dec.tell_frac(),
                    tell[j + 1]
                );
            }
        }
    }
}
