#[cfg(feature = "decoder")]
pub(crate) use decoder::CeltDecoder;

#[cfg(feature = "decoder")]
pub(crate) mod decoder;
pub(crate) mod kiss_fft;
pub(crate) mod mdct;
