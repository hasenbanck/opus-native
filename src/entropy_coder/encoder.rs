///! Implements the entropy encoder.
use crate::entropy_coder::Tell;

/// The entropy encoder.
pub(crate) struct Encoder {
    /// Buffered output.
    buf: Vec<u8>,
    /// The size of the buffer. // TODO can maybe be refactored
    storage: usize,
    /// The offset at which the last byte containing raw bits was written.  // TODO can maybe be refactored
    end_offs: usize,
    /// Bits that will be written at the end (Integer needs to be at least 32 bit).
    end_window: u32,
    /// Number of valid bits in end_window.
    end_bits: u32,
    /// The total number of whole bits written.
    /// This does not include partial bits currently in the range coder.
    bits_total: u32,
    /// The offset at which the next range coder byte will be written.
    offs: u32,
    /// The number of values in the current range.
    range: u32,
    /// The low end of the current range.
    val: u32,
    /// The number of outstanding carry propagating symbols.
    ext: u32,
    /// A buffered input symbol, awaiting carry propagation.
    rem: u32, // TODO this might need to be a signed value?
}

impl Tell for Encoder {
    #[inline(always)]
    fn bits_total(&self) -> u32 {
        self.bits_total
    }

    #[inline(always)]
    fn range(&self) -> u32 {
        self.range
    }
}
