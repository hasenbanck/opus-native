//! Implements the Celt decoder.

use crate::celt::Mode;
use crate::{Channels, DecoderError, SamplingRate};

/// The Celt decoder.
#[derive(Clone, Debug)]
pub(crate) struct CeltDecoder {
    mode: Mode,
    // Signaling is only used for custom decoder mode.
}

impl CeltDecoder {
    /// Creates a new Celt decoder.
    pub(crate) fn new(
        _sampling_rate: SamplingRate,
        _channels: Channels,
    ) -> Result<Self, DecoderError> {
        let mode = Mode::default();
        // TODO calculate and set downsample

        Ok(Self { mode })
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
