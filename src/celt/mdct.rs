//! Implements the modified discrete cosine transform.

use num_complex::Complex32;
use num_traits::Zero;

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
pub(crate) struct Mdct {
    pub(crate) n: usize,
    pub(crate) max_shift: usize,
    pub(crate) kfft: &'static [KissFft],
    pub(crate) trig: &'static [f32],
}

impl Mdct {
    /// Compute a forward MDCT and scale by 4/N, trashes the input array.
    pub(crate) fn forward(
        &self,
        fin: &[f32],
        fout: &mut [f32],
        window: &[f32],
        overlap: usize,
        shift: usize,
        stride: usize,
    ) {
        let fft = &self.kfft[shift];
        let trig_offset = 0;

        let mut n = self.n;
        let mut twiddle_offset = 0;
        (0..fft.shift).into_iter().for_each(|x| {
            n >>= 1;
            twiddle_offset += n;
        });
        let n2 = n >> 1;
        let n4 = n >> 2;

        let mut f = vec![0_f32; n2];
        let mut f2 = vec![Complex32::zero(); n4];

        // Consider the input to be composed of four blocks: [a, b, c, d]
        // Window, shuffle, fold
        {
            let mut xp1 = overlap >> 1;
            let mut xp2 = n2 - 1 + (overlap >> 1);
            let mut yp = 0;

            let mut wp1 = overlap >> 1;
            let mut wp2 = (overlap >> 1) - 1;

            // Real part arranged as -d-cR, Imag part arranged as -b+aR.
            (0..((overlap + 3) >> 2)).into_iter().for_each(|i| {
                f[yp] = (window[wp2] * fin[xp1 + n2]) + (window[wp1] * fin[xp2]);
                f[yp + 1] = (window[wp1] * fin[xp1]) - (window[wp2] * fin[xp2 - n2]);

                yp += 2;
                xp1 += 2;
                xp2 = xp2.wrapping_sub(2);
                wp1 += 2;
                wp2 = wp2.wrapping_sub(2);
            });

            wp1 = 0;
            wp2 = overlap - 1;

            // Real part arranged as a-bR, Imag part arranged as -c-dR.
            (((overlap + 3) >> 2)..n4 - ((overlap + 3) >> 2))
                .into_iter()
                .for_each(|i| {
                    f[yp] = fin[xp2];
                    f[yp + 1] = fin[xp1];

                    yp += 2;
                    xp1 += 2;
                    xp2 = xp2.wrapping_sub(2);
                });

            // Real part arranged as a-bR, Imag part arranged as -c-dR.
            (n4 - ((overlap + 3) >> 2)..n4).into_iter().for_each(|i| {
                f[yp] = -(window[wp1] * fin[xp1 - n2]) + (window[wp2] * fin[xp2]);
                f[yp + 1] = (window[wp2] * fin[xp1]) + (window[wp1] * fin[xp2 + n2]);

                yp += 2;
                xp1 += 2;
                xp2 = xp2.wrapping_sub(2);
                wp1 += 2;
                wp2 = wp2.wrapping_sub(2);
            });
        }

        // Pre-rotation
        {
            let mut yp = 0;
            let mut yc = Complex32::zero();

            (0..n4).into_iter().for_each(|i| {
                let t0 = self.trig[i];
                let t1 = self.trig[n4 + i];
                let re = f[yp];
                let im = f[yp + 1];

                yc.re = (re * t0) - (im * t1);
                yc.im = (im * t0) + (re * t1);
                yc += fft.scale;
                f2[fft.bitrev[i]] = yc;

                yp += 2;
            });
        }

        fft.process(&mut f2);

        // Post-rotate
        {
            let mut fp = 0;
            let mut yp1 = 0;
            let mut yp2 = stride * (n2 - 1);

            (0..n4).into_iter().for_each(|i| {
                fout[yp1] = (f2[fp].im * self.trig[n4 + i]) - (f2[fp].re * self.trig[i]);
                fout[yp2] = (f2[fp].re * self.trig[n4 + i]) + (f2[fp].im * self.trig[i]);

                fp += 1;
                yp1 += 2 * stride;
                yp2 = yp2.wrapping_sub(2 * stride);
            });
        }
    }

    /// Compute a backward MDCT (no scaling) and performs weighted overlap-add
    /// (scales implicitly by 1/2).
    pub(crate) fn backward(
        &self,
        fin: &[f32],
        fout: &mut [f32],
        window: &[f32],
        overlap: usize,
        shift: usize,
        stride: usize,
    ) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use std::f64::consts::PI;

    use nanorand::RNG;

    use crate::celt;

    use super::*;

    fn check_inv(fin: &[f32], fout: &[f32], nfft: usize) {
        let mut err_pow: f64 = 0.0;
        let mut sig_pow: f64 = 0.0;

        (0..nfft / 2).into_iter().for_each(|i| {
            let mut ansr: f64 = 0.0;

            (0..nfft).into_iter().for_each(|k| {
                let phase: f64 =
                    2.0 * PI * (k as f64 + 0.75 * nfft as f64) * (i as f64 + 0.5) / nfft as f64;
                let mut re = phase.cos();

                re /= (nfft / 4) as f64;

                ansr += fin[k] as f64 * re;
            });

            let difr = ansr - fout[i] as f64;
            err_pow += difr * difr;
            sig_pow += ansr * ansr;
        });

        let snr = 10.0 * (sig_pow / err_pow).log10();
        assert!(
            snr > 60.0,
            "nfft={}, inverse={}, poor snr={}",
            nfft,
            true,
            snr
        );
    }

    fn check(fin: &[f32], fout: &[f32], nfft: usize) {
        let mut err_pow: f64 = 0.0;
        let mut sig_pow: f64 = 0.0;

        (0..nfft).into_iter().for_each(|i| {
            let mut ansr: f64 = 0.0;

            (0..nfft / 2).into_iter().for_each(|k| {
                let phase: f64 =
                    2.0 * PI * (i as f64 + 0.75 * nfft as f64) * (k as f64 + 0.5) / nfft as f64;
                let mut re = phase.cos();

                ansr += fin[k] as f64 * re;
            });

            let difr = ansr - fout[i] as f64;
            err_pow += difr * difr;
            sig_pow += ansr * ansr;
        });

        let snr = 10.0 * (sig_pow / err_pow).log10();
        assert!(
            snr > 60.0,
            "nfft={}, inverse={}, poor snr={}",
            nfft,
            false,
            snr
        );
    }

    fn test1d(nfft: usize, is_inverse: bool) {
        let mut rng = nanorand::WyRand::new_seed(42);

        let mut mode = celt::Mode::default();
        let shift = match nfft {
            1920 => 0,
            960 => 1,
            480 => 2,
            240 => 3,
            _ => return,
        };
        let mdct = &mode.mdct;

        let mut fin = vec![0_f32; nfft];
        let mut fout = vec![0_f32; nfft];
        let mut window = vec![1.0_f32; nfft / 2];

        fin.iter_mut().for_each(|x| {
            *x = (rng.generate_range::<u32>(0, 32768) as i16 - 16384) as f32;
            *x *= 32768.0;
        });

        if is_inverse {
            fin.iter_mut().for_each(|x| {
                *x /= nfft as f32;
            });
        }

        let fin_copy = fin.clone();

        if is_inverse {
            mdct.backward(&fin, &mut fout, &window, nfft / 2, shift, 1);

            // Apply TDAC because backward() no longer does that.
            (0..nfft / 4).into_iter().for_each(|i| {
                fout[nfft - i - 1] = fout[nfft / 2 + i];
            });

            check_inv(&fin, &fout, nfft);
        } else {
            mdct.forward(&fin, &mut fout, &window, nfft / 2, shift, 1);
            check(&fin_copy, &fout, nfft);
        }
    }

    #[test]
    fn test_dft() {
        test1d(120, false);
        test1d(120, true);
        test1d(240, false);
        test1d(240, true);
        test1d(480, false);
        test1d(480, true);
        test1d(960, false);
        test1d(960, true);
        test1d(1920, false);
        test1d(1920, true);
    }
}
