#[cfg(feature = "decoder")]
pub(crate) use decoder::CeltDecoder;

#[cfg(feature = "decoder")]
mod decoder;
mod kiss_fft;
mod mdct;
