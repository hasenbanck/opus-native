//! Implements the FFT used for the MDCT.

/// A mixed-radix Fast Fourier Transform based up on the principle, "Keep It Simple, Stupid."
///
/// This code is originally from Mark Borgerding's KISS-FFT but has been
/// heavily modified to better suit Opus.
// TODO tests: test_unit_dft.c
pub(crate) struct KissFft {}
