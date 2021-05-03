///! Implements the entropy encoder.
use crate::entropy_coder::Tell;

/// A entropy encoder. Called "range encoder" in the specification.
///
/// See the `EntropyDecoder` documentation and RFC 6716 for implementation details.
///
/// [RFC6716](https://tools.ietf.org/html/rfc6716)
pub(crate) struct EntropyEncoder<'e> {
    /// Buffered output.
    buffer: &'e mut Vec<u8>,
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
    offs: usize,
    /// The number of values in the current range.
    range: u32,
    /// The low end of the current range.
    // TODO can this be u8?
    val: u32,
    /// The number of outstanding carry propagating symbols.
    // TODO can this be u8?
    ext: u32,
    /// A buffered output symbol, awaiting carry propagation.
    // TODO does this need to be wider?
    rem: u8,
}

impl<'e> Tell for EntropyEncoder<'e> {
    #[inline(always)]
    fn bits_total(&self) -> u32 {
        self.bits_total
    }

    #[inline(always)]
    fn range(&self) -> u32 {
        self.range
    }
}

// TODO implement the encoder.
