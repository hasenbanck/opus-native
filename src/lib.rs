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
use std::convert::TryFrom;

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
    fn from_f32(float: f32) -> Self {
        float
    }
}

impl Sample for i16 {
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

/// Audio channels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channels {
    /// Mono - 1 channel
    Mono,
    /// Stereo - 2 channels
    Stereo,
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
enum CodecMode {
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
pub fn query_packet_channel_count(packet: &[u8]) -> usize {
    debug_assert!(!packet.is_empty());

    if packet[0] & 0x4 != 0 {
        2
    } else {
        1
    }
}

/// Returns the number of frames in an Opus packet.
///
/// Returns `None` if the packet has an invalid format.
///
/// Packet must have at least a size of 1.
///
/// # Arguments
/// * `packet` - Input payload.
///
pub fn query_packet_frame_count(packet: &[u8]) -> Option<usize> {
    debug_assert!(!packet.is_empty());

    let count = packet[0] & 0x3;
    if count == 0 {
        Some(1)
    } else if count != 3 {
        Some(2)
    } else if packet.len() < 2 {
        None
    } else {
        Some((packet[1] & 0x3F) as usize)
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
/// Returns `None` if the packet has an invalid format.
///
/// # Arguments
/// * `packet`        - Input payload.
/// * `sampling_rate` - Sampling rate.
///
pub fn query_packet_sample_count(packet: &[u8], sampling_rate: SamplingRate) -> Option<usize> {
    if let Some(count) = query_packet_frame_count(packet) {
        let samples = count * query_packet_samples_per_frame(packet, sampling_rate);
        if samples * 25 > sampling_rate as usize * 3 {
            None
        } else {
            Some(samples)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use super::*;

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
        assert_eq!(query_packet_channel_count(&[0]), 1);
        assert_eq!(query_packet_channel_count(&[0x4]), 2);
    }

    #[test]
    fn test_query_packet_frame_count() {
        assert_eq!(query_packet_frame_count(&[0]), Some(1));
        assert_eq!(query_packet_frame_count(&[1]), Some(2));
        assert_eq!(query_packet_frame_count(&[2]), Some(2));
        assert_eq!(query_packet_frame_count(&[3]), None);
        assert_eq!(query_packet_frame_count(&[3, 5]), Some(5));
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
            query_packet_sample_count(&[70], SamplingRate::Hz48000),
            Some(960)
        );
        assert_eq!(query_packet_sample_count(&[3], SamplingRate::Hz48000), None);
        assert_eq!(
            query_packet_sample_count(&[255, 5], SamplingRate::Hz48000),
            Some(4800)
        );
    }
}
