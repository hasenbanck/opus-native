///! Implements the entropy decoder.
use crate::entropy_coder::Tell;

/// The entropy decoder.
pub(crate) struct Decoder {
    /// Buffered input.
    buf: Vec<u8>,
    /// The size of the buffer. // TODO can maybe be refactored
    storage: usize,
    /// The offset at which the last byte containing raw bits was read.  // TODO can maybe be refactored
    end_offs: usize,
    /// Bits that will be read from at the end (Integer needs to be at least 32 bit).
    end_window: u32,
    /// Number of valid bits in end_window.
    end_bits: u32,
    /// The total number of whole bits read.
    /// This does not include partial bits currently in the range coder.
    bits_total: u32,
    /// The offset at which the next range coder byte will be read.
    offs: u32,
    /// The number of values in the current range.
    range: u32,
    /// The difference between the top of the current range and the input value, minus one.
    val: u32,
    /// The saved normalization factor from ec_decode().
    ext: u32,
    /// A buffered output symbol, awaiting carry propagation.
    rem: u32, // TODO this might need to be a signed value?
}

impl Tell for Decoder {
    #[inline(always)]
    fn bits_total(&self) -> u32 {
        self.bits_total
    }

    #[inline(always)]
    fn range(&self) -> u32 {
        self.range
    }
}
