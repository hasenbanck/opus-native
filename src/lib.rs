#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
// FIXME only temporary until the main library calls are implemented.
#![allow(unused)]
//! Implements the free and open audio codec Opus in Rust.
//!
//! The Opus codec is designed for interactive speech and audio transmission over the Internet.
//! It is designed by the IETF Codec Working Group and incorporates technology from
//! Skype's Silk codec and Xiph.Org's Celt codec.
//!
//! The Opus codec is designed to handle a wide range of interactive audio applications,
//! including Voice over IP, videoconferencing, in-game chat, and even remote live music
//! performances. It can scale from low bit-rate narrowband speech to very high quality
//! stereo music. Its main features are:
//!
//! * Sampling rates from 8 to 48 kHz
//! * Bit-rates from 6 kb/s to 510 kb/s
//! * Support for both constant bit-rate (CBR) and variable bit-rate (VBR)
//! * Audio bandwidth from narrowband to full-band
//! * Support for speech and music
//! * Support for mono and stereo
//! * Support for multichannel (up to 255 channels)
//! * Frame sizes from 2.5 ms to 60 ms
//! * Good loss robustness and packet loss concealment (PLC)
//!
pub use decoder::*;
pub use decoder_error::*;
pub use encoder::*;
pub use encoder::*;

pub(crate) mod celt;
mod decoder;
mod decoder_error;
mod encoder;
mod encoder_error;
pub(crate) mod math;
#[cfg(feature = "ogg")]
mod ogg;
pub(crate) mod range_coder;
pub(crate) mod silk;

// Affects the following targets: avr and msp430
#[cfg(any(target_pointer_width = "8", target_pointer_width = "16"))]
compile_error!("usize needs to be at least 32 bit wide");

/// Allows applications to use their own sample format.
pub trait Sample {
    /// Converts the given float into the custom sample.
    fn from_f32(float: f32) -> Self;
}

impl Sample for f32 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        float
    }
}

impl Sample for f64 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        float as f64
    }
}

impl Sample for i16 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        let float = float * 32768.0;
        if float > 32767.0 {
            32767
        } else if float < -32768.0 {
            -32768
        } else {
            float as i16
        }
    }
}

impl Sample for i32 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        let float = float * 2_147_483_648.0;
        if float > 2_147_483_647.0 {
            2_147_483_647
        } else if float < -2_147_483_648.0 {
            -2_147_483_648
        } else {
            float as i32
        }
    }
}

impl Sample for u16 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        let float = float * 32768.0 + 32768.0;
        if float > 32767.0 {
            32767
        } else if float < 0.0 {
            0
        } else {
            float as u16
        }
    }
}

impl Sample for u32 {
    #[inline(always)]
    fn from_f32(float: f32) -> Self {
        let float = float * 2_147_483_648.0 + 2_147_483_648.0;
        if float > 4_294_967_295.0 {
            4_294_967_295
        } else if float < 0.0 {
            0
        } else {
            float as u32
        }
    }
}

/// Audio channels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channels {
    /// Mono - 1 channel
    Mono = 1,
    /// Stereo - 2 channels
    Stereo = 2,
}

/// Samples per second.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SamplingRate {
    /// 8 kHz
    Hz8000 = 8000,
    /// 12 kHz
    Hz12000 = 12000,
    /// 16 kHz
    Hz16000 = 16000,
    /// 16 kHz
    Hz24000 = 24000,
    /// 48 kHz
    Hz48000 = 48000,
}

/// Audio bandwidth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bandwidth {
    /// Automatic selection.
    Auto,
    /// 4 kHz passband.
    Narrowband,
    /// 6 kHz passband.
    Mediumband,
    /// 8 kHz passband.
    Wideband,
    /// 12 kHz passband.
    Superwideband,
    /// 20 kHz passband.
    Fullband,
}

const BANDWIDTH_TABLE: [Bandwidth; 32] = [
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Mediumband,
    Bandwidth::Mediumband,
    Bandwidth::Mediumband,
    Bandwidth::Mediumband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Superwideband,
    Bandwidth::Superwideband,
    Bandwidth::Fullband,
    Bandwidth::Fullband,
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Narrowband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Wideband,
    Bandwidth::Superwideband,
    Bandwidth::Superwideband,
    Bandwidth::Superwideband,
    Bandwidth::Superwideband,
    Bandwidth::Fullband,
    Bandwidth::Fullband,
    Bandwidth::Fullband,
    Bandwidth::Fullband,
];

impl From<u8> for Bandwidth {
    fn from(u: u8) -> Self {
        BANDWIDTH_TABLE[u as usize]
    }
}

/// Codec mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecMode {
    /// Silk only.
    Silk,
    /// Hybrid mode.
    Hybrid,
    /// Celt only.
    Celt,
}

/// Returns the bandwidth of an Opus packet.
///
/// Packet must have at least a size of 1.
///
/// # Arguments
/// * `packet` - Input payload.
///
pub fn query_packet_bandwidth(packet: &[u8]) -> Bandwidth {
    debug_assert!(!packet.is_empty());

    let value = (packet[0] & 0xF8) >> 3;
    value.into()
}

/// Returns the number of channels from an Opus packet.
///
/// Packet must have at least a size of 1.
///
/// # Arguments
/// * `packet` - Input payload.
///
pub fn query_packet_channel_count(packet: &[u8]) -> Channels {
    debug_assert!(!packet.is_empty());

    if packet[0] & 0x4 != 0 {
        Channels::Stereo
    } else {
        Channels::Mono
    }
}

/// Returns the number of frames in an Opus packet.
///
/// Packet must have at least a size of 1.
///
/// # Arguments
/// * `packet` - Input payload.
///
pub fn query_packet_frame_count(packet: &[u8]) -> Result<usize, DecoderError> {
    debug_assert!(!packet.is_empty());

    let count = packet[0] & 0x3;
    if count == 0 {
        Ok(1)
    } else if count != 3 {
        Ok(2)
    } else if packet.len() < 2 {
        Err(DecoderError::InvalidPacket)
    } else {
        Ok((packet[1] & 0x3F) as usize)
    }
}

/// Returns the number of samples per frame from an Opus packet.
///
/// # Arguments
/// * `packet`        - Input payload.
/// * `sampling_rate` - Sampling rate.
///
pub fn query_packet_samples_per_frame(packet: &[u8], sampling_rate: SamplingRate) -> usize {
    if packet[0] & 0x80 != 0 {
        let audio_size = usize::from((packet[0] >> 3) & 0x3);
        ((sampling_rate as usize) << audio_size) / 400
    } else if (packet[0] & 0x60) == 0x60 {
        if packet[0] & 0x08 != 0 {
            sampling_rate as usize / 50
        } else {
            sampling_rate as usize / 100
        }
    } else {
        let audio_size = usize::from((packet[0] >> 3) & 0x3);
        if audio_size == 3 {
            sampling_rate as usize * 60 / 1000
        } else {
            ((sampling_rate as usize) << audio_size) / 100
        }
    }
}

/// Returns the number of samples of an Opus packet.
///
/// Packet must have at least a size of 1.
///
/// # Arguments
/// * `packet`        - Input payload.
/// * `sampling_rate` - Sampling rate.
///
pub fn query_packet_sample_count(
    packet: &[u8],
    sampling_rate: SamplingRate,
) -> Result<usize, DecoderError> {
    let count = query_packet_frame_count(packet)?;
    let samples = count * query_packet_samples_per_frame(packet, sampling_rate);
    if samples * 25 > sampling_rate as usize * 3 {
        Err(DecoderError::InvalidPacket)
    } else {
        Ok(samples)
    }
}

/// Returns the codec mode of the Opus packet.
///
/// # Arguments
/// * `packet`        - Input payload.
///
pub fn query_packet_codec_mode(packet: &[u8]) -> CodecMode {
    if packet[0] & 0x80 == 0x80 {
        CodecMode::Celt
    } else if packet[0] & 0x60 == 0x60 {
        CodecMode::Hybrid
    } else {
        CodecMode::Silk
    }
}

/// Parse an Opus packet into one or more frames.
///
/// Returns the number of frames inside the packet.
///
/// Opus_decode will perform this operation internally so most applications do not need
/// to use this function.
///
/// This function does not copy the frames, it returns the offsets to the frames inside the packet.
///
/// # Arguments
/// * `packet`         - Opus packet to be parsed.
/// * `self_delimited` - True if the packet has self delimited framing.
/// * `frames`         - Returns the encapsulated frame offsets.
/// * `sizes`          - Returns the sizes of the encapsulated frames.
/// * `payload_offset` - Returns the position of the payload within the packet (in bytes).
/// * `packet_offset`  - Returns the position of the next packet (in bytes) in
///                      multi channel packets.
///
pub fn parse_packet(
    packet: &[u8],
    self_delimited: bool,
    mut frames: Option<&mut [usize; 48]>,
    sizes: &mut [usize; 48],
    payload_offset: Option<&mut usize>,
    packet_offset: Option<&mut usize>,
) -> Result<usize, DecoderError> {
    let framesize = query_packet_samples_per_frame(packet, SamplingRate::Hz48000);
    let mut offset = 1;
    let mut len = packet.len() - offset;
    let mut last_size = len;
    let mut cbr = false;
    let mut pad = 0;
    let count: usize;

    match packet[0] & 0x3 {
        0 => {
            // One frame.
            count = 1;
        }
        1 => {
            // Two CBR frames.
            count = 2;
            cbr = true;

            if !self_delimited {
                if len & 0x1 == 1 {
                    return Err(DecoderError::InvalidPacket);
                }
                last_size = len / 2;
                // If last_size doesn't fit in size[0], we'll catch it later.
                sizes[0] = last_size;
            }
        }
        2 => {
            // Two VBR frames.
            count = 2;
            let bytes = parse_size(&packet[offset..], &mut sizes[0])?;
            len -= bytes;
            if sizes[0] > len {
                return Err(DecoderError::InvalidPacket);
            }
            offset += bytes;
            last_size = len - sizes[0];
        }
        3 => {
            // Multiple CBR/VBR frames (from 0 to 120 ms).
            if len < 1 {
                return Err(DecoderError::InvalidPacket);
            }
            // Number of frames encoded in bits 0 to 5.
            let ch = usize::from(packet[offset]);
            offset += 1;

            count = ch & 0x3F;
            if framesize * count > 5760 {
                return Err(DecoderError::InvalidPacket);
            }
            len -= 1;

            // Padding flag is bit 6.
            if ch & 0x40 != 0x0 {
                let mut p = 255;
                while p == 255 {
                    p = usize::from(packet[offset]);
                    offset += 1;
                    len -= 1;

                    let tmp = if p == 255 { 254 } else { p };
                    len -= tmp;
                    pad += tmp;
                }
            }

            // VBR flag is bit 7.
            cbr = ch & 0x80 == 0;
            if !cbr {
                // VBR case
                last_size = len;
                (0..count - 1).into_iter().try_for_each(|i| {
                    let bytes = parse_size(&packet[offset..], &mut sizes[i])?;
                    len -= bytes;
                    if sizes[i] > len {
                        return Err(DecoderError::InvalidPacket);
                    }
                    offset += bytes;
                    last_size -= bytes + sizes[i];

                    Ok(())
                })?;
            } else if !self_delimited {
                // CBR case.
                last_size = len / count;
                if last_size * count != len {
                    return Err(DecoderError::InvalidPacket);
                }
                (0..count - 1).into_iter().for_each(|i| {
                    sizes[i] = last_size;
                });
            }
        }
        _ => {
            unreachable!()
        }
    }

    // Self-delimited framing has an extra size for the last frame.
    if self_delimited {
        let bytes = parse_size(&packet[offset..], &mut sizes[count - 1])?;
        len -= bytes;
        if sizes[count - 1] > len {
            return Err(DecoderError::InvalidPacket);
        }
        offset += bytes;
        // For CBR packets, apply the size to all the frames.
        if cbr {
            if sizes[count - 1] * count > len {
                return Err(DecoderError::InvalidPacket);
            }
            (0..count - 1).into_iter().for_each(|i| {
                sizes[i] = sizes[count - 1];
            });
        } else if bytes + sizes[count - 1] > last_size {
            return Err(DecoderError::InvalidPacket);
        }
    } else {
        // Because it's not encoded explicitly, it's possible the size of the
        // last packet (or all the packets, for the CBR case) is larger than
        // 1275. Reject them here.
        if last_size > 1275 {
            return Err(DecoderError::InvalidPacket);
        }
        sizes[count - 1] = last_size;
    }

    if let Some(payload_offset) = payload_offset {
        *payload_offset = offset;
    }

    (0..count).into_iter().for_each(|i| {
        if let Some(frames) = &mut frames {
            frames[i] = offset;
        }

        offset += sizes[i];
    });

    if let Some(packet_offset) = packet_offset {
        *packet_offset = pad + offset;
    }

    Ok(count)
}

fn parse_size(data: &[u8], size: &mut usize) -> Result<usize, DecoderError> {
    if data.is_empty() {
        Err(DecoderError::InvalidPacket)
    } else if data[0] < 252 {
        *size = data[0] as usize;
        Ok(1)
    } else if data.len() < 2 {
        Err(DecoderError::InvalidPacket)
    } else {
        *size = 4 * usize::from(data[1]) + usize::from(data[0]);
        Ok(2)
    }
}

/// Applies soft-clipping to bring a float signal within the [-1,1] range. If
/// the signal is already in that range, nothing is done. If there are values
/// outside of [-1,1], then the signal is clipped as smoothly as possible to
/// both fit in the range and avoid creating excessive distortion in the
/// process.
///
/// # Arguments
/// * `pcm`          - Input PCM and modified PCM.
/// * `channels`     - Number of channels.
/// * `softclip_mem` - State memory for the soft clipping process
///                    (one float per channel, initialized to zero).
///
pub fn pcm_soft_clip(pcm: &mut [f32], channels: usize, softclip_mem: &mut [f32]) {
    if pcm.is_empty() || channels == 0 || softclip_mem.len() < channels {
        return;
    }
    let channels = channels;
    let frame_size = pcm.len() / channels;

    // First thing: saturate everything to +/- 2 which is the highest level our
    // non-linearity can handle. At the point where the signal reaches +/-2,
    // the derivative will be zero anyway, so this doesn't introduce any
    // discontinuity in the derivative.
    pcm.iter_mut()
        .for_each(|x| *x = f32::min(f32::max(*x, -2.0), 2.0));

    (0..channels).into_iter().for_each(|c| {
        let mut a = softclip_mem[c];

        // Continue applying the non-linearity from the previous frame to avoid
        // any discontinuity.
        for i in 0..frame_size {
            let off = c + i * channels;
            if pcm[off] * a >= 0.0 {
                break;
            }
            pcm[off] += a * pcm[off] * pcm[off];
        }

        let mut curr = 0;
        let x0 = pcm[c];

        loop {
            let mut pos = 0;
            for i in curr..frame_size {
                pos = i;
                if pcm[c + pos * channels] > 1.0 || pcm[c + pos * channels] < -1.0 {
                    break;
                }
            }

            if pos == frame_size {
                a = 0.0;
                break;
            }

            let mut peak_pos = pos;
            let mut start = pos;
            let mut end = pos;
            let mut maxval = f32::abs(pcm[c + pos * channels]);

            // Look for first zero crossing before clipping.
            while start > 0 && pcm[c + pos * channels] * pcm[c + (start - 1) * channels] >= 0.0 {
                start -= 1;
            }

            // Look for first zero crossing after clipping.
            while end < frame_size && pcm[c + pos * channels] * pcm[c + end * channels] >= 0.0 {
                // Look for other peaks until the next zero-crossing.
                if f32::abs(pcm[c + end * channels]) > maxval {
                    maxval = f32::abs(pcm[c + end * channels]);
                    peak_pos = end;
                }
                end += 1;
            }

            // Detect the special case where we clip before the first zero crossing.
            let special = start == 0 && (pcm[c + pos * channels] * pcm[c]) >= 0.0;

            // Compute a such that maxval + a * maxval^2 = 1
            a = (maxval - 1.0) / (maxval * maxval);

            // Ported for compatibility with the reference implementation:
            // Slightly boost "a" by 2^-22. This is just enough to ensure -ffast-math
            // does not cause output values larger than +/-1, but small enough not
            // to matter even for 24-bit output.
            a += a * 2.4e-7;

            if pcm[c + pos * channels] > 0.0 {
                a = -a;
            }

            // Apply soft clipping.
            (start..end).into_iter().for_each(|i| {
                let off = c + i * channels;
                pcm[off] += a * pcm[off] * pcm[off];
            });

            if special && peak_pos >= 2 {
                // Add a linear ramp from the first sample to the signal peak.
                // This avoids a discontinuity at the beginning of the frame.
                let mut offset = x0 - pcm[c];
                let delta = offset / peak_pos as f32;

                (curr..peak_pos).into_iter().for_each(|i| {
                    let off = c + i * channels;
                    offset -= delta;
                    pcm[off] += offset;
                    pcm[off] = f32::min(f32::max(pcm[off], -1.0), 1.0);
                });
            }

            curr = end;
            if curr == frame_size {
                break;
            }
        }
        softclip_mem[c] = a;
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use super::*;

    const TEST_PACKET_SINGLE: &[u8] = &[
        0x80, 0xDA, 0x84, 0xE8, 0x87, 0x77, 0x83, 0xD6, 0x48, 0xB3, 0x6B, 0x45,
    ];
    const TEST_PACKET_CBR: &[u8] = &[
        0x81, 0xDA, 0x84, 0xE8, 0x87, 0x77, 0x83, 0xD6, 0x48, 0xB3, 0x6B,
    ];
    const TEST_PACKET_VBR: &[u8] = &[
        0x82, 0x4, 0xDA, 0x84, 0xE8, 0x87, 0x77, 0x83, 0xD6, 0x48, 0xB3, 0x6B,
    ];
    const TEST_PACKET_INVALID: &[u8] = &[0x81, 0xDA];

    #[test]
    fn test_query_packet_bandwidth() {
        let bandwidths: Vec<Bandwidth> = (0..32)
            .into_iter()
            .map(|c| {
                let arr = [c << 3];
                query_packet_bandwidth(&arr)
            })
            .collect();

        assert_eq!(bandwidths.len(), 32);
        assert_eq!(bandwidths[0], Bandwidth::Narrowband);
        assert_eq!(bandwidths[1], Bandwidth::Narrowband);
        assert_eq!(bandwidths[2], Bandwidth::Narrowband);
        assert_eq!(bandwidths[3], Bandwidth::Narrowband);
        assert_eq!(bandwidths[4], Bandwidth::Mediumband);
        assert_eq!(bandwidths[5], Bandwidth::Mediumband);
        assert_eq!(bandwidths[6], Bandwidth::Mediumband);
        assert_eq!(bandwidths[7], Bandwidth::Mediumband);
        assert_eq!(bandwidths[8], Bandwidth::Wideband);
        assert_eq!(bandwidths[9], Bandwidth::Wideband);
        assert_eq!(bandwidths[10], Bandwidth::Wideband);
        assert_eq!(bandwidths[11], Bandwidth::Wideband);
        assert_eq!(bandwidths[12], Bandwidth::Superwideband);
        assert_eq!(bandwidths[13], Bandwidth::Superwideband);
        assert_eq!(bandwidths[14], Bandwidth::Fullband);
        assert_eq!(bandwidths[15], Bandwidth::Fullband);
        assert_eq!(bandwidths[16], Bandwidth::Narrowband);
        assert_eq!(bandwidths[17], Bandwidth::Narrowband);
        assert_eq!(bandwidths[18], Bandwidth::Narrowband);
        assert_eq!(bandwidths[19], Bandwidth::Narrowband);
        assert_eq!(bandwidths[20], Bandwidth::Wideband);
        assert_eq!(bandwidths[21], Bandwidth::Wideband);
        assert_eq!(bandwidths[22], Bandwidth::Wideband);
        assert_eq!(bandwidths[23], Bandwidth::Wideband);
        assert_eq!(bandwidths[24], Bandwidth::Superwideband);
        assert_eq!(bandwidths[25], Bandwidth::Superwideband);
        assert_eq!(bandwidths[26], Bandwidth::Superwideband);
        assert_eq!(bandwidths[27], Bandwidth::Superwideband);
        assert_eq!(bandwidths[28], Bandwidth::Fullband);
        assert_eq!(bandwidths[29], Bandwidth::Fullband);
        assert_eq!(bandwidths[30], Bandwidth::Fullband);
        assert_eq!(bandwidths[31], Bandwidth::Fullband);
    }

    #[test]
    fn test_query_packet_channel_count() {
        assert_eq!(query_packet_channel_count(&[0]), Channels::Mono);
        assert_eq!(query_packet_channel_count(&[0x4]), Channels::Stereo);
    }

    #[test]
    fn test_query_packet_frame_count() {
        assert_eq!(query_packet_frame_count(&[0]).unwrap(), 1);
        assert_eq!(query_packet_frame_count(&[1]).unwrap(), 2);
        assert_eq!(query_packet_frame_count(&[2]).unwrap(), 2);
        assert!(query_packet_frame_count(&[3]).is_err());
        assert_eq!(query_packet_frame_count(&[3, 5]).unwrap(), 5);
    }

    #[test]
    fn test_query_packet_samples_per_frame() {
        let frame_sizes: Vec<usize> = (0..32)
            .into_iter()
            .map(|c| {
                let arr = [c << 3];
                query_packet_samples_per_frame(&arr, SamplingRate::Hz48000)
            })
            .collect();

        assert_eq!(frame_sizes.len(), 32);
        assert_eq!(frame_sizes[0], 480);
        assert_eq!(frame_sizes[1], 960);
        assert_eq!(frame_sizes[2], 1920);
        assert_eq!(frame_sizes[3], 2880);
        assert_eq!(frame_sizes[4], 480);
        assert_eq!(frame_sizes[5], 960);
        assert_eq!(frame_sizes[6], 1920);
        assert_eq!(frame_sizes[7], 2880);
        assert_eq!(frame_sizes[8], 480);
        assert_eq!(frame_sizes[9], 960);
        assert_eq!(frame_sizes[10], 1920);
        assert_eq!(frame_sizes[11], 2880);
        assert_eq!(frame_sizes[12], 480);
        assert_eq!(frame_sizes[13], 960);
        assert_eq!(frame_sizes[14], 480);
        assert_eq!(frame_sizes[15], 960);
        assert_eq!(frame_sizes[16], 120);
        assert_eq!(frame_sizes[17], 240);
        assert_eq!(frame_sizes[18], 480);
        assert_eq!(frame_sizes[19], 960);
        assert_eq!(frame_sizes[20], 120);
        assert_eq!(frame_sizes[21], 240);
        assert_eq!(frame_sizes[22], 480);
        assert_eq!(frame_sizes[23], 960);
        assert_eq!(frame_sizes[24], 120);
        assert_eq!(frame_sizes[25], 240);
        assert_eq!(frame_sizes[26], 480);
        assert_eq!(frame_sizes[27], 960);
        assert_eq!(frame_sizes[28], 120);
        assert_eq!(frame_sizes[29], 240);
        assert_eq!(frame_sizes[30], 480);
        assert_eq!(frame_sizes[31], 960);
    }

    #[test]
    fn test_query_packet_sample_count() {
        assert_eq!(
            query_packet_sample_count(&[70], SamplingRate::Hz48000).unwrap(),
            960
        );
        assert!(query_packet_sample_count(&[3], SamplingRate::Hz48000).is_err());
        assert_eq!(
            query_packet_sample_count(&[255, 5], SamplingRate::Hz48000).unwrap(),
            4800
        );
    }

    #[test]
    fn test_parse_packet_with_single_frame() {
        let mut frames = [0; 48];
        let mut sizes = [0; 48];
        let mut payload_offset = 0;
        let mut packet_offset = 0;

        let count = parse_packet(
            TEST_PACKET_SINGLE,
            false,
            Some(&mut frames),
            &mut sizes,
            Some(&mut payload_offset),
            Some(&mut packet_offset),
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(frames[0], 1);
        assert_eq!(sizes[0], 11);
        assert_eq!(payload_offset, 1);
        assert_eq!(packet_offset, 12);
    }

    #[test]
    fn test_parse_packet_with_two_cbr_frames() {
        let mut frames = [0; 48];
        let mut sizes = [0; 48];
        let mut payload_offset = 0;
        let mut packet_offset = 0;

        let count = parse_packet(
            TEST_PACKET_CBR,
            false,
            Some(&mut frames),
            &mut sizes,
            Some(&mut payload_offset),
            Some(&mut packet_offset),
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(frames[0], 1);
        assert_eq!(sizes[0], 5);
        assert_eq!(frames[1], 6);
        assert_eq!(sizes[1], 5);
        assert_eq!(payload_offset, 1);
        assert_eq!(packet_offset, 11);
    }

    #[test]
    fn test_parse_packet_with_two_vbr_frames() {
        let mut frames = [0; 48];
        let mut sizes = [0; 48];
        let mut payload_offset = 0;
        let mut packet_offset = 0;

        let count = parse_packet(
            TEST_PACKET_VBR,
            false,
            Some(&mut frames),
            &mut sizes,
            Some(&mut payload_offset),
            Some(&mut packet_offset),
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(frames[0], 2);
        assert_eq!(sizes[0], 4);
        assert_eq!(frames[1], 6);
        assert_eq!(sizes[1], 6);
        assert_eq!(payload_offset, 2);
        assert_eq!(packet_offset, 12);
    }

    #[test]
    fn test_parse_packet_invalid_frame() {
        let mut frames = [0; 48];
        let mut sizes = [0; 48];

        assert!(parse_packet(
            TEST_PACKET_INVALID,
            false,
            Some(&mut frames),
            &mut sizes,
            None,
            None,
        )
        .is_err())
    }

    #[test]
    fn test_pcm_soft_clip() {
        let mut x = [0_f32; 1024];
        let mut s = [0_f32; 8];

        (0..1024).into_iter().for_each(|i| {
            (0..1024).into_iter().for_each(|j| {
                x[j] = (j & 255) as f32 * (1.0 / 32.0) - 4.0;
            });

            pcm_soft_clip(&mut x[i..], 1, &mut s);

            (i..1024).into_iter().for_each(|j| {
                assert!(x[j] <= 1.0);
                assert!(x[j] >= -1.0);
            });
        });

        (1..9).into_iter().for_each(|i| {
            (0..1024).into_iter().for_each(|j| {
                x[j] = (j & 255) as f32 * (1.0 / 32.0) - 4.0;
            });
            pcm_soft_clip(&mut x, i, &mut s);
            (0..(1024 / i) * i).into_iter().for_each(|j| {
                assert!(x[j] <= 1.0);
                assert!(x[j] >= -1.0);
            });
        });
    }
}
