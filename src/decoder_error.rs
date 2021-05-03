//! Decoder errors.

use std::error::Error;

/// Errors thrown by the decoder.
#[derive(Debug)]
pub enum DecoderError {
    /// A `std::num::TryFromIntError`.
    TryFromIntError(std::num::TryFromIntError),
    /// An internal decoder error.
    InternalError(&'static str),
}

impl std::fmt::Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderError::TryFromIntError(err) => {
                write!(f, "{:?}", err.source())
            }
            DecoderError::InternalError(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl From<std::num::TryFromIntError> for DecoderError {
    fn from(err: std::num::TryFromIntError) -> DecoderError {
        DecoderError::TryFromIntError(err)
    }
}

impl std::error::Error for DecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            DecoderError::TryFromIntError(ref e) => Some(e),
            _ => None,
        }
    }
}
