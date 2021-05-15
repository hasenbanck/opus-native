//! Decoder errors.

/// Errors thrown by the decoder.
#[derive(Debug)]
pub enum DecoderError {
    /// Bad arguments.
    BadArguments(&'static str),
    /// Invalid packet.
    InvalidPacket,
    /// Frame size is too small for the packet.
    FrameSizeTooSmall,
    /// The sample buffer is too small.
    BufferToSmall,
    /// An internal decoder error.
    InternalError(&'static str),
}

impl std::fmt::Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderError::BadArguments(message) => {
                write!(f, "{}", message)
            }
            DecoderError::InternalError(message) => {
                write!(f, "{}", message)
            }
            DecoderError::BufferToSmall => {
                write!(f, "Sample buffer is too small")
            }
            DecoderError::FrameSizeTooSmall => {
                write!(f, "the frame size is too small for the packet")
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
