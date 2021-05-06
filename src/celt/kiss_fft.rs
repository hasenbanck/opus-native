//! Implements the FFT used for the MDCT.

const MAX_FACTORS: usize = 8;

/// A complex number used inside the FFT functions.
pub(crate) struct Complex {
    pub(crate) r: f32,
    pub(crate) i: f32,
}

/// A complex number for the twiddle factor used inside the FFT functions.
pub struct Twiddle {
    pub(crate) r: f32,
    pub(crate) i: f32,
}

/// A mixed-radix Fast Fourier Transform based up on the principle, "Keep It Simple, Stupid."
///
/// This code is originally from Mark Borgerding's KISS-FFT but has been
/// heavily modified to better suit Opus.
// TODO tests: test_unit_dft.c
pub(crate) struct KissFft {
    pub(crate) nfft: usize,
    pub(crate) scale: f32,
    pub(crate) shift: usize,
    pub(crate) factors: [u16; 2 * MAX_FACTORS],
    pub(crate) bitrev: &'static [u16],
    pub(crate) twiddles: &'static [Twiddle],
}

impl KissFft {}
