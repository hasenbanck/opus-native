//! Implements the range decoder.

use crate::math::Log;
use crate::range_coder::{
    Tell, CODE_BITS, CODE_BOT, CODE_EXTRA, CODE_TOP, SYM_BITS, SYM_MAX, UINT_BITS, WINDOW_SIZE,
};

/// The range decoder.
///
/// This is an entropy decoder based upon Mar79, which is itself a
/// rediscovery of the FIFO arithmetic code introduced by Pas76.
///
/// It is very similar to arithmetic encoding, except that encoding is done with
/// digits in any base, instead of with bits, and so it is faster when using
/// larger bases (i.e.: a byte).
///
/// The author claims an average waste of `1/2*log_b(2b)` bits, where `b`
/// is the base, longer than the theoretical optimum, but to my knowledge there
/// is no published justification for this claim.
///
/// This only seems true when using near-infinite precision arithmetic so that
/// the process is carried out with no rounding errors.
///
/// An excellent description of implementation details is available at
/// http://www.arturocampos.com/ac_range.html
///
/// A recent work MNW98 which proposes several changes to arithmetic
/// encoding for efficiency actually re-discovers many of the principles
/// behind range encoding, and presents a good theoretical analysis of them.
///
/// End of stream is handled by writing out the smallest number of bits that
/// ensures that the stream will be correctly decoded regardless of the value of
/// any subsequent bits.
///
/// tell() can be used to determine how many bits were needed to decode
/// all the symbols thus far; other data can be packed in the remaining bits of
/// the input buffer.
///
/// * Pas76: "Source coding algorithms for fast data compression"
///          by Richard Clark Pasco (1976).
///
/// * Mar79: "Range encoding: an algorithm for removing redundancy from a digitised message"
///          by Martin, G.N.N. (1979)
///
/// * MNW98: "Arithmetic Coding Revisited"
///          by Alistair Moffat and Radford Neal and Ian H. Witten (1998).
pub(crate) struct RangeDecoder<'d> {
    /// Buffered input.
    buffer: &'d [u8],
    /// The offset at which the last byte containing raw bits was read.
    end_offs: usize,
    /// Bits that will be read from at the end.
    end_window: u32,
    /// Number of valid bits in end_window.
    end_bits: u32,
    /// The total number of whole bits read.
    /// This does not include partial bits currently in the range coder.
    bits_total: u32,
    /// The offset at which the next range coder byte will be read.
    offs: usize,
    /// The number of values in the current range.
    rng: u32,
    /// The difference between the top of the current range and the input value, minus one.
    val: u32,
    /// The saved normalization factor from decode().
    ext: u32,
    /// A buffered input symbol, awaiting carry propagation.
    rem: u8,
}

impl<'d> Tell for RangeDecoder<'d> {
    #[inline(always)]
    fn bits_total(&self) -> u32 {
        self.bits_total
    }

    #[inline(always)]
    fn range(&self) -> u32 {
        self.rng
    }
}

impl<'d> RangeDecoder<'d> {
    /// Creates a new decoder from the given buffer.
    pub(crate) fn new(buffer: &'d [u8]) -> Self {
        // This is the offset from which tell() will subtract partial bits.
        // The final value after the normalize() call will be the same as in
        // the encoder, but we have to compensate for the bits that are added there.
        let bits_total = CODE_BITS + 1 - ((CODE_BITS - CODE_EXTRA) / SYM_BITS) * SYM_BITS;
        let rng = 1 << CODE_EXTRA;

        let mut dec = Self {
            buffer,
            end_offs: 0,
            end_window: 0,
            end_bits: 0,
            bits_total,
            offs: 0,
            rng,
            val: 0,
            ext: 0,
            rem: 0,
        };

        dec.rem = dec.read_byte();
        dec.val = rng - 1 - (u32::from(dec.rem) >> (SYM_BITS - CODE_EXTRA));

        // Normalize the interval.
        dec.normalize();

        dec
    }

    /// Reads the next byte from the start of the buffer.
    fn read_byte(&mut self) -> u8 {
        if self.offs < self.buffer.len() {
            let b = self.buffer[self.offs];
            self.offs += 1;
            b
        } else {
            0
        }
    }

    /// Reads the next byte from the end of the buffer.
    fn read_byte_from_end(&mut self) -> u8 {
        let size = self.buffer.len();
        if self.end_offs < size {
            self.end_offs += 1;
            self.buffer[size - self.end_offs]
        } else {
            0
        }
    }

    /// Normalizes the contents of val and range so that range lies entirely
    /// in the high-order symbol.
    fn normalize(&mut self) {
        // If the range is too small, rescale it and input some bits.
        while self.rng <= CODE_BOT {
            self.bits_total += SYM_BITS;
            self.rng <<= SYM_BITS;
            // Use up the remaining bits from our last symbol.
            let mut symbol = u32::from(self.rem);
            // Read the next value from the input.
            self.rem = self.read_byte();
            // Take the rest of the bits we need from this new symbol.
            symbol = (symbol << SYM_BITS | u32::from(self.rem)) >> (SYM_BITS - CODE_EXTRA);
            // And subtract them from val, capped to be less than CODE_TOP.
            self.val = ((self.val << SYM_BITS) + (SYM_MAX & !symbol)) & (CODE_TOP - 1);
        }
    }

    /// Calculates the cumulative frequency for the next symbol.
    ///
    /// # Arguments
    /// * `ft` - The total frequency of the symbols in the alphabet the
    ///          next symbol was encoded with.
    ///
    /// Returns the cumulative frequency representing the encoded symbol.
    ///
    /// This can then be fed into the probability model to determine what that
    /// symbol is, and the additional frequency information required to advance to
    /// the next symbol.
    ///
    /// This function cannot be called more than once without a corresponding call to
    /// update(), or decoding will not proceed correctly.
    ///
    /// If the cumulative frequency of all the symbols before the one that
    /// was encoded was fl, and the cumulative frequency of all the symbols
    /// up to and including the one encoded is fh, then the returned value
    /// will fall in the range [fl..fh].
    pub(crate) fn decode(&mut self, ft: u32) -> u32 {
        self.ext = self.rng / ft;
        let s = self.val / self.ext;
        ft - u32::min(s + 1, ft)
    }

    /// Equivalent to decode() with ft == 1 << bits.
    pub(crate) fn decode_bin(&mut self, bits: u32) -> u32 {
        self.ext = self.rng >> bits;
        let s = self.val / self.ext;
        (1 << bits) - u32::min(s + 1, 1 << bits)
    }

    /// Advance the decoder past the next symbol using the frequency information the
    /// symbol was encoded with.
    ///
    /// Exactly one call to decode() must have been made so that all necessary
    /// intermediate calculations are performed.
    ///
    /// # Arguments
    /// * `fl` - The cumulative frequency of all symbols that come before the symbol
    ///          decoded.
    /// * `fh` - The cumulative frequency of all symbols up to and including the symbol
    ///          decoded. Together with fl, this defines the range [fl,fh) in which the
    ///          value returned above must fall.
    /// * `ft` - The total frequency of the symbols in the alphabet the symbol decoded
    ///          was encoded in. This must be the same as passed to the preceding call
    ///          to decode().
    ///
    pub(crate) fn update(&mut self, fl: u32, fh: u32, ft: u32) {
        let s = self.ext * (ft - fh);
        self.val -= s;
        self.rng = if fl > 0 {
            self.ext * (fh - fl)
        } else {
            self.rng - s
        };
        self.normalize();
    }

    /// Decode a bit that has a `1/(1<<logp)` probability of being a one.
    pub(crate) fn decode_bit_logp(&mut self, logp: u32) -> bool {
        let r = self.rng;
        let d = self.val;
        let s = r >> logp;
        let ret = d < s;
        if !ret {
            self.val = d - s
        };
        self.rng = if ret { s } else { r - s };
        self.normalize();
        ret
    }

    /// Decodes a symbol given an "inverse" CDF table.
    ///
    ///
    /// No call to update() is necessary after this call.
    ///
    /// # Arguments
    /// * `icdf` - The "inverse" CDF, such that symbol `s` falls in the range
    ///            `[s>0?ft-icdf[s-1]:0..ft-icdf[s]]`, where `ft = 1 << ftb`.
    ///            The values must be monotonically non-increasing, and the last
    ///            value must be 0.
    /// * `ftb`  - The number of bits of precision in the cumulative distribution.
    ///
    /// Returns the decoded symbol `s`.
    pub(crate) fn decode_icdf(&mut self, icdf: &[u8], ftb: u32) -> u32 {
        let mut s = self.rng;
        let d = self.val;
        let r = s >> ftb;

        let mut t: u32;
        let mut ret: u32 = 0;
        loop {
            t = s;
            s = r * u32::from(icdf[ret as usize]);

            if d >= s {
                break;
            }
            ret += 1;
        }

        self.val = d - s;
        self.rng = t - s;
        self.normalize();

        ret
    }

    /// Extracts a raw unsigned integer with a non-power-of-2 range from the stream.
    ///
    /// The bits must have been encoded with uint().
    ///
    /// No call to update() is necessary after this call.
    ///
    /// # Arguments
    /// * `ft` - The number of integers that can be decoded (one more than the max).
    ///          This must be at least 2, and no more than 2**32-1.
    ///
    /// Returns the decoded bits.
    pub(crate) fn decode_uint(&mut self, mut ft: u32) -> u32 {
        debug_assert!(ft > 1);
        ft -= 1;
        let mut ftb = ft.ilog();
        if ftb > UINT_BITS {
            ftb -= UINT_BITS;
            let ft1 = (ft >> ftb) + 1;
            let s = self.decode(ft1);
            self.update(s, s + 1, ft1);
            let t = s << ftb | self.decode_bits(ftb);
            if t <= ft {
                return t;
            };
            // The frame is corrupt. The specification allows to saturate to (ft-1) in this case.
            ft
        } else {
            ft += 1;
            let s = self.decode(ft);
            self.update(s, s + 1, ft);
            s
        }
    }

    /// Extracts a sequence of raw bits from the stream.
    ///
    /// The bits must have been encoded with bits().
    ///
    /// No call to update() is necessary after this call.
    ///
    /// # Arguments
    /// * `bits`   - The number of bits to extract. This must be
    ///              between 0 and 25, inclusive.
    ///
    /// Returns the decoded bits.
    pub(crate) fn decode_bits(&mut self, bits: u32) -> u32 {
        debug_assert!(bits <= 25);
        let mut window = self.end_window;
        let mut available = self.end_bits;

        if available < bits {
            loop {
                window |= u32::from(self.read_byte_from_end()) << available;
                available += SYM_BITS;

                if available > WINDOW_SIZE - SYM_BITS {
                    break;
                }
            }
        }

        let ret = window & ((1 << bits) - 1);
        window >>= bits;
        available -= bits;

        self.end_window = window;
        self.end_bits = available;
        self.bits_total += bits;
        ret
    }
}
