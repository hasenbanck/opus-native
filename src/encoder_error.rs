//! Encoder errors.

/// Errors thrown by the encoder.
#[derive(Debug)]
pub enum EncoderError {
    /// The output buffer is too small.
    BufferToSmall,
    /// An internal encoder error.
    InternalError(&'static str),
}

impl std::fmt::Display for EncoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncoderError::BufferToSmall => {
                write!(f, "output buffer is too small")
            }
            EncoderError::InternalError(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for EncoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
