//! Custom errors.

/// Errors thrown by the decoder / encoder.
#[derive(Debug)]
pub enum OpusError {
    /// Bad arguments.
    BadArguments(&'static str),
    /// Invalid packet.
    InvalidPacket,
    /// Frame size is too small for the packet.
    FrameSizeTooSmall,
    /// The buffer is too small.
    BufferToSmall,
    /// An internal error.
    InternalError(&'static str),
}

impl std::fmt::Display for OpusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpusError::BadArguments(message) => {
                write!(f, "{}", message)
            }
            OpusError::InternalError(message) => {
                write!(f, "{}", message)
            }
            OpusError::BufferToSmall => {
                write!(f, "buffer is too small")
            }
            OpusError::FrameSizeTooSmall => {
                write!(f, "the frame size is too small for the packet")
            }
            OpusError::InvalidPacket => {
                write!(f, "invalid packet")
            }
        }
    }
}

impl std::error::Error for OpusError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
