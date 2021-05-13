//! Implements the Silk decoder.

use crate::{Channels, DecoderError, SamplingRate};

/// The Silk decoder.
#[derive(Clone, Debug)]
pub(crate) struct SilkDecoder {
    sampling_rate: SamplingRate,
    channels: Channels,
    internal_sampling_rate: SamplingRate,
    internal_channels: Channels,
    // TODO silk_decoder_state
}

impl SilkDecoder {
    /// Creates a new Silk decoder. Configures the output sampling rate and output channels.
    pub(crate) fn new(
        sampling_rate: SamplingRate,
        channels: Channels,
    ) -> Result<Self, DecoderError> {
        // TODO
        Ok(Self {
            sampling_rate,
            channels,
            internal_sampling_rate: SamplingRate::Hz48000,
            internal_channels: Channels::Stereo,
        })
    }

    /// Resets the Silk decoder.
    pub(crate) fn reset(&mut self) -> Result<(), DecoderError> {
        unimplemented!()
    }

    /// Gets the pitch of the last decoded frame.
    pub(crate) fn pitch(&self) -> u32 {
        unimplemented!()
    }

    /// Sets the internal channels.
    pub(crate) fn internal_channels(&mut self, internal_channels: Channels) {
        self.internal_channels = internal_channels;
    }

    /// Sets the internal sampling rate.
    pub(crate) fn set_internal_sampling_rate(&mut self, sampling_rate: SamplingRate) {
        self.internal_sampling_rate = sampling_rate;
    }
}
