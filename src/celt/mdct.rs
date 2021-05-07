//! Implements the modified discrete cosine transform.

use crate::celt::kiss_fft::KissFft;

/// This is a simple MDCT implementation that uses a N/4 complex FFT
/// to do most of the work. It should be relatively straightforward to
/// plug in pretty much any FFT here.
///
/// This replaces the Vorbis FFT (and uses the exact same API), which
/// was a bit too messy and that was ending up duplicating code
/// (might as well use the same FFT everywhere).
///
/// The algorithm is similar to (and inspired from) Fabrice Bellard's
/// MDCT implementation in FFMPEG, but has differences in signs, ordering
/// and scaling in many places.
// TODO tests: test_unit_mdct.c
pub(crate) struct Mdct {
    pub(crate) n: usize,
    pub(crate) max_shift: usize,
    pub(crate) kfft: &'static [KissFft],
    pub(crate) trig: &'static [f32],
}
