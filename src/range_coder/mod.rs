//! Implements the range coder.
//!
//! This is an entropy coder based upon [Mar79], which is itself a
//! rediscovery of the FIFO arithmetic code introduced by [Pas76].
//!
//! It is very similar to arithmetic encoding, except that encoding is done with
//! digits in any base, instead of with bits, and so it is faster when using
//! larger bases (i.e.: a byte).
//!
//! The author claims an average waste of `1/2*log_b(2b)` bits, where `b`
//! is the base, longer than the theoretical optimum, but to my knowledge there
//! is no published justification for this claim.
//!
//! This only seems true when using near-infinite precision arithmetic so that
//! the process is carried out with no rounding errors.
//!
//! An excellent description of implementation details is available at
//! http://www.arturocampos.com/ac_range.html
//!
//! A recent work [MNW98] which proposes several changes to arithmetic
//! encoding for efficiency actually re-discovers many of the principles
//! behind range encoding, and presents a good theoretical analysis of them.
//!
//! End of stream is handled by writing out the smallest number of bits that
//! ensures that the stream will be correctly decoded regardless of the value of
//! any subsequent bits.
//!
//! tell() can be used to determine how many bits were needed to decode
//! all the symbols thus far; other data can be packed in the remaining bits of
//! the input buffer.
//!
//! * Pas76: "Source coding algorithms for fast data compression"
//!          by Richard Clark Pasco (1976).
//!
//! * Mar79: "Range encoding: an algorithm for removing redundancy from a digitised message"
//!          by Martin, G.N.N. (1979)
//!
//! * MNW98: "Arithmetic Coding Revisited"
//!          by Alistair Moffat and Radford Neal and Ian H. Witten (1998).
pub(crate) use decoder::RangeDecoder;
pub(crate) use encoder::RangeEncoder;

use crate::math::Log;

mod decoder;
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
    /// Must return the total number of whole bits read or written.
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
        self.bits_total() - self.range().ilog()
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
        let mut l = range.ilog();
        let r = range >> (l - 16);
        let mut b = (r >> 12) - 8;
        if r > correction[b as usize] {
            b += 1;
        }
        l = (l << 3) + b;
        bits - l
    }
}

fn get_lapace_freq(fs0: u32, decay: u32) -> u32 {
    let ft = 32768 - 32 - fs0;
    (ft * (16384 - decay)) >> 15
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use std::f64::consts::LOG2_E;

    use nanorand::RNG;

    use super::*;

    const DATA_SIZE: usize = 10000;

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
    fn test_simple_uint_bits() {
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

        assert!((entropy - 5777073.343410888).abs() < f64::EPSILON);
        assert!((ldexp(nbits as f64, -3.0) - 5778365.00).abs() < f64::EPSILON);
        assert_eq!(enc.range_bytes(), 497192);

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
    fn test_encoder_prefers_range_coder_data() {
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

    #[test]
    fn test_random_data() {
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

    /// Test compatibility between multiple different encode / decode routines.
    #[test]
    fn test_compatibility() {
        let seed = 42;
        let mut rnd = nanorand::WyRand::new_seed(seed);
        let mut buffer = vec![0_u8; DATA_SIZE];

        for _ in 0..1024 {
            let sz = rnd.generate_range::<usize>(128, 512);
            let mut logp1 = vec![0_u32; sz];
            let mut data = vec![0_u32; sz];
            let mut tell = vec![0_u32; sz + 1];
            let mut enc_method = vec![0_u32; sz];

            let mut enc = RangeEncoder::new(&mut buffer);
            tell[0] = enc.tell_frac();
            for j in 0..sz {
                data[j] = rnd.generate_range::<u32>(0, 2);
                logp1[j] = rnd.generate_range::<u32>(1, 17);
                enc_method[j] = rnd.generate_range::<u32>(0, 4);
                match enc_method[j] {
                    0 => {
                        let x = if data[j] != 0 { (1 << logp1[j]) - 1 } else { 0 };
                        let y = if data[j] != 0 { 0 } else { 1 };
                        enc.encode(x, (1 << logp1[j]) - y, 1 << logp1[j]).unwrap();
                    }
                    1 => {
                        let x = if data[j] != 0 { (1 << logp1[j]) - 1 } else { 0 };
                        let y = if data[j] != 0 { 0 } else { 1 };
                        enc.encode_bin(x, (1 << logp1[j]) - y, logp1[j]).unwrap();
                    }
                    2 => {
                        enc.encode_bit_logp(data[j], logp1[j]).unwrap();
                    }
                    3 => {
                        let icdf = [1, 0];
                        enc.encode_icdf(data[j] as usize, &icdf, logp1[j]).unwrap();
                    }
                    _ => panic!("unreachable"),
                }
                tell[j + 1] = enc.tell_frac();
            }
            enc.done().unwrap();

            assert!(
                (enc.tell() + 7) / 8 >= enc.range_bytes() as u32,
                "tell() lied, there's {} bytes instead of {}",
                enc.range_bytes(),
                (enc.tell() + 7) / 8,
            );

            drop(enc);
            let mut dec = RangeDecoder::new(&buffer);

            assert_eq!(
                dec.tell_frac(),
                tell[0],
                "Tell mismatch between encoder and decoder at symbol {}: {} instead of {}",
                0,
                dec.tell_frac(),
                tell[0]
            );

            for j in 0..sz {
                let dec_method = rnd.generate_range::<u32>(0, 4);
                let sym: u32;
                match dec_method {
                    0 => {
                        let fs = dec.decode(1 << logp1[j]);
                        let s = fs >= (1 << logp1[j]) - 1;
                        let x = if s { (1 << logp1[j]) - 1 } else { 0 };
                        let y = if s { 0 } else { 1 };
                        sym = if s { 1 } else { 0 };

                        dec.update(x, (1 << logp1[j]) - y, 1 << logp1[j]);
                    }
                    1 => {
                        let fs = dec.decode_bin(logp1[j]);
                        let s = fs >= (1 << logp1[j]) - 1;
                        let x = if s { (1 << logp1[j]) - 1 } else { 0 };
                        let y = if s { 0 } else { 1 };
                        sym = if s { 1 } else { 0 };

                        dec.update(x, (1 << logp1[j]) - y, 1 << logp1[j]);
                    }
                    2 => {
                        sym = if dec.decode_bit_logp(logp1[j]) { 1 } else { 0 };
                    }
                    3 => {
                        let icdf = [1, 0];
                        sym = dec.decode_icdf(&icdf, logp1[j]);
                    }
                    _ => panic!("unreachable"),
                }
                assert_eq!(
                    sym,
                    data[j],
                    "Decoded {} instead of {} with logp1 of {} at position {} of {}. Encoding method: {}, decoding method: {}",
                    sym,
                    data[j],
                    logp1[j],
                    j,
                    sz,
                    enc_method[j],
                    dec_method
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

    #[test]
    fn test_patch_initial_bits() {
        let mut buffer = vec![0_u8; DATA_SIZE];
        let mut enc = RangeEncoder::new(&mut buffer);
        enc.encode_bit_logp(0, 1).unwrap();
        enc.encode_bit_logp(0, 1).unwrap();
        enc.encode_bit_logp(1, 6).unwrap();
        enc.encode_bit_logp(0, 2).unwrap();
        enc.patch_initial_bits(0, 2).unwrap();
        enc.done().unwrap();

        assert_eq!(enc.range_bytes(), 2);
        drop(enc);

        assert_eq!(
            buffer[0], 63,
            "Got {} when expecting 63 for patch_initial_bits()",
            buffer[0]
        );
    }

    #[test]
    fn test_shrink() {
        let mut buffer = vec![0_u8; DATA_SIZE];
        let mut enc = RangeEncoder::new(&mut buffer);
        enc.encode_uint(1, 255).unwrap();
        enc.encode_uint(2, 255).unwrap();
        enc.encode_uint(3, 255).unwrap();
        enc.encode_uint(4, 255).unwrap();
        enc.done().unwrap();
        enc.shrink(5);
    }

    fn get_start_freq(decay: u32) -> u32 {
        let ft = 32768 - 33;
        let fs = (ft * (16384 - decay)) / (16384 + decay);
        fs + 1
    }

    #[test]
    fn test_laplace() {
        let mut rng = nanorand::WyRand::new_seed(42);
        let mut val = vec![0_i32; 10000];
        let mut decay = vec![0_u32; 10000];
        let mut buffer = vec![0_u8; 40000];
        val[0] = 3;
        val[1] = 0;
        val[2] = -1;
        decay[0] = 6000;
        decay[1] = 5800;
        decay[2] = 5600;

        let mut enc = RangeEncoder::new(&mut buffer);

        (3..10000).into_iter().for_each(|i| {
            val[i] = rng.generate_range::<u32>(0, 16) as i32 - 7;
            decay[i] = rng.generate_range::<u32>(5000, 16000);
        });

        (0..10000).into_iter().for_each(|i| {
            enc.encode_laplace(&mut val[i], get_start_freq(decay[i]), decay[i])
                .unwrap();
        });

        enc.done().unwrap();
        drop(enc);

        let mut dec = RangeDecoder::new(&buffer);

        (0..10000).into_iter().for_each(|i| {
            let d = dec.decode_laplace(get_start_freq(decay[i]), decay[i]);
            assert_eq!(d, val[i], "Got {} instead of {}", d, val[i]);
        });
    }
}
