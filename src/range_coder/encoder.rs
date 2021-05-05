///! Implements the range encoder.
use crate::encoder_error::EncoderError;
use crate::range_coder::{
    Tell, CODE_BITS, CODE_BOT, CODE_SHIFT, CODE_TOP, SYM_BITS, SYM_MAX, UINT_BITS, WINDOW_SIZE,
};

/// The range encoder.
///
/// See the `RangeDecoder` documentation and RFC 6716 for implementation details.
///
/// [RFC6716](https://tools.ietf.org/html/rfc6716)
pub(crate) struct RangeEncoder<'e> {
    /// Buffered output.
    buffer: &'e mut [u8],
    /// The size of the currently used region of the buffer.
    storage: usize,
    /// The offset at which the last byte containing raw bits was written.
    end_offs: usize,
    /// Bits that will be written at the end.
    end_window: u32,
    /// Number of valid bits in end_window.
    end_bits: u32,
    /// The total number of whole bits written.
    /// This does not include partial bits currently in the range coder.
    bits_total: u32,
    /// The offset at which the next range coder byte will be written.
    offs: usize,
    /// The number of values in the current range.
    rng: u32,
    /// The low end of the current range.
    val: u32,
    /// The number of outstanding carry propagating symbols.
    ext: u32,
    /// A buffered output symbol, awaiting carry propagation.
    rem: Option<u32>,
}

impl<'e> Tell for RangeEncoder<'e> {
    #[inline(always)]
    fn bits_total(&self) -> u32 {
        self.bits_total
    }

    #[inline(always)]
    fn range(&self) -> u32 {
        self.rng
    }
}

impl<'e> RangeEncoder<'e> {
    /// Creates a new encoder from the given buffer.
    pub(crate) fn new(buffer: &'e mut [u8]) -> Self {
        // This is the offset from which tell() will subtract partial bits.
        let bits_total = CODE_BITS + 1;
        let range = CODE_TOP;
        let storage = buffer.len();

        Self {
            buffer,
            storage,
            end_offs: 0,
            end_window: 0,
            end_bits: 0,
            bits_total,
            offs: 0,
            rng: range,
            val: 0,
            ext: 0,
            rem: None,
        }
    }

    /// Resets the state of the encoder.
    pub(crate) fn reset(&mut self) {
        self.storage = self.buffer.len();
        self.end_offs = 0;
        self.end_window = 0;
        self.end_bits = 0;
        self.bits_total = CODE_BITS + 1;
        self.offs = 0;
        self.rng = CODE_TOP;
        self.val = 0;
        self.ext = 0;
        self.rem = None;
    }

    /// Returns the range of the compressed bytes. Valid after calling `done()`.
    pub fn range_bytes(&self) -> usize {
        self.offs
    }

    /// Writes a byte from front to back.
    fn write_byte(&mut self, value: u8) -> Result<(), EncoderError> {
        if self.offs + self.end_offs >= self.storage {
            return Err(EncoderError::BufferToSmall);
        }
        self.buffer[self.offs] = value;
        self.offs += 1;

        Ok(())
    }

    /// Writes a byte from back to front.
    fn write_byte_at_end(&mut self, value: u8) -> Result<(), EncoderError> {
        if self.offs + self.end_offs >= self.storage {
            return Err(EncoderError::BufferToSmall);
        }
        self.end_offs += 1;
        self.buffer[self.storage - self.end_offs] = value;
        Ok(())
    }

    /// Outputs a symbol, with a carry bit.
    ///
    /// If there is a potential to propagate a carry over several symbols, they are
    /// buffered until it can be determined whether or not an actual carry will occur.
    ///
    /// If the counter for the buffered symbols overflows, then the stream becomes
    /// undecodable.
    ///
    /// This gives a theoretical limit of a few billion symbols in a single packet on
    /// 32-bit systems.
    ///
    /// The alternative is to truncate the range in order to force a carry, but
    /// requires similar carry tracking in the decoder, needlessly slowing it down.
    fn carry_out(&mut self, c: u32) -> Result<(), EncoderError> {
        if c != SYM_MAX {
            // No further carry propagation possible, flush buffer.
            let carry = c >> SYM_BITS;

            // Don't output a byte on the first write.
            // This compare should be taken care of by branch-prediction thereafter.
            if let Some(rem) = self.rem {
                let b = (rem + carry) as u8;
                self.write_byte(b)?
            }

            if self.ext > 0 {
                let sym = ((SYM_MAX + carry) & SYM_MAX) as u8;
                loop {
                    self.write_byte(sym)?;

                    self.ext -= 1;
                    if self.ext == 0 {
                        break;
                    }
                }
            }
            self.rem = Some(c & SYM_MAX);
        } else {
            self.ext += 1;
        }

        Ok(())
    }

    /// Normalizes the contents of val and range so that range lies entirely
    /// in the high-order symbol.
    fn normalize(&mut self) -> Result<(), EncoderError> {
        // If the range is too small, output some bits and rescale it.
        while self.rng <= CODE_BOT {
            self.carry_out(self.val >> CODE_SHIFT)?;
            // Move the next-to-high-order symbol into the high-order position.
            self.val = (self.val << SYM_BITS) & (CODE_TOP - 1);
            self.rng <<= SYM_BITS;
            self.bits_total += SYM_BITS;
        }

        Ok(())
    }

    /// Encodes a symbol given its frequency information.
    ///
    /// The frequency information must be discernible by the decoder, assuming it
    /// has read only the previous symbols from the stream.
    ///
    /// It is allowable to change the frequency information, or even the entire
    /// source alphabet, so long as the decoder can tell from the context of the
    /// previously encoded information that it is supposed to do so as well.
    ///
    /// # Argument  
    /// * `fl` - The cumulative frequency of all symbols that come before the one to be
    ///          encoded.
    /// * `fh` - The cumulative frequency of all symbols up to and including the one to
    ///          be encoded. Together with _fl, this defines the range [_fl,_fh) in
    ///          which the decoded value will fall.
    /// * `ft` - The sum of the frequencies of all the symbols.
    ///
    pub(crate) fn encode(&mut self, fl: u32, fh: u32, ft: u32) -> Result<(), EncoderError> {
        let r = self.rng / ft;
        if fl > 0 {
            self.val += self.rng - (r * (ft - fl));
            self.rng = r * (fh - fl);
        } else {
            self.rng -= r * (ft - fh);
        };
        self.normalize()?;

        Ok(())
    }

    /// Equivalent to encode() with `ft == 1 << bits`.
    pub(crate) fn encode_bin(&mut self, fl: u32, fh: u32, bits: u32) -> Result<(), EncoderError> {
        let r = self.rng >> bits;
        if fl > 0 {
            self.val += self.rng - (r * ((1 << bits) - fl));
            self.rng = r * (fh - fl);
        } else {
            self.rng -= r * ((1 << bits) - fh);
        }
        self.normalize()?;

        Ok(())
    }

    /// Encode a bit that has a `1/(1<<logp)` probability of being a one.
    pub(crate) fn encode_bit_logp(&mut self, val: u32, logp: u32) -> Result<(), EncoderError> {
        let mut r = self.rng;
        let l = self.val;
        let s = r >> logp;
        r -= s;
        if val != 0 {
            self.val = l + r
        };
        self.rng = if val != 0 { s } else { r };
        self.normalize()?;

        Ok(())
    }

    /// Encodes a symbol given an "inverse" CDF table.
    ///
    /// # Arguments
    /// * `s`    - The index of the symbol to encode.
    /// * `icdf` - The "inverse" CDF, such that symbol _s falls in the range
    ///            `[s>0?ft-icdf[s-1]:0..ft-icdf[s]]`, where `ft = 1 << ftb`.
    ///            The values must be monotonically non-increasing, and the last value
    ///            must be 0.
    /// * `ftb`  - The number of bits of precision in the cumulative distribution.
    ///
    pub(crate) fn encode_icdf(
        &mut self,
        s: usize,
        icdf: &[u8],
        ftb: u32,
    ) -> Result<(), EncoderError> {
        let r = self.rng >> ftb;
        if s > 0 {
            self.val += self.rng - (r * u32::from(icdf[s - 1]));
            self.rng = r * u32::from(icdf[s - 1] - icdf[s]);
        } else {
            self.rng -= r * u32::from(icdf[s])
        };
        self.normalize()?;

        Ok(())
    }

    /// Encodes a raw unsigned integer in the stream.
    ///
    /// # Arguments
    /// * `fl` - The integer to encode.
    /// * `ft` - The number of integers that can be encoded (one more than the max).
    ///          This must be at least 2, and no more than 2**32-1.
    pub(crate) fn encode_uint(&mut self, fl: u32, mut ft: u32) -> Result<(), EncoderError> {
        // In order to optimize log(), it is undefined for the value 0.
        debug_assert!(ft > 1);
        ft -= 1;
        let mut ftb = self.log(ft);
        if ftb > UINT_BITS {
            ftb -= UINT_BITS;
            let ft1 = (ft >> ftb) + 1;
            let fl1 = fl >> ftb;
            self.encode(fl1, fl1 + 1, ft1)?;
            self.encode_bits(fl & ((1 << ftb) - 1), ftb)?;
        } else {
            self.encode(fl, fl + 1, ft + 1)?;
        };

        Ok(())
    }

    /// Encodes a sequence of raw bits in the stream.
    ///
    /// # Arguments
    /// * `fl`   - The bits to encode.
    /// * `bits` - The number of bits to encode.
    ///            This must be between 1 and 25, inclusive.
    pub(crate) fn encode_bits(&mut self, fl: u32, bits: u32) -> Result<(), EncoderError> {
        debug_assert!(bits > 0);
        let mut window = self.end_window;
        let mut used = self.end_bits;

        if used + bits > WINDOW_SIZE {
            loop {
                self.write_byte_at_end((window & SYM_MAX) as u8)?;
                window >>= SYM_BITS;
                used -= SYM_BITS;

                if used < SYM_BITS {
                    break;
                }
            }
        }
        window |= fl << used;
        used += bits;
        self.end_window = window;
        self.end_bits = used;
        self.bits_total += bits;

        Ok(())
    }

    /// Overwrites a few bits at the very start of an existing stream, after they
    /// have already been encoded.
    ///
    /// This makes it possible to have a few flags up front, where it is easy for
    /// decoders to access them without parsing the whole stream, even if their
    /// values are not determined until late in the encoding process, without having
    /// to buffer all the intermediate symbols in the encoder.
    ///
    /// In order for this to work, at least _nbits bits must have already been
    /// encoded using probabilities that are an exact power of two.
    ///
    /// The encoder can verify the number of encoded bits is sufficient, but cannot
    /// check this latter condition.
    ///
    /// # Arguments
    /// * `val`   - The bits to encode (in the least _nbits significant bits).
    ///            They will be decoded in order from most-significant to least.
    /// * `nbits` - The number of bits to overwrite.
    ///            This must be no more than 8.
    ///
    pub(crate) fn patch_initial_bits(&mut self, val: u32, nbits: u32) -> Result<(), EncoderError> {
        debug_assert!(nbits <= SYM_BITS);
        let shift = SYM_BITS - nbits;
        let mask = ((1 << nbits) - 1) << shift;
        if self.offs > 0 {
            // The first byte has been finalized.
            self.buffer[0] = ((u32::from(self.buffer[0]) & !mask) | val << shift) as u8;
        } else if let Some(rem) = self.rem {
            // The first byte is still awaiting carry propagation.
            self.rem = Some((rem & !mask) | val << shift);
        } else if self.rng <= (CODE_TOP >> nbits) {
            // The renormalization loop has never been run.
            self.val = (self.val & !(mask << CODE_SHIFT)) | val << (CODE_SHIFT + shift);
        } else {
            return Err(EncoderError::InternalError(
                "the encoder hasn't even encoded nbits of data yet",
            ));
        }

        Ok(())
    }

    /// Compacts the data to fit in the target size.
    ///
    /// This moves up the raw bits at the end of the current buffer so they are at
    /// the end of the new buffer size.
    ///
    /// The caller must ensure that the amount of data that's already been written
    /// will fit in the new size.
    ///
    /// # Arguments
    /// * `len` - The number of bytes in the new buffer.
    ///           This must be large enough to contain the bits already written, and
    ///           must be no larger than the existing size.
    pub(crate) fn shrink(&mut self, len: usize) {
        debug_assert!(self.offs + self.end_offs <= len);
        let start = self.storage - self.end_offs;
        let end = self.storage;
        let dest = len - self.end_offs;

        self.buffer.copy_within(start..end, dest);
        self.storage = len;
    }

    /// Indicates that there are no more symbols to encode.
    ///
    /// All remaining output bytes are flushed to the output buffer.
    ///
    /// `reset()` must be called before the encoder can be used again.
    pub(crate) fn done(&mut self) -> Result<(), EncoderError> {
        // We output the minimum number of bits that ensures that the symbols encoded
        // thus far will be decoded correctly regardless of the bits that follow.
        let mut l: i32 = (CODE_BITS - self.log(self.rng)) as i32;
        let mut mask = (CODE_TOP - 1) >> l;
        let mut end = (self.val + mask) & !mask;
        if (end | mask) >= self.val + self.rng {
            l += 1;
            mask >>= 1;
            end = (self.val + mask) & !mask;
        }
        while l > 0 {
            self.carry_out(end >> CODE_SHIFT)?;
            end = (end << SYM_BITS) & (CODE_TOP - 1);
            l -= SYM_BITS as i32;
        }
        // If we have a buffered byte flush it into the output buffer.
        if self.rem.is_some() || self.ext > 0 {
            self.carry_out(0)?
        };
        // If we have buffered extra bits, flush them as well.
        let mut window = self.end_window;
        let mut used = self.end_bits;
        while used >= SYM_BITS {
            self.write_byte_at_end((window & SYM_MAX) as u8)?;
            window >>= SYM_BITS;
            used -= SYM_BITS;
        }
        // Clear any excess space and add any remaining extra bits to the last byte.
        self.buffer[self.offs..self.storage - self.end_offs]
            .iter_mut()
            .for_each(|x| *x = 0);

        if used > 0 {
            // If there's no range coder data at all, give up.
            if self.end_offs >= self.storage {
                return Err(EncoderError::InternalError("no range coder data"));
            } else {
                l = -l;
                // If we've busted, don't add too many extra bits to the last byte; it
                // would corrupt the range coder data, and that's more important.
                if self.offs + self.end_offs >= self.storage && l < used as i32 {
                    window &= (1 << l) - 1;
                }
                self.buffer[self.storage - self.end_offs - 1] |= window as u8;
            }
        }

        Ok(())
    }
}
