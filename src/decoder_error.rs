//! Decoder errors.

/// Errors thrown by the decoder.
#[derive(Debug)]
pub enum DecoderError {
    /// An internal decoder error.
    InternalError(&'static str),
}

impl std::fmt::Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderError::InternalError(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for DecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
