//! Implement the Opus decoder.

use std::cmp::Ordering;
use std::num::NonZeroUsize;

use crate::celt::CeltDecoder;
use crate::range_coder::{RangeDecoder, Tell};
use crate::silk::{LostFlag, SilkDecoder};
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
    inner: DecoderInner,
    buffer: Vec<f32>,
}

impl Decoder {
    /// Creates a new `Decoder` with the given configuration.
    pub(crate) fn new(configuration: &DecoderConfiguration) -> Result<Self, DecoderError> {
        let inner = DecoderInner::new(configuration)?;
        Ok(Self {
            inner,
            buffer: vec![],
        })
    }

    /// Resets the Decoder to be equivalent to a freshly initialized decoder.
    ///
    /// This should be called when switching streams in order to prevent
    /// the back to back decoding from giving different results from
    /// one at a time decoding.
    pub(crate) fn reset(&mut self) -> Result<(), DecoderError> {
        self.buffer = vec![];
        self.inner.reset()
    }

    /// Returns the sampling rate the decoder was initialized with.
    pub fn sampling_rate(&self) -> SamplingRate {
        self.inner.sampling_rate
    }

    /// Returns the channels the decoder was initialized with.
    pub fn channels(&self) -> Channels {
        self.inner.channels
    }

    /// Returns the amount to scale PCM signal by in Q8 dB units.
    pub fn gain(&self) -> i16 {
        self.inner.decode_gain
    }

    /// Returns the decoder's last bandpass.
    pub fn bandwidth(&self) -> Option<Bandwidth> {
        self.inner.bandwidth
    }

    /// Returns the pitch of the last decoded frame, measured in samples at 48 kHz
    pub fn pitch(&self) -> Option<u32> {
        if let Some(prev_mode) = self.inner.prev_mode {
            match prev_mode {
                CodecMode::CeltOnly => Some(self.inner.celt_dec.pitch()),
                CodecMode::SilkOnly | CodecMode::Hybrid => Some(self.inner.silk_dec.pitch()),
            }
        } else {
            None
        }
    }

    /// Returns the duration (in samples) of the last packet successfully decoded or concealed.
    pub fn last_packet_duration(&self) -> Option<usize> {
        self.inner.last_packet_duration
    }

    /// Returns the final state of the codec's entropy coder.
    ///
    /// This is used for testing purposes, the encoder and decoder state
    /// should be identical after coding a payload assuming no data
    /// corruption or software bugs).
    pub fn final_range(&mut self) -> u32 {
        self.inner.final_range
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
                let sample_count = query_packet_sample_count(&packet, self.inner.sampling_rate)?;
                if sample_count == 0 {
                    return Err(DecoderError::InvalidPacket);
                }
                frame_size = usize::min(frame_size, sample_count);
            }
        }

        let size = frame_size * self.inner.channels as usize;
        if self.buffer.len() < size {
            self.buffer.resize(size, 0_f32);
        }

        let (sample_count, _) = self.inner.decode_native(
            &packet,
            &mut self.buffer,
            frame_size,
            decode_fec,
            false,
            true,
        )?;

        if sample_count != 0 {
            if sample_count > samples.len() {
                return Err(DecoderError::BufferToSmall);
            }

            (0..sample_count * self.inner.channels as usize)
                .into_iter()
                .for_each(|i| {
                    samples[i] = S::from_f32(self.buffer[i]);
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
        let (sample_count, _) = self.inner.decode_native(
            &packet,
            samples,
            frame_size.get(),
            decode_fec,
            false,
            false,
        )?;
        Ok(sample_count)
    }
}

#[derive(Clone, Debug)]
struct DecoderInner {
    celt_dec: CeltDecoder,
    silk_dec: SilkDecoder,
    channels: Channels,
    sampling_rate: SamplingRate,
    decode_gain: i16,

    stream_channels: Channels,
    bandwidth: Option<Bandwidth>,
    mode: Option<CodecMode>,
    prev_mode: Option<CodecMode>,
    frame_size: usize,
    prev_redundancy: bool,
    last_packet_duration: Option<usize>,
    // 48 x 2.5 ms = 120 ms
    frame_sizes: [usize; 48],
    softclip_mem: [f32; 2],

    silk_buffer: Vec<f32>,
    redundant_audio: Vec<f32>,

    final_range: u32,
}

impl DecoderInner {
    fn new(configuration: &DecoderConfiguration) -> Result<Self, DecoderError> {
        let celt_dec = CeltDecoder::new(configuration.sampling_rate, configuration.channels)?;
        let silk_dec = SilkDecoder::new(configuration.sampling_rate, configuration.channels)?;

        Ok(Self {
            celt_dec,
            silk_dec,
            sampling_rate: configuration.sampling_rate,
            channels: configuration.channels,
            decode_gain: configuration.gain,
            stream_channels: configuration.channels,
            bandwidth: None,
            mode: None,
            prev_mode: None,
            frame_size: configuration.sampling_rate as usize / 400,
            prev_redundancy: false,
            last_packet_duration: None,
            frame_sizes: [0_usize; 48],
            softclip_mem: [0f32; 2],
            silk_buffer: vec![],
            redundant_audio: vec![],
            final_range: 0,
        })
    }

    fn reset(&mut self) -> Result<(), DecoderError> {
        self.silk_dec.reset()?;
        self.celt_dec.reset()?;

        self.stream_channels = self.channels;
        self.bandwidth = None;
        self.mode = None;
        self.prev_mode = None;
        self.frame_size = self.sampling_rate as usize / 400;
        self.prev_redundancy = false;
        self.last_packet_duration = None;
        self.frame_sizes = [0_usize; 48];
        self.softclip_mem = [0f32; 2];
        self.silk_buffer = vec![];
        self.redundant_audio = vec![];

        Ok(())
    }

    /// Returns the samples decoded and the packet_offset (used for multiple streams).
    fn decode_native(
        &mut self,
        packet: &Option<&[u8]>,
        samples: &mut [f32],
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
                self_delimited,
                None,
                &mut self.frame_sizes,
                Some(&mut offset),
                Some(&mut packet_offset),
            )?;

            if decode_fec {
                // If no FEC can be present, run the PLC (recursive call).
                if frame_size < packet_frame_size
                    || packet_mode == CodecMode::CeltOnly
                    || self.mode == Some(CodecMode::CeltOnly)
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
                self.bandwidth = Some(packet_bandwidth);
                self.frame_size = packet_frame_size;
                self.stream_channels = packet_stream_channels;

                self.decode_frame(
                    &Some(&packet[offset..offset + self.frame_sizes[0]]),
                    samples,
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
                self.bandwidth = Some(packet_bandwidth);
                self.frame_size = packet_frame_size;
                self.stream_channels = packet_stream_channels;

                // Update the state as the last step to avoid updating it on an invalid packet.
                let mut sample_count = 0;
                (0..count).into_iter().try_for_each(|i| {
                    let count = self.decode_frame(
                        &Some(&packet[offset..offset + self.frame_sizes[i]]),
                        samples,
                        sample_count * self.channels as usize,
                        frame_size - sample_count,
                        false,
                    )?;
                    debug_assert_eq!(count, packet_frame_size);

                    offset += self.frame_sizes[i];
                    sample_count += count;
                    Ok::<(), DecoderError>(())
                })?;

                self.last_packet_duration = Some(sample_count);
                if soft_clip {
                    pcm_soft_clip(
                        &mut samples[..sample_count],
                        self.channels as usize,
                        &mut self.softclip_mem,
                    );
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
                    samples,
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

    #[allow(clippy::excessive_precision)]
    fn decode_frame(
        &mut self,
        packet: &Option<&[u8]>,
        samples: &mut [f32],
        mut sample_offset: usize,
        mut frame_size: usize,
        decode_fec: bool,
    ) -> Result<usize, DecoderError> {
        let mut redundancy = false;
        let mut redundancy_bytes = 0;
        let mut celt_to_silk = false;
        let mut redundant_range: u32 = 0;
        let mut len = packet.map_or(0, |x| x.len()) as u32;

        let f20 = self.sampling_rate as usize / 50;
        let f10 = f20 >> 1;
        let f5 = f10 >> 1;
        let f2_5 = f5 >> 1;
        if frame_size < f2_5 {
            return Err(DecoderError::FrameSizeTooSmall);
        }

        // Payloads of 1 (2 including ToC) or 0 trigger the PLC/DTX.
        let (mut dec, audiosize, mode, bandwidth) = if len <= 1 {
            // In that case, don't conceal more than what the ToC says.
            frame_size = usize::min(frame_size, self.frame_size);

            let mut audiosize = frame_size;
            let mode = self.prev_mode;
            let bandwidth = None;

            if mode.is_none() {
                // If we haven't got any packet yet, all we can do is return zeros.
                (0..audiosize * self.channels as usize)
                    .into_iter()
                    .for_each(|i| {
                        samples[sample_offset + i] = 0.0;
                    });

                return Ok(audiosize);
            }

            // Avoids trying to run the PLC on sizes other than 2.5 (CELT), 5 (CELT), 10, or 20 (e.g. 12.5 or 30 ms).
            match audiosize.cmp(&f20) {
                Ordering::Greater => {
                    while audiosize > 0 {
                        let sample_count = self.decode_frame(
                            &None,
                            samples,
                            sample_offset,
                            usize::min(audiosize, f20),
                            false,
                        )?;
                        sample_offset += sample_count * self.channels as usize;
                        audiosize -= sample_count;
                    }
                    return Ok(frame_size);
                }
                Ordering::Less => {
                    if audiosize > f10 {
                        audiosize = f10;
                    } else if mode != Some(CodecMode::SilkOnly) && audiosize > f5 && audiosize < f10
                    {
                        audiosize = f5;
                    }
                }
                Ordering::Equal => { /* Case not covered by the reference implementation */ }
            }

            (None, audiosize, mode, bandwidth)
        } else {
            let mut dec = packet.as_ref().map(|packet| RangeDecoder::new(packet));
            (dec, self.frame_size, self.mode, self.bandwidth)
        };

        let (mut pcm_transition_silk_size, pcm_transition_celt_size) = if packet.is_some()
            && self.prev_mode.is_some()
            && ((mode == Some(CodecMode::CeltOnly)
                && self.prev_mode != Some(CodecMode::CeltOnly)
                && self.prev_redundancy)
                || (mode != Some(CodecMode::CeltOnly)
                    && self.prev_mode == Some(CodecMode::CeltOnly)))
        {
            let size = f5 * self.channels as usize;
            if mode == Some(CodecMode::CeltOnly) {
                (None, Some(size))
            } else {
                (Some(size), None)
            }
        } else {
            (None, None)
        };

        let mut transition_buffer = pcm_transition_celt_size.map(|size| vec![0_f32; size]);

        if mode == Some(CodecMode::CeltOnly) {
            if let Some(buffer) = transition_buffer.as_mut() {
                self.decode_frame(&None, buffer, 0, usize::min(f5, audiosize), false)?;
            }
        }

        if audiosize > frame_size {
            return Err(DecoderError::FrameSizeTooSmall);
        } else {
            frame_size = audiosize;
        }

        // SILK processing.
        if mode != Some(CodecMode::CeltOnly) {
            let mut silk_frame_size = frame_size * self.channels as usize;
            if silk_frame_size > self.silk_buffer.len() {
                self.silk_buffer.resize(silk_frame_size, 0_f32);
            }

            if self.prev_mode == Some(CodecMode::CeltOnly) {
                self.silk_dec.reset()?;
            }

            // The SILK PLC cannot produce frames of less than 10 ms.
            self.silk_dec.set_payload_size_ms(usize::max(
                10,
                1000 * audiosize / self.sampling_rate as usize,
            ));

            if packet.is_some() {
                self.silk_dec.set_internal_channels(self.stream_channels);
                if mode == Some(CodecMode::SilkOnly) {
                    if bandwidth == Some(Bandwidth::Narrowband) {
                        self.silk_dec
                            .set_internal_sampling_rate(SamplingRate::Hz8000);
                    } else if bandwidth == Some(Bandwidth::Mediumband) {
                        self.silk_dec
                            .set_internal_sampling_rate(SamplingRate::Hz12000);
                    } else if bandwidth == Some(Bandwidth::Wideband) {
                        self.silk_dec
                            .set_internal_sampling_rate(SamplingRate::Hz16000);
                    } else {
                        // TODO can this happen normally? Should we return an error instead?
                        self.silk_dec
                            .set_internal_sampling_rate(SamplingRate::Hz16000);
                        debug_assert!(false);
                    }
                } else {
                    // Hybrid mode.
                    self.silk_dec
                        .set_internal_sampling_rate(SamplingRate::Hz16000);
                }
            }

            let lost_flag = if packet.is_none() {
                LostFlag::Loss
            } else if decode_fec {
                LostFlag::DecodeFec
            } else {
                LostFlag::NoLoss
            };

            let mut decoded_samples = 0;
            while decoded_samples < frame_size {
                // Call SILK decoder.
                let first_frame = decoded_samples == 0;
                if let Err(err) = self.silk_dec.decode(
                    &mut dec,
                    &self.silk_buffer[decoded_samples * self.channels as usize..],
                    &mut silk_frame_size,
                    lost_flag,
                    first_frame,
                ) {
                    // PLC failure should not be fatal.
                    if lost_flag != LostFlag::NoLoss {
                        silk_frame_size = frame_size;
                        (0..frame_size * self.channels as usize)
                            .into_iter()
                            .for_each(|i| {
                                self.silk_buffer[i] = 0.0;
                            });
                    } else {
                        return Err(err);
                    }
                }
                decoded_samples += silk_frame_size;
            }
        }

        if !decode_fec && mode != Some(CodecMode::CeltOnly) {
            if let Some(dec) = dec.as_mut() {
                if dec.tell() + 17 + 20 * (self.mode == Some(CodecMode::Hybrid)) as u32 <= 8 * len {
                    // Check if we have a redundant 0-8 kHz band.
                    if mode == Some(CodecMode::Hybrid) {
                        redundancy = dec.decode_bit_logp(12);
                    } else {
                        redundancy = true;
                    }

                    if redundancy {
                        celt_to_silk = dec.decode_bit_logp(1);
                        // redundancy_bytes will be at least two, in the non-hybrid case due to the ec_tell() check above.
                        redundancy_bytes = if mode == Some(CodecMode::Hybrid) {
                            dec.decode_uint(256) + 2
                        } else {
                            len as u32 - ((dec.tell() + 7) >> 3)
                        };
                        len -= redundancy_bytes;
                        // This is a sanity check. It should never happen for a valid packet, so the exact behaviour is not normative.
                        if len * 8 < dec.tell() {
                            len = 0;
                            redundancy_bytes = 0;
                            redundancy = false;
                        }
                        // Shrink decoder because of raw bits.
                        dec.remove_storage(redundancy_bytes as usize);
                    }
                }
            }
        }

        if redundancy {
            pcm_transition_silk_size = None;
        }

        transition_buffer = pcm_transition_silk_size.map(|size| vec![0_f32; size]);

        if mode != Some(CodecMode::CeltOnly) {
            if let Some(buffer) = transition_buffer.as_mut() {
                self.decode_frame(&None, buffer, 0, usize::min(f5, audiosize), false)?;
            }
        }

        if let Some(bandwidth) = bandwidth {
            let end_band = match bandwidth {
                Bandwidth::Narrowband => 13,
                Bandwidth::Mediumband | Bandwidth::Wideband => 17,
                Bandwidth::Superwideband => 19,
                Bandwidth::Fullband => 21,
            };
            self.celt_dec.set_end_band(end_band);
        }
        self.celt_dec.set_stream_channels(self.stream_channels);

        if redundancy {
            let size = f5 * self.channels as usize;
            if size < self.redundant_audio.len() {
                self.redundant_audio.resize(size, 0_f32);
            }
        }

        // 5 ms redundant frame for CELT->SILK.
        if redundancy && celt_to_silk {
            self.celt_dec.set_start_band(0);
            if let Some(packet) = packet {
                self.celt_dec.decode(
                    &Some(&packet[len as usize..]),
                    redundancy_bytes as usize,
                    &mut self.redundant_audio,
                    f5,
                    &mut dec,
                );
            }

            redundant_range = self.celt_dec.final_range();
        }

        // MUST be after PLC.
        if mode != Some(CodecMode::CeltOnly) {
            self.celt_dec.set_start_band(17);
        } else {
            self.celt_dec.set_start_band(0);
        };

        if mode != Some(CodecMode::SilkOnly) {
            let celt_frame_size = usize::min(f20, frame_size);
            // Make sure to discard any previous CELT state.
            if mode != self.prev_mode && self.prev_mode.is_some() && !self.prev_redundancy {
                self.celt_dec.reset()?;
            }

            let data = if decode_fec { &None } else { packet };

            // Decode CELT.
            self.celt_dec
                .decode(data, len as usize, samples, celt_frame_size, &mut dec);
        } else if self.prev_mode == Some(CodecMode::Hybrid)
            && !(redundancy && celt_to_silk && self.prev_redundancy)
        {
            // For hybrid -> SILK transitions, we let the CELT MDCT do a fade-out by decoding a silence frame.
            self.celt_dec.set_start_band(0);
            let silence = [0xFF, 0xFF];
            self.celt_dec
                .decode(&Some(&silence), 2, samples, f2_5, &mut dec);
        }

        if mode != Some(CodecMode::CeltOnly) {
            // This merges the CELT and SILK outputs.
            (0..frame_size * self.channels as usize)
                .into_iter()
                .for_each(|i| {
                    samples[i] += (1.0 / 32768.0) * self.silk_buffer[i];
                });
        }

        // 5 ms redundant frame for SILK->CELT.
        if redundancy && !celt_to_silk {
            self.celt_dec.reset()?;
            self.celt_dec.set_start_band(0);

            if let Some(packet) = packet {
                self.celt_dec.decode(
                    &Some(&packet[len as usize..]),
                    redundancy_bytes as usize,
                    &mut self.redundant_audio,
                    f5,
                    &mut None,
                );
            }
            redundant_range = self.celt_dec.final_range();
            smooth_fade_into_in1(
                &mut samples[self.channels as usize * (frame_size - f2_5)..],
                &self.redundant_audio[self.channels as usize * f2_5..],
                f2_5,
                self.channels as usize,
                self.celt_dec.window(),
                self.sampling_rate as usize,
            );
        }

        if redundancy && celt_to_silk {
            (0..self.channels as usize).into_iter().for_each(|c| {
                (0..f2_5).into_iter().for_each(|i| {
                    samples[self.channels as usize * i + c] =
                        self.redundant_audio[self.channels as usize * i + c];
                });
            });
            smooth_fade_into_in1(
                &mut self.redundant_audio[self.channels as usize * f2_5..],
                &samples[self.channels as usize * f2_5..],
                f2_5,
                self.channels as usize,
                &self.celt_dec.window(),
                self.sampling_rate as usize,
            );
        }

        if let Some(buffer) = transition_buffer {
            if audiosize >= f5 {
                (0..self.channels as usize * f2_5)
                    .into_iter()
                    .for_each(|i| {
                        samples[i] = buffer[i];
                    });
                smooth_fade_into_in2(
                    &buffer[self.channels as usize * f2_5..],
                    &mut samples[self.channels as usize * f2_5..],
                    f2_5,
                    self.channels as usize,
                    self.celt_dec.window(),
                    self.sampling_rate as usize,
                );
            } else {
                // Not enough time to do a clean transition, but we do it anyway
                // This will not preserve amplitude perfectly and may introduce
                // a bit of temporal aliasing, but it shouldn't be too bad and
                // that's pretty much the best we can do. In any case, generating this
                // transition it pretty silly in the first place.
                smooth_fade_into_in2(
                    &buffer,
                    samples,
                    f2_5,
                    self.channels as usize,
                    self.celt_dec.window(),
                    self.sampling_rate as usize,
                );
            }
        }

        if self.decode_gain != 0 {
            let gain = f32::exp2(6.48814081e-4 * self.decode_gain as f32);
            (0..frame_size * self.channels as usize)
                .into_iter()
                .for_each(|i| {
                    samples[i] *= gain;
                });
        }

        if let Some(dec) = dec.as_ref() {
            self.final_range = dec.range() ^ redundant_range;
        } else {
            self.final_range = 0;
        }

        self.prev_mode = mode;
        self.prev_redundancy = redundancy && !celt_to_silk;

        Ok(audiosize)
    }
}

fn smooth_fade_into_in1(
    in1: &mut [f32],
    in2: &[f32],
    overlap: usize,
    channels: usize,
    window: &[f32],
    sampling_rate: usize,
) {
    let inc = 48000 / sampling_rate;
    (0..channels).into_iter().for_each(|c| {
        (0..overlap).into_iter().for_each(|i| {
            let w = window[i * inc] * window[i * inc];
            in1[c + i * channels] =
                (w * in2[i * channels + c]) + ((1.0 - w) * in1[i * channels + c]);
        });
    });
}

fn smooth_fade_into_in2(
    in1: &[f32],
    in2: &mut [f32],
    overlap: usize,
    channels: usize,
    window: &[f32],
    sampling_rate: usize,
) {
    let inc = 48000 / sampling_rate;
    (0..channels).into_iter().for_each(|c| {
        (0..overlap).into_iter().for_each(|i| {
            let w = window[i * inc] * window[i * inc];
            in2[c + i * channels] =
                (w * in2[i * channels + c]) + ((1.0 - w) * in1[i * channels + c]);
        });
    });
}
