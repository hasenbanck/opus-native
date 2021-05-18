//! Implements the Celt decoder.

use crate::celt::Mode;
use crate::range_coder::RangeDecoder;
use crate::{Channels, DecoderError, SamplingRate};

/// The Celt decoder.
#[derive(Clone, Debug)]
pub(crate) struct CeltDecoder {
    mode: Mode,
    // Signaling is only used for custom decoder mode.
    // Startband
    start: u32,
    // Endband
    end: u32,
    stream_channels: Channels,

    rng: u32,
}

impl CeltDecoder {
    /// Creates a new Celt decoder.
    pub(crate) fn new(
        _sampling_rate: SamplingRate,
        channels: Channels,
    ) -> Result<Self, DecoderError> {
        let mode = Mode::default();
        // TODO Port opus_custom_decoder_init
        // TODO calculate and set downsample

        Ok(Self {
            mode,
            start: 0,
            end: 21,
            stream_channels: channels,
            rng: 0,
        })
    }

    /// Resets the Celt decoder.
    pub(crate) fn reset(&mut self) -> Result<(), DecoderError> {
        unimplemented!()
    }

    /// Returns the window.
    pub(crate) fn window(&self) -> &[f32] {
        self.mode.window
    }

    /// Gets the pitch of the last decoded frame.
    pub(crate) fn pitch(&self) -> u32 {
        unimplemented!()
    }

    // TODO This is just a test.
    pub(crate) fn decode(
        &self,
        data: &Option<&[u8]>,
        len: usize,
        pcm: &mut [f32],
        frame_size: usize,
        dec: &mut Option<RangeDecoder>,
    ) -> usize {
        unimplemented!()
    }

    /// Get the final range.
    pub(crate) fn final_range(&mut self) -> u32 {
        self.rng
    }

    /// Sets the end band.
    pub(crate) fn set_end_band(&mut self, end_band: u32) {
        self.end = end_band;
    }

    /// Sets the start band.
    pub(crate) fn set_start_band(&mut self, start_band: u32) {
        self.start = start_band;
    }

    /// Sets the stream channels.
    pub(crate) fn set_stream_channels(&mut self, channels: Channels) {
        self.stream_channels = channels;
    }
}
