//! Implements the FFT used for the MDCT.

use crate::math::Log;

const MAX_FACTORS: usize = 8;

/// A complex number used inside the FFT functions.
pub struct Complex {
    r: f32,
    i: f32,
}

/// A complex number for the twiddle factor used inside the FFT functions.
pub struct TwiddleComplex {
    r: f32,
    i: f32,
}

/// A mixed-radix Fast Fourier Transform based up on the principle, "Keep It Simple, Stupid."
///
/// This code is originally from Mark Borgerding's KISS-FFT but has been
/// heavily modified to better suit Opus.
// TODO tests: test_unit_dft.c
pub(crate) struct KissFft {
    nfft: usize,
    scale: usize,
    shift: usize,
    factors: [i16; 2 * MAX_FACTORS],
    bitrev: Vec<i16>,
    twiddles: Vec<TwiddleComplex>,
}

impl KissFft {
    /// Creates a FFT context.
    pub(crate) fn new(nfft: usize) -> Self {
        let scale = nfft.log2() as usize;
        // TODO

        Self {
            nfft,
            scale,
            shift: 0,
            factors: [0_i16; 16],
            bitrev: vec![],
            twiddles: vec![],
        }
    }
}
