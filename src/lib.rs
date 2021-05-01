#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_results)]
#![deny(clippy::as_conversions)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
//! Implements the free and open audio codec Opus in Rust.

#[cfg(feature = "decoder")]
mod decoder;

#[cfg(feature = "encoder")]
mod encoder;

#[cfg(feature = "ogg")]
mod ogg;
