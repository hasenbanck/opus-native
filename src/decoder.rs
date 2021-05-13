//! Implement the Opus decoder.

use crate::celt::CeltDecoder;
use crate::silk::SilkDecoder;
use crate::{Bandwidth, Channels, CodecMode, DecoderError, Sample, SamplingRate};

/// Configures the decoder on creation.
///
/// Internally Opus stores data at 48000 Hz, so that should be the default
/// value for the sampling rate. However, the decoder can efficiently decode
/// to buffers at 8, 12, 16, and 24 kHz so if for some reason the caller cannot
/// use data at the full sample rate, or knows the compressed data doesn't
/// use the full frequency range, it can request decoding at a reduced
/// rate. Likewise, the decoder is capable of filling in either mono or
/// interleaved stereo pcm buffers, at the caller's request.
#[derive(Clone, Debug)]
pub struct DecoderConfiguration {
    /// Sample rate to decode at (Hz). Default: 48000 kHz.
    pub sampling_rate: SamplingRate,
    /// Number of channels to decode. Default: Stereo.
    pub channels: Channels,
    /// Scales the decoded output by a factor specified in Q8 dB units. Default: 0.
    pub gain: i16,
}

impl Default for DecoderConfiguration {
    fn default() -> Self {
        Self {
            sampling_rate: SamplingRate::Hz48000,
            channels: Channels::Stereo,
            gain: 0,
        }
    }
}

/// Opus decoder.
///
/// Opus is a stateful codec with overlapping blocks and as a result Opus
/// packets are not coded independently of each other. Packets must be
/// passed into the decoder serially and in the correct order for a correct
/// decode. Lost packets can be replaced with loss concealment by calling
/// the decoder with `None` for the missing packet.
#[derive(Clone, Debug)]
pub struct Decoder {
    celt_dec: CeltDecoder,
    silk_dec: SilkDecoder,
    channels: Channels,
    sampling_rate: SamplingRate,
    decode_gain: i16,

    stream_channels: Channels,
    bandwidth: Bandwidth,
    mode: Option<CodecMode>,
    prev_mode: Option<CodecMode>,
    frame_size: usize,
    prev_redundancy: Option<usize>,
    last_packet_duration: Option<u32>,

    final_range: u32,
}

impl Decoder {
    /// Creates a new `Decoder` with the given configuration.
    pub fn new(configuration: &DecoderConfiguration) -> Result<Self, DecoderError> {
        let celt_dec = CeltDecoder::new(configuration.sampling_rate, configuration.channels)?;
        let silk_dec = SilkDecoder::new(configuration.sampling_rate, configuration.channels)?;

        Ok(Self {
            celt_dec,
            silk_dec,
            sampling_rate: configuration.sampling_rate,
            channels: configuration.channels,
            decode_gain: configuration.gain,
            stream_channels: configuration.channels,
            bandwidth: Bandwidth::Auto,
            mode: None,
            prev_mode: None,
            frame_size: configuration.sampling_rate as usize / 400,
            prev_redundancy: None,
            last_packet_duration: None,

            final_range: 0,
        })
    }

    /// Resets the Decoder to be equivalent to a freshly initialized decoder.
    ///
    /// This should be called when switching streams in order to prevent
    /// the back to back decoding from giving different results from
    /// one at a time decoding.
    pub fn reset(&mut self) -> Result<(), DecoderError> {
        self.silk_dec.reset()?;
        self.celt_dec.reset()?;

        self.stream_channels = self.channels;
        self.bandwidth = Bandwidth::Auto;
        self.mode = None;
        self.prev_mode = None;
        self.frame_size = self.sampling_rate as usize / 400;
        self.prev_redundancy = None;
        self.last_packet_duration = None;

        Ok(())
    }

    /// Returns the sampling rate the decoder was initialized with.
    pub fn sampling_rate(&self) -> SamplingRate {
        self.sampling_rate
    }

    /// Returns the channels the decoder was initialized with.
    pub fn channels(&self) -> Channels {
        self.channels
    }

    /// Returns the amount to scale PCM signal by in Q8 dB units.
    pub fn gain(&self) -> i16 {
        self.decode_gain
    }

    /// Returns the decoder's last bandpass.
    pub fn bandwidth(&self) -> Bandwidth {
        self.bandwidth
    }

    /// Returns the pitch of the last decoded frame, measured in samples at 48 kHz
    pub fn pitch(&self) -> Option<u32> {
        if let Some(prev_mode) = self.prev_mode {
            match prev_mode {
                CodecMode::Celt => Some(self.celt_dec.pitch()),
                CodecMode::Silk | CodecMode::Hybrid => Some(self.silk_dec.pitch()),
            }
        } else {
            None
        }
    }

    /// Returns the duration (in samples) of the last packet successfully decoded or concealed.
    pub fn last_packet_duration(&self) -> Option<u32> {
        self.last_packet_duration
    }

    /// Returns the final state of the codec's entropy coder.
    ///
    /// This is used for testing purposes, the encoder and decoder state
    /// should be identical after coding a payload assuming no data
    /// corruption or software bugs).
    pub fn final_range(&mut self) -> u32 {
        self.final_range
    }

    /// Decode an Opus packet with a generic sample output.
    ///
    /// Returns number of decoded samples.
    ///
    /// Caller needs to make sure that the samples buffer has enough space to fit
    /// all samples inside the packet. Call `query_packet_sample_count()` to query
    /// the number of samples inside a packet and resize the buffer if needed.
    ///
    /// The internal format is `f32`. Use `decode_float()` to access it directly.
    ///
    /// # Arguments
    /// * `packet`     - Input payload. Use a `None` to indicate packet loss.
    /// * `samples`    - Output signal encoded as PCM samples (interleaved if 2 channels).
    ///                  Length must be at least `frame_size` * `channels`.
    /// * `frame_size` - Number of samples per channel of available space in a PCM.
    ///                  `frame_size` must be a multiple of 2.5 ms (400 for 48kHz).
    ///                  In the case of PLC (packet==`None`) or FEC (decode_fec=`true`), then
    ///                  `frame_size` needs to be exactly the duration of audio that is missing,
    ///                  otherwise the decoder will not be in the optimal state to decode
    ///                  the next incoming packet.
    /// * `decode_fec` - Request that any in-band forward error correction data be decoded.
    ///                  If no such data is available, the frame is decoded as if it were lost.
    ///
    pub fn decode<S: Sample>(
        _packet: Option<&[u8]>,
        _samples: &mut [S],
        _frame_size: usize,
        _decode_fec: bool,
    ) -> Result<u32, DecoderError> {
        unimplemented!()
    }

    /// Decode an Opus packet with floating point output.
    ///
    /// Returns number of decoded samples.
    ///
    /// Caller needs to make sure that the samples buffer has enough space to fit
    /// all samples inside the packet. Call `query_packet_sample_count()` to query
    /// the number of samples inside a packet and resize the buffer if needed.
    ///
    /// # Arguments
    /// * `packet`     - Input payload. Use a `None` to indicate packet loss.
    /// * `samples`    - Output signal encoded as PCM samples (interleaved if 2 channels).
    ///                  Length is frame_size * channels.
    /// * `frame_size` - Number of samples per channel of available space in a PCM.
    ///                  `frame_size` must be a multiple of 2.5 ms (400 for 48kHz).
    ///                  In the case of PLC (packet==`None`) or FEC (decode_fec=`true`), then
    ///                  `frame_size` needs to be exactly the duration of audio that is missing,
    ///                  otherwise the decoder will not be in the optimal state to decode
    ///                  the next incoming packet.
    /// * `decode_fec` - Request that any in-band forward error correction data be decoded.
    ///                  If no such data is available, the frame is decoded as if it were lost.
    ///
    pub fn decode_float(
        _packet: Option<&[u8]>,
        _samples: &mut [f32],
        _frame_size: usize,
        _decode_fec: bool,
    ) -> Result<u32, DecoderError> {
        unimplemented!()
    }

    /// Returns the samples decoded and the packet_offset (used for multiple streams).
    fn decode_native(
        &mut self,
        _packet: Option<&[u8]>,
        _samples: &mut [f32],
        _frame_size: usize,
        _decode_fec: bool,
        _self_delimited: usize,
        _soft_clip: bool,
    ) -> (u32, usize) {
        unimplemented!()
    }
}
