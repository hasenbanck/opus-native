//! Implements the Celt decoder.

use crate::{Channels, DecoderError, SamplingRate};

/// The Celt decoder.
#[derive(Clone, Debug)]
pub(crate) struct CeltDecoder {
    // Signaling is only used for custom decoder mode.
}

impl CeltDecoder {
    /// Creates a new Celt decoder.
    pub(crate) fn new(
        _sampling_rate: SamplingRate,
        _channels: Channels,
    ) -> Result<Self, DecoderError> {
        // TODO
        Ok(Self {})
    }

    /// Resets the Celt decoder.
    pub(crate) fn reset(&mut self) -> Result<(), DecoderError> {
        unimplemented!()
    }

    /// Gets the pitch of the last decoded frame.
    pub(crate) fn pitch(&self) -> u32 {
        unimplemented!()
    }
}
