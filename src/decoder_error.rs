//! Decoder errors.

/// Errors thrown by the decoder.
#[derive(Debug)]
pub enum DecoderError {
    /// Invalid packet.
    InvalidPacket,
    /// An internal decoder error.
    InternalError(&'static str),
}

impl std::fmt::Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderError::InternalError(message) => {
                write!(f, "{}", message)
            }
            DecoderError::InvalidPacket => {
                write!(f, "invalid packet")
            }
        }
    }
}

impl std::error::Error for DecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
