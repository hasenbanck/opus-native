#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
//! Implements the free and open audio codec Opus in Rust.
//!
//! The Opus codec is designed for interactive speech and audio transmission over the Internet.
//! It is designed by the IETF Codec Working Group and incorporates technology from
//! Skype's SILK codec and Xiph.Org's CELT codec.
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
//! * Floating point and fixed-point implementation
//!
pub(crate) use celt::*;
#[cfg(feature = "decoder")]
pub use decoder::*;
#[cfg(feature = "decoder")]
pub use decoder_error::*;
#[cfg(feature = "encoder")]
pub use encoder::*;
#[cfg(feature = "encoder")]
pub use encoder::*;
pub(crate) use silk::*;

mod celt;
#[cfg(feature = "decoder")]
mod decoder;
#[cfg(feature = "decoder")]
mod decoder_error;
#[cfg(feature = "encoder")]
mod encoder;
#[cfg(feature = "encoder")]
mod encoder_error;
#[cfg(feature = "ogg")]
mod ogg;
mod range_coder;
mod silk;

/// Allows applications to use their own sample format.
pub trait Sample {
    #[cfg(feature = "decoder")]
    /// Converts the given float into the custom sample.
    fn from_f32(float: f32) -> Self;
}

#[cfg(feature = "decoder")]
impl Sample for f32 {
    fn from_f32(float: f32) -> Self {
        float
    }
}

#[cfg(feature = "decoder")]
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
    _8000Hz,
    /// 12 kHz
    _12000Hz,
    /// 16 kHz
    _16000Hz,
    /// 16 kHz
    _24000Hz,
    /// 48 kHz
    _48000Hz,
}

impl From<SamplingRate> for usize {
    fn from(fs: SamplingRate) -> usize {
        match fs {
            SamplingRate::_8000Hz => 8000,
            SamplingRate::_12000Hz => 12000,
            SamplingRate::_16000Hz => 16000,
            SamplingRate::_24000Hz => 24000,
            SamplingRate::_48000Hz => 48000,
        }
    }
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

/// Codec mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    /// Silk only.
    Silk,
    /// Hybrid mode.
    Hybrid,
    /// Celt only.
    Celt,
}
