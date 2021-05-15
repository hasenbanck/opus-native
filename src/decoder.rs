//! Implement the Opus decoder.

use std::num::NonZeroUsize;

use crate::celt::CeltDecoder;
use crate::silk::SilkDecoder;
use crate::DecoderError::FrameSizeTooSmall;
use crate::{
    parse_packet, pcm_soft_clip, query_packet_bandwidth, query_packet_channel_count,
    query_packet_codec_mode, query_packet_sample_count, query_packet_samples_per_frame, Bandwidth,
    Channels, CodecMode, DecoderError, Sample, SamplingRate,
};

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
    out_buffer: Vec<f32>,
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
    last_packet_duration: Option<usize>,
    // 48 x 2.5 ms = 120 ms
    frame_sizes: [usize; 48],
    softclip_mem: [f32; 2],

    final_range: u32,
}

impl Decoder {
    /// Creates a new `Decoder` with the given configuration.
    pub fn new(configuration: &DecoderConfiguration) -> Result<Self, DecoderError> {
        let celt_dec = CeltDecoder::new(configuration.sampling_rate, configuration.channels)?;
        let silk_dec = SilkDecoder::new(configuration.sampling_rate, configuration.channels)?;

        Ok(Self {
            out_buffer: vec![],
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
            frame_sizes: [0_usize; 48],
            softclip_mem: [0f32; 2],
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
        self.frame_sizes = [0_usize; 48];
        self.softclip_mem = [0f32; 2];

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
    pub fn last_packet_duration(&self) -> Option<usize> {
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
    /// Returns number of decoded samples for one channel.
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
        &mut self,
        packet: Option<&[u8]>,
        samples: &mut [S],
        frame_size: NonZeroUsize,
        decode_fec: bool,
    ) -> Result<usize, DecoderError> {
        let mut frame_size = frame_size.get();
        if !decode_fec {
            if let Some(packet) = packet {
                let sample_count = query_packet_sample_count(&packet, self.sampling_rate)?;
                if sample_count == 0 {
                    return Err(DecoderError::InvalidPacket);
                }
                frame_size = usize::min(frame_size, sample_count);
            }
        }

        let size = frame_size * self.channels as usize;
        if self.out_buffer.len() < size {
            self.out_buffer.resize(size, 0_f32);
        }

        let (sample_count, _) =
            self.decode_native(&packet, &mut None, frame_size, decode_fec, false, true)?;

        if sample_count > 0 {
            (0..sample_count * self.channels as usize)
                .into_iter()
                .for_each(|i| {
                    samples[i] = S::from_f32(self.out_buffer[i]);
                });
        }

        Ok(sample_count)
    }

    /// Decode an Opus packet with floating point output.
    ///
    /// Returns number of decoded samples for one channel.
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
        &mut self,
        packet: Option<&[u8]>,
        samples: &mut [f32],
        frame_size: NonZeroUsize,
        decode_fec: bool,
    ) -> Result<usize, DecoderError> {
        let (sample_count, _) = self.decode_native(
            &packet,
            &mut Some(samples),
            frame_size.get(),
            decode_fec,
            false,
            false,
        )?;
        Ok(sample_count)
    }

    /// Returns the samples decoded and the packet_offset (used for multiple streams).
    fn decode_native(
        &mut self,
        packet: &Option<&[u8]>,
        samples: &mut Option<&mut [f32]>,
        frame_size: usize,
        decode_fec: bool,
        self_delimited: bool,
        soft_clip: bool,
    ) -> Result<(usize, usize), DecoderError> {
        // The frame_size has to be to have a multiple of 2.5 ms.
        if frame_size % (self.sampling_rate as usize / 400) != 0 {
            return Err(DecoderError::BadArguments(
                "frame_size must be a multiple of 2.5 ms of the sampling rate",
            ));
        }

        if let Some(packet) = packet {
            if packet.is_empty() {
                return Err(DecoderError::BadArguments("packet is empty"));
            }
            let mut offset = 0;
            let mut packet_offset = 0;

            let packet_mode = query_packet_codec_mode(packet);
            let packet_bandwidth = query_packet_bandwidth(packet);
            let packet_frame_size = query_packet_samples_per_frame(packet, self.sampling_rate);
            let packet_stream_channels = query_packet_channel_count(packet);

            let count = parse_packet(
                packet,
                false,
                None,
                &mut self.frame_sizes,
                Some(&mut offset),
                Some(&mut packet_offset),
            )?;

            if decode_fec {
                // If no FEC can be present, run the PLC (recursive call).
                if frame_size < packet_frame_size
                    || packet_mode == CodecMode::Celt
                    || self.mode == Some(CodecMode::Celt)
                {
                    return self.decode_native(&None, samples, frame_size, false, false, soft_clip);
                }

                // Otherwise, run the PLC on everything except the size for which we might have FEC.
                let duration_copy = self.last_packet_duration;
                if frame_size - packet_frame_size != 0 {
                    let sample_count = match self.decode_native(
                        &None,
                        samples,
                        frame_size - packet_frame_size,
                        false,
                        false,
                        soft_clip,
                    ) {
                        Ok((sample_count, ..)) => sample_count,
                        Err(err) => {
                            self.last_packet_duration = duration_copy;
                            return Err(err);
                        }
                    };
                    debug_assert_eq!(sample_count, frame_size - packet_frame_size);
                }

                // Complete with FEC.
                self.mode = Some(packet_mode);
                self.bandwidth = packet_bandwidth;
                self.frame_size = packet_frame_size;
                self.stream_channels = packet_stream_channels;

                let sample_count = self.decode_frame(
                    &Some(&packet[offset + self.frame_sizes[0]..]),
                    &samples,
                    (frame_size - packet_frame_size) * self.channels as usize,
                    packet_frame_size,
                    true,
                )?;
                self.last_packet_duration = Some(frame_size);

                Ok((frame_size, packet_offset))
            } else {
                if count * packet_frame_size > frame_size {
                    return Err(FrameSizeTooSmall);
                }

                self.mode = Some(packet_mode);
                self.bandwidth = packet_bandwidth;
                self.frame_size = packet_frame_size;
                self.stream_channels = packet_stream_channels;

                // Update the state as the last step to avoid updating it on an invalid packet.
                let mut sample_count = 0;
                (0..count).into_iter().try_for_each(|i| {
                    let count = self.decode_frame(
                        &Some(&packet[offset + self.frame_sizes[i]..]),
                        samples,
                        sample_count * self.channels as usize,
                        frame_size - sample_count,
                        false,
                    )?;
                    debug_assert_eq!(count, packet_frame_size);

                    offset += self.frame_sizes[i];
                    sample_count += count;
                    Ok::<(), DecoderError>(())
                });

                self.last_packet_duration = Some(sample_count);
                if soft_clip {
                    if let Some(samples) = samples.as_mut() {
                        pcm_soft_clip(
                            &mut samples[..sample_count],
                            self.channels as usize,
                            &mut self.softclip_mem,
                        );
                    } else {
                        pcm_soft_clip(
                            &mut self.out_buffer[..sample_count],
                            self.channels as usize,
                            &mut self.softclip_mem,
                        );
                    }
                } else {
                    self.softclip_mem[0] = 0.0;
                    self.softclip_mem[1] = 0.0;
                }

                Ok((sample_count, packet_offset))
            }
        } else {
            let mut sample_count = 0;
            while sample_count < frame_size {
                let count = self.decode_frame(
                    &None,
                    &samples,
                    sample_count * self.channels as usize,
                    frame_size - sample_count,
                    false,
                )?;
                sample_count += count;
            }
            debug_assert_eq!(sample_count, frame_size);
            self.last_packet_duration = Some(sample_count);
            Ok((sample_count, 0))
        }
    }

    fn decode_frame(
        &mut self,
        packet: &Option<&[u8]>,
        samples: &Option<&mut [f32]>,
        sample_offset: usize,
        frame_size: usize,
        decode_fec: bool,
    ) -> Result<usize, DecoderError> {
        unimplemented!()
    }
}
