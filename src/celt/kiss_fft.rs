//! Implements the FFT used for the MDCT.

use std::cmp::max;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use num_complex::Complex32;
use num_traits::Zero;

const MAX_FACTORS: usize = 8;

/// A mixed-radix Fast Fourier Transform based up on the principle, "Keep It Simple, Stupid."
///
/// This code is originally from Mark Borgerding's KISS-FFT but has been heavily modified
/// to better suit Opus.
pub(crate) struct KissFft {
    pub(crate) nfft: usize,
    pub(crate) scale: f32,
    pub(crate) shift: usize,
    pub(crate) factors: [usize; 2 * MAX_FACTORS],
    pub(crate) bitrev: &'static [usize],
    pub(crate) twiddles: &'static [Complex32],
}

impl KissFft {
    /// N/4 complex FFT.
    pub(crate) fn process(&self, data: &mut [Complex32]) {
        let mut strides = [0_usize; MAX_FACTORS];
        strides[0] = 1;

        let shift = self.shift;

        let mut m = 0;
        let mut l = 0;
        while m != 1 {
            let p = self.factors[2 * l];
            m = self.factors[2 * l + 1];
            strides[l + 1] = strides[l] * p;
            l += 1;
        }
        m = self.factors[2 * l - 1];

        (0..l).into_iter().rev().for_each(|i| {
            let m2 = if i != 0 { self.factors[2 * i - 1] } else { 1 };

            let stride = strides[i] << self.shift;
            match self.factors[2 * i] {
                2 => self.butterfly2(data, m, strides[i]),
                4 => self.butterfly4(data, stride, m, strides[i], m2),
                3 => self.butterfly3(data, stride, m, strides[i], m2),
                5 => self.butterfly5(data, stride, m, strides[i], m2),
                _ => {
                    unreachable!()
                }
            }
            m = m2;
        });
    }

    fn butterfly2(&self, data: &mut [Complex32], m: usize, n: usize) {
        // We know that m==4 here because the radix-2 is just after a radix-4.
        debug_assert!(m == 4);

        let mut offset = 0;
        let mut offset2 = 0;
        let tw = std::f32::consts::FRAC_1_SQRT_2;

        (0..n).into_iter().for_each(|i| {
            offset2 = offset + 4;

            let mut t = data[offset2];
            data[offset2] = data[offset] - t;
            data[offset] += t;

            t.re = (data[offset2 + 1].re + data[offset2 + 1].im) * tw;
            t.im = (data[offset2 + 1].im - data[offset2 + 1].re) * tw;
            data[offset2 + 1] = data[offset + 1] - t;
            data[offset + 1] += t;

            t.re = data[offset2 + 2].im;
            t.im = -data[offset2 + 2].re;
            data[offset2 + 2] = data[offset + 2] - t;
            data[offset + 2] += t;

            t.re = (data[offset2 + 3].im - data[offset2 + 3].re) * tw;
            t.im = (-(data[offset2 + 3].im + data[offset2 + 3].re)) * tw;
            data[offset2 + 3] = data[offset + 3] - t;
            data[offset + 3] += t;

            offset += 8;
        });
    }

    fn butterfly3(&self, data: &mut [Complex32], stride: usize, m: usize, n: usize, mm: usize) {
        // m is guaranteed to be a multiple of 4.
        debug_assert!(m % 4 == 0);

        let mut scratch = [Complex32::zero(); 5];
        let m2 = 2 * m;
        let epi3 = self.twiddles[stride * m];

        (0..n).into_iter().for_each(|i| {
            let mut offset = i * mm;
            let mut tw1_offset = 0;
            let mut tw2_offset = 0;

            (1..m + 1).into_iter().rev().for_each(|k| {
                scratch[1] = data[offset + m] * self.twiddles[tw1_offset];
                scratch[2] = data[offset + m2] * self.twiddles[tw2_offset];

                scratch[3] = scratch[1] + scratch[2];
                scratch[0] = scratch[1] - scratch[2];
                tw1_offset += stride;
                tw2_offset += stride * 2;

                data[offset + m] = data[offset] - (scratch[3] * 0.5);

                scratch[0] *= epi3.im;

                data[offset] += scratch[3];

                data[offset + m2].re = data[offset + m].re + scratch[0].im;
                data[offset + m2].im = data[offset + m].im - scratch[0].re;

                data[offset + m].re -= scratch[0].im;
                data[offset + m].im += scratch[0].re;

                offset += 1;
            });
        });
    }

    fn butterfly4(&self, data: &mut [Complex32], stride: usize, m: usize, n: usize, mm: usize) {
        if m == 1 {
            let mut offset = 0;

            // Degenerate case where all the twiddles are 1.
            (0..n).into_iter().for_each(|i| {
                let scratch0 = data[offset] - data[offset + 2];
                data[offset] += data[offset + 2];
                let mut scratch1 = data[offset + 1] + data[offset + 3];
                data[offset + 2] = data[offset] - scratch1;
                data[offset] += scratch1;
                scratch1 = data[offset + 1] - data[offset + 3];

                data[offset + 1].re = scratch0.re + scratch1.im;
                data[offset + 1].im = scratch0.im - scratch1.re;
                data[offset + 3].re = scratch0.re - scratch1.im;
                data[offset + 3].im = scratch0.im + scratch1.re;

                offset += 4;
            });
        } else {
            // m is guaranteed to be a multiple of 4.
            debug_assert!(m % 4 == 0);

            let mut scratch = [Complex32::zero(); 6];
            let m2 = 2 * m;
            let m3 = 3 * m;

            (0..n).into_iter().for_each(|i| {
                let mut offset = i * mm;
                let mut tw1_offset = 0;
                let mut tw2_offset = 0;
                let mut tw3_offset = 0;

                (0..m).into_iter().for_each(|j| {
                    scratch[0] = data[offset + m] * self.twiddles[tw1_offset];
                    scratch[1] = data[offset + m2] * self.twiddles[tw2_offset];
                    scratch[2] = data[offset + m3] * self.twiddles[tw3_offset];

                    scratch[5] = data[offset] - scratch[1];
                    data[offset] += scratch[1];
                    scratch[3] = scratch[0] + scratch[2];
                    scratch[4] = scratch[0] - scratch[2];
                    data[offset + m2] = data[offset] - scratch[3];
                    tw1_offset += stride;
                    tw2_offset += stride * 2;
                    tw3_offset += stride * 3;
                    data[offset] += scratch[3];

                    data[offset + m].re = scratch[5].re + scratch[4].im;
                    data[offset + m].im = scratch[5].im - scratch[4].re;
                    data[offset + m3].re = scratch[5].re - scratch[4].im;
                    data[offset + m3].im = scratch[5].im + scratch[4].re;

                    offset += 1;
                });
            });
        }
    }

    fn butterfly5(&self, data: &mut [Complex32], stride: usize, m: usize, n: usize, mm: usize) {
        // m is guaranteed to be a multiple of 4.
        debug_assert!(m % 4 == 0);

        let mut scratch = [Complex32::zero(); 13];
        let ya = self.twiddles[stride * m];
        let yb = self.twiddles[stride * 2 * m];

        (0..n).into_iter().for_each(|i| {
            let mut offset0 = i * mm;
            let mut offset1 = offset0 + m;
            let mut offset2 = offset0 + 2 * m;
            let mut offset3 = offset0 + 3 * m;
            let mut offset4 = offset0 + 4 * m;

            (0..m).into_iter().for_each(|u| {
                scratch[0] = data[offset0];
                scratch[1] = data[offset1] * self.twiddles[u * stride];
                scratch[2] = data[offset2] * self.twiddles[2 * u * stride];
                scratch[3] = data[offset3] * self.twiddles[3 * u * stride];
                scratch[4] = data[offset4] * self.twiddles[4 * u * stride];

                scratch[7] = scratch[1] + scratch[4];
                scratch[10] = scratch[1] - scratch[4];
                scratch[8] = scratch[2] + scratch[3];
                scratch[9] = scratch[2] - scratch[3];

                data[offset0] += scratch[7] + scratch[8];

                scratch[5].re = scratch[0].re + (scratch[7].re * ya.re + scratch[8].re * yb.re);
                scratch[5].im = scratch[0].im + (scratch[7].im * ya.re + scratch[8].im * yb.re);

                scratch[6].re = (scratch[10].im * ya.im + scratch[9].im * yb.im);
                scratch[6].im = -(scratch[10].re * ya.im + scratch[9].re * yb.im);

                data[offset1] = scratch[5] - scratch[6];
                data[offset4] = scratch[5] + scratch[6];

                scratch[11].re = scratch[0].re + (scratch[7].re * yb.re + scratch[8].re * ya.re);
                scratch[11].im = scratch[0].im + (scratch[7].im * yb.re + scratch[8].im * ya.re);
                scratch[12].re = scratch[9].im * ya.im - scratch[10].im * yb.im;
                scratch[12].im = scratch[10].re * yb.im - scratch[9].re * ya.im;

                data[offset2] = scratch[11] + scratch[12];
                data[offset3] = scratch[11] - scratch[12];

                offset0 += 1;
                offset1 += 1;
                offset2 += 1;
                offset3 += 1;
                offset4 += 1;
            });
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use nanorand::RNG;

    use crate::celt;
    use crate::celt::Mdct;

    use super::*;

    /// Applies the forward FFT on the given data in `input` and saved the result in `output`.
    fn forward(fft: &KissFft, input: &[Complex32], output: &mut [Complex32]) {
        // Bit-reverse and scale the input.
        (0..fft.nfft).into_iter().for_each(|i| {
            output[fft.bitrev[i]] = input[i] * fft.scale;
        });

        fft.process(output);
    }

    /// Applies the inverse FFT on the given data in `input` and saved the result in `output`.
    fn inverse(fft: &KissFft, input: &[Complex32], output: &mut [Complex32]) {
        // Bit-reverse the input.
        (0..fft.nfft).into_iter().for_each(|i| {
            output[fft.bitrev[i]] = input[i];
        });

        (0..fft.nfft).into_iter().for_each(|i| {
            output[i].im = -output[i].im;
        });

        fft.process(output);

        (0..fft.nfft).into_iter().for_each(|i| {
            output[i].im = -output[i].im;
        });
    }

    fn check(input: &[Complex32], output: &[Complex32], nfft: usize, is_inverse: bool) {
        let mut err_pow: f64 = 0.0;
        let mut sig_pow: f64 = 0.0;

        output.iter().enumerate().for_each(|(i, fout)| {
            let mut ansr: f64 = 0.0;
            let mut ansi: f64 = 0.0;

            input.iter().enumerate().for_each(|(k, fin)| {
                let phase = -2.0 * std::f64::consts::PI * i as f64 * k as f64 / nfft as f64;
                let mut re = phase.cos();
                let mut im = phase.sin();

                if is_inverse {
                    im = -im;
                } else {
                    re /= nfft as f64;
                    im /= nfft as f64;
                }

                ansr += fin.re as f64 * re - fin.im as f64 * im;
                ansi += fin.re as f64 * im + fin.im as f64 * re;
            });

            let difr = ansr - fout.re as f64;
            let difi = ansi - fout.im as f64;
            err_pow += difr * difr + difi * difi;
            sig_pow += ansr * ansr + ansi * ansi;
        });

        let snr = 10.0 * (sig_pow / err_pow).log10();
        assert!(
            snr > 60.0,
            "nfft={}, inverse={}, poor snr={}",
            nfft,
            is_inverse,
            snr
        );
    }

    fn test1d(nfft: usize, is_inverse: bool) {
        let mut rng = nanorand::WyRand::new_seed(42);
        let mut input = vec![Complex32::default(); nfft];
        let mut output = vec![Complex32::default(); nfft];

        let id = match nfft {
            480 => 0,
            240 => 1,
            120 => 2,
            60 => 3,
            _ => return,
        };
        let mdct = Mdct::default();
        let fft = &mdct.kfft[id];

        input.iter_mut().for_each(|x| {
            x.re = (rng.generate_range::<u32>(0, 32767) as i16 - 16384) as f32;
            x.im = (rng.generate_range::<u32>(0, 32767) as i16 - 16384) as f32;
        });

        input.iter_mut().for_each(|x| {
            x.re *= 32768.0;
            x.im *= 32768.0;
        });

        if is_inverse {
            input.iter_mut().for_each(|x| {
                x.re /= nfft as f32;
                x.im /= nfft as f32;
            });
        }

        if is_inverse {
            inverse(&fft, &input, &mut output);
        } else {
            forward(&fft, &input, &mut output);
        }

        check(&input, &output, nfft, is_inverse);
    }

    #[test]
    fn test_dft() {
        test1d(60, false);
        test1d(60, true);
        test1d(120, false);
        test1d(120, true);
        test1d(240, false);
        test1d(240, true);
        test1d(480, false);
        test1d(480, true);
    }
}
