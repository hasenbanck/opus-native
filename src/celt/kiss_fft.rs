//! Implements the FFT used for the MDCT.

use std::f32::consts::FRAC_1_SQRT_2;

use crate::math::Complex;

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
    pub(crate) bitrev: &'static [u16],
    pub(crate) twiddles: &'static [Complex],
}

impl KissFft {
    /// N/4 complex FFT.
    pub(crate) fn process(&self, data: &mut [Complex]) {
        let mut strides = [0_usize; MAX_FACTORS];
        strides[0] = 1;

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

    fn butterfly2(&self, data: &mut [Complex], m: usize, n: usize) {
        // We know that m==4 here because the radix-2 is just after a radix-4.
        debug_assert!(m == 4);

        let mut offset = 0;
        let mut offset2 = 0;

        (0..n).into_iter().for_each(|_| {
            offset2 = offset + 4;

            let mut t = data[offset2];
            data[offset2] = data[offset] - t;
            data[offset] += t;

            t.r = (data[offset2 + 1].r + data[offset2 + 1].i) * FRAC_1_SQRT_2;
            t.i = (data[offset2 + 1].i - data[offset2 + 1].r) * FRAC_1_SQRT_2;
            data[offset2 + 1] = data[offset + 1] - t;
            data[offset + 1] += t;

            t.r = data[offset2 + 2].i;
            t.i = -data[offset2 + 2].r;
            data[offset2 + 2] = data[offset + 2] - t;
            data[offset + 2] += t;

            t.r = (data[offset2 + 3].i - data[offset2 + 3].r) * FRAC_1_SQRT_2;
            t.i = (-(data[offset2 + 3].i + data[offset2 + 3].r)) * FRAC_1_SQRT_2;
            data[offset2 + 3] = data[offset + 3] - t;
            data[offset + 3] += t;

            offset += 8;
        });
    }

    fn butterfly3(&self, data: &mut [Complex], stride: usize, m: usize, n: usize, mm: usize) {
        // m is guaranteed to be a multiple of 4.
        debug_assert!(m % 4 == 0);

        let mut scratch = [Complex::default(); 5];
        let m2 = 2 * m;
        let epi3 = self.twiddles[stride * m];

        (0..n).into_iter().for_each(|i| {
            let mut offset = i * mm;
            let mut tw1_offset = 0;
            let mut tw2_offset = 0;

            (1..m + 1).into_iter().rev().for_each(|_| {
                scratch[1] = data[offset + m] * self.twiddles[tw1_offset];
                scratch[2] = data[offset + m2] * self.twiddles[tw2_offset];

                scratch[3] = scratch[1] + scratch[2];
                scratch[0] = scratch[1] - scratch[2];
                tw1_offset += stride;
                tw2_offset += stride * 2;

                data[offset + m] = data[offset] - (scratch[3] * 0.5);

                scratch[0] *= epi3.i;

                data[offset] += scratch[3];

                data[offset + m2].r = data[offset + m].r + scratch[0].i;
                data[offset + m2].i = data[offset + m].i - scratch[0].r;

                data[offset + m].r -= scratch[0].i;
                data[offset + m].i += scratch[0].r;

                offset += 1;
            });
        });
    }

    fn butterfly4(&self, data: &mut [Complex], stride: usize, m: usize, n: usize, mm: usize) {
        if m == 1 {
            let mut offset = 0;

            // Degenerate case where all the twiddles are 1.
            (0..n).into_iter().for_each(|_| {
                let scratch0 = data[offset] - data[offset + 2];
                let scratch1 = data[offset + 1] + data[offset + 3];

                data[offset] += data[offset + 2];
                data[offset + 2] = data[offset] - scratch1;
                data[offset] += scratch1;

                let scratch1 = data[offset + 1] - data[offset + 3];

                data[offset + 1].r = scratch0.r + scratch1.i;
                data[offset + 1].i = scratch0.i - scratch1.r;
                data[offset + 3].r = scratch0.r - scratch1.i;
                data[offset + 3].i = scratch0.i + scratch1.r;

                offset += 4;
            });
        } else {
            // m is guaranteed to be a multiple of 4.
            debug_assert!(m % 4 == 0);

            let m2 = 2 * m;
            let m3 = 3 * m;
            let mut scratch = [Complex::default(); 6];

            (0..n).into_iter().for_each(|i| {
                let mut offset = i * mm;
                let mut tw1_offset = 0;
                let mut tw2_offset = 0;
                let mut tw3_offset = 0;

                (0..m).into_iter().for_each(|_| {
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

                    data[offset + m].r = scratch[5].r + scratch[4].i;
                    data[offset + m].i = scratch[5].i - scratch[4].r;
                    data[offset + m3].r = scratch[5].r - scratch[4].i;
                    data[offset + m3].i = scratch[5].i + scratch[4].r;

                    offset += 1;
                });
            });
        }
    }

    fn butterfly5(&self, data: &mut [Complex], stride: usize, m: usize, n: usize, mm: usize) {
        // m is guaranteed to be a multiple of 4.
        debug_assert!(m % 4 == 0);

        let mut scratch = [Complex::default(); 13];
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

                scratch[5].r = scratch[0].r + (scratch[7].r * ya.r + scratch[8].r * yb.r);
                scratch[5].i = scratch[0].i + (scratch[7].i * ya.r + scratch[8].i * yb.r);

                scratch[6].r = scratch[10].i * ya.i + scratch[9].i * yb.i;
                scratch[6].i = -(scratch[10].r * ya.i + scratch[9].r * yb.i);

                data[offset1] = scratch[5] - scratch[6];
                data[offset4] = scratch[5] + scratch[6];

                scratch[11].r = scratch[0].r + (scratch[7].r * yb.r + scratch[8].r * ya.r);
                scratch[11].i = scratch[0].i + (scratch[7].i * yb.r + scratch[8].i * ya.r);
                scratch[12].r = scratch[9].i * ya.i - scratch[10].i * yb.i;
                scratch[12].i = scratch[10].r * yb.i - scratch[9].r * ya.i;

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

#[allow(clippy::excessive_precision)]
pub(crate) const FFT_CONFIGURATION: &[KissFft; 4] = &[
    KissFft {
        nfft: 480,
        scale: 0.002083333,
        shift: 0,
        factors: [5, 96, 3, 32, 4, 8, 2, 4, 4, 1, 0, 0, 0, 0, 0, 0],
        bitrev: &BITREV_480,
        twiddles: &TWIDDLES_480000_960,
    },
    KissFft {
        nfft: 240,
        scale: 0.004166667,
        shift: 1,
        factors: [5, 48, 3, 16, 4, 4, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        bitrev: &BITREV_240,
        twiddles: &TWIDDLES_480000_960,
    },
    KissFft {
        nfft: 120,
        scale: 0.008333333,
        shift: 2,
        factors: [5, 24, 3, 8, 2, 4, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        bitrev: &BITREV_120,
        twiddles: &TWIDDLES_480000_960,
    },
    KissFft {
        nfft: 60,
        scale: 0.016666667,
        shift: 3,
        factors: [5, 12, 3, 4, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        bitrev: &BITREV_60,
        twiddles: &TWIDDLES_480000_960,
    },
];

const BITREV_480: &[u16] = &[
    0, 96, 192, 288, 384, 32, 128, 224, 320, 416, 64, 160, 256, 352, 448, 8, 104, 200, 296, 392,
    40, 136, 232, 328, 424, 72, 168, 264, 360, 456, 16, 112, 208, 304, 400, 48, 144, 240, 336, 432,
    80, 176, 272, 368, 464, 24, 120, 216, 312, 408, 56, 152, 248, 344, 440, 88, 184, 280, 376, 472,
    4, 100, 196, 292, 388, 36, 132, 228, 324, 420, 68, 164, 260, 356, 452, 12, 108, 204, 300, 396,
    44, 140, 236, 332, 428, 76, 172, 268, 364, 460, 20, 116, 212, 308, 404, 52, 148, 244, 340, 436,
    84, 180, 276, 372, 468, 28, 124, 220, 316, 412, 60, 156, 252, 348, 444, 92, 188, 284, 380, 476,
    1, 97, 193, 289, 385, 33, 129, 225, 321, 417, 65, 161, 257, 353, 449, 9, 105, 201, 297, 393,
    41, 137, 233, 329, 425, 73, 169, 265, 361, 457, 17, 113, 209, 305, 401, 49, 145, 241, 337, 433,
    81, 177, 273, 369, 465, 25, 121, 217, 313, 409, 57, 153, 249, 345, 441, 89, 185, 281, 377, 473,
    5, 101, 197, 293, 389, 37, 133, 229, 325, 421, 69, 165, 261, 357, 453, 13, 109, 205, 301, 397,
    45, 141, 237, 333, 429, 77, 173, 269, 365, 461, 21, 117, 213, 309, 405, 53, 149, 245, 341, 437,
    85, 181, 277, 373, 469, 29, 125, 221, 317, 413, 61, 157, 253, 349, 445, 93, 189, 285, 381, 477,
    2, 98, 194, 290, 386, 34, 130, 226, 322, 418, 66, 162, 258, 354, 450, 10, 106, 202, 298, 394,
    42, 138, 234, 330, 426, 74, 170, 266, 362, 458, 18, 114, 210, 306, 402, 50, 146, 242, 338, 434,
    82, 178, 274, 370, 466, 26, 122, 218, 314, 410, 58, 154, 250, 346, 442, 90, 186, 282, 378, 474,
    6, 102, 198, 294, 390, 38, 134, 230, 326, 422, 70, 166, 262, 358, 454, 14, 110, 206, 302, 398,
    46, 142, 238, 334, 430, 78, 174, 270, 366, 462, 22, 118, 214, 310, 406, 54, 150, 246, 342, 438,
    86, 182, 278, 374, 470, 30, 126, 222, 318, 414, 62, 158, 254, 350, 446, 94, 190, 286, 382, 478,
    3, 99, 195, 291, 387, 35, 131, 227, 323, 419, 67, 163, 259, 355, 451, 11, 107, 203, 299, 395,
    43, 139, 235, 331, 427, 75, 171, 267, 363, 459, 19, 115, 211, 307, 403, 51, 147, 243, 339, 435,
    83, 179, 275, 371, 467, 27, 123, 219, 315, 411, 59, 155, 251, 347, 443, 91, 187, 283, 379, 475,
    7, 103, 199, 295, 391, 39, 135, 231, 327, 423, 71, 167, 263, 359, 455, 15, 111, 207, 303, 399,
    47, 143, 239, 335, 431, 79, 175, 271, 367, 463, 23, 119, 215, 311, 407, 55, 151, 247, 343, 439,
    87, 183, 279, 375, 471, 31, 127, 223, 319, 415, 63, 159, 255, 351, 447, 95, 191, 287, 383, 479,
];

const BITREV_240: &[u16] = &[
    0, 48, 96, 144, 192, 16, 64, 112, 160, 208, 32, 80, 128, 176, 224, 4, 52, 100, 148, 196, 20,
    68, 116, 164, 212, 36, 84, 132, 180, 228, 8, 56, 104, 152, 200, 24, 72, 120, 168, 216, 40, 88,
    136, 184, 232, 12, 60, 108, 156, 204, 28, 76, 124, 172, 220, 44, 92, 140, 188, 236, 1, 49, 97,
    145, 193, 17, 65, 113, 161, 209, 33, 81, 129, 177, 225, 5, 53, 101, 149, 197, 21, 69, 117, 165,
    213, 37, 85, 133, 181, 229, 9, 57, 105, 153, 201, 25, 73, 121, 169, 217, 41, 89, 137, 185, 233,
    13, 61, 109, 157, 205, 29, 77, 125, 173, 221, 45, 93, 141, 189, 237, 2, 50, 98, 146, 194, 18,
    66, 114, 162, 210, 34, 82, 130, 178, 226, 6, 54, 102, 150, 198, 22, 70, 118, 166, 214, 38, 86,
    134, 182, 230, 10, 58, 106, 154, 202, 26, 74, 122, 170, 218, 42, 90, 138, 186, 234, 14, 62,
    110, 158, 206, 30, 78, 126, 174, 222, 46, 94, 142, 190, 238, 3, 51, 99, 147, 195, 19, 67, 115,
    163, 211, 35, 83, 131, 179, 227, 7, 55, 103, 151, 199, 23, 71, 119, 167, 215, 39, 87, 135, 183,
    231, 11, 59, 107, 155, 203, 27, 75, 123, 171, 219, 43, 91, 139, 187, 235, 15, 63, 111, 159,
    207, 31, 79, 127, 175, 223, 47, 95, 143, 191, 239,
];

const BITREV_120: &[u16] = &[
    0, 24, 48, 72, 96, 8, 32, 56, 80, 104, 16, 40, 64, 88, 112, 4, 28, 52, 76, 100, 12, 36, 60, 84,
    108, 20, 44, 68, 92, 116, 1, 25, 49, 73, 97, 9, 33, 57, 81, 105, 17, 41, 65, 89, 113, 5, 29,
    53, 77, 101, 13, 37, 61, 85, 109, 21, 45, 69, 93, 117, 2, 26, 50, 74, 98, 10, 34, 58, 82, 106,
    18, 42, 66, 90, 114, 6, 30, 54, 78, 102, 14, 38, 62, 86, 110, 22, 46, 70, 94, 118, 3, 27, 51,
    75, 99, 11, 35, 59, 83, 107, 19, 43, 67, 91, 115, 7, 31, 55, 79, 103, 15, 39, 63, 87, 111, 23,
    47, 71, 95, 119,
];

const BITREV_60: &[u16] = &[
    0, 12, 24, 36, 48, 4, 16, 28, 40, 52, 8, 20, 32, 44, 56, 1, 13, 25, 37, 49, 5, 17, 29, 41, 53,
    9, 21, 33, 45, 57, 2, 14, 26, 38, 50, 6, 18, 30, 42, 54, 10, 22, 34, 46, 58, 3, 15, 27, 39, 51,
    7, 19, 31, 43, 55, 11, 23, 35, 47, 59,
];

#[rustfmt::skip]
#[allow(clippy::approx_constant)]
#[allow(clippy::excessive_precision)]
const TWIDDLES_480000_960: &[Complex] = &[
    Complex { r: 1.0000000, i: -0.0000000 }, Complex { r: 0.99991433, i: -0.013089596 },
    Complex { r: 0.99965732, i: -0.026176948 }, Complex { r: 0.99922904, i: -0.039259816 },
    Complex { r: 0.99862953, i: -0.052335956 }, Complex { r: 0.99785892, i: -0.065403129 },
    Complex { r: 0.99691733, i: -0.078459096 }, Complex { r: 0.99580493, i: -0.091501619 },
    Complex { r: 0.99452190, i: -0.10452846 }, Complex { r: 0.99306846, i: -0.11753740 },
    Complex { r: 0.99144486, i: -0.13052619 }, Complex { r: 0.98965139, i: -0.14349262 },
    Complex { r: 0.98768834, i: -0.15643447 }, Complex { r: 0.98555606, i: -0.16934950 },
    Complex { r: 0.98325491, i: -0.18223553 }, Complex { r: 0.98078528, i: -0.19509032 },
    Complex { r: 0.97814760, i: -0.20791169 }, Complex { r: 0.97534232, i: -0.22069744 },
    Complex { r: 0.97236992, i: -0.23344536 }, Complex { r: 0.96923091, i: -0.24615329 },
    Complex { r: 0.96592583, i: -0.25881905 }, Complex { r: 0.96245524, i: -0.27144045 },
    Complex { r: 0.95881973, i: -0.28401534 }, Complex { r: 0.95501994, i: -0.29654157 },
    Complex { r: 0.95105652, i: -0.30901699 }, Complex { r: 0.94693013, i: -0.32143947 },
    Complex { r: 0.94264149, i: -0.33380686 }, Complex { r: 0.93819134, i: -0.34611706 },
    Complex { r: 0.93358043, i: -0.35836795 }, Complex { r: 0.92880955, i: -0.37055744 },
    Complex { r: 0.92387953, i: -0.38268343 }, Complex { r: 0.91879121, i: -0.39474386 },
    Complex { r: 0.91354546, i: -0.40673664 }, Complex { r: 0.90814317, i: -0.41865974 },
    Complex { r: 0.90258528, i: -0.43051110 }, Complex { r: 0.89687274, i: -0.44228869 },
    Complex { r: 0.89100652, i: -0.45399050 }, Complex { r: 0.88498764, i: -0.46561452 },
    Complex { r: 0.87881711, i: -0.47715876 }, Complex { r: 0.87249601, i: -0.48862124 },
    Complex { r: 0.86602540, i: -0.50000000 }, Complex { r: 0.85940641, i: -0.51129309 },
    Complex { r: 0.85264016, i: -0.52249856 }, Complex { r: 0.84572782, i: -0.53361452 },
    Complex { r: 0.83867057, i: -0.54463904 }, Complex { r: 0.83146961, i: -0.55557023 },
    Complex { r: 0.82412619, i: -0.56640624 }, Complex { r: 0.81664156, i: -0.57714519 },
    Complex { r: 0.80901699, i: -0.58778525 }, Complex { r: 0.80125381, i: -0.59832460 },
    Complex { r: 0.79335334, i: -0.60876143 }, Complex { r: 0.78531693, i: -0.61909395 },
    Complex { r: 0.77714596, i: -0.62932039 }, Complex { r: 0.76884183, i: -0.63943900 },
    Complex { r: 0.76040597, i: -0.64944805 }, Complex { r: 0.75183981, i: -0.65934582 },
    Complex { r: 0.74314483, i: -0.66913061 }, Complex { r: 0.73432251, i: -0.67880075 },
    Complex { r: 0.72537437, i: -0.68835458 }, Complex { r: 0.71630194, i: -0.69779046 },
    Complex { r: 0.70710678, i: -0.70710678 }, Complex { r: 0.69779046, i: -0.71630194 },
    Complex { r: 0.68835458, i: -0.72537437 }, Complex { r: 0.67880075, i: -0.73432251 },
    Complex { r: 0.66913061, i: -0.74314483 }, Complex { r: 0.65934582, i: -0.75183981 },
    Complex { r: 0.64944805, i: -0.76040597 }, Complex { r: 0.63943900, i: -0.76884183 },
    Complex { r: 0.62932039, i: -0.77714596 }, Complex { r: 0.61909395, i: -0.78531693 },
    Complex { r: 0.60876143, i: -0.79335334 }, Complex { r: 0.59832460, i: -0.80125381 },
    Complex { r: 0.58778525, i: -0.80901699 }, Complex { r: 0.57714519, i: -0.81664156 },
    Complex { r: 0.56640624, i: -0.82412619 }, Complex { r: 0.55557023, i: -0.83146961 },
    Complex { r: 0.54463904, i: -0.83867057 }, Complex { r: 0.53361452, i: -0.84572782 },
    Complex { r: 0.52249856, i: -0.85264016 }, Complex { r: 0.51129309, i: -0.85940641 },
    Complex { r: 0.50000000, i: -0.86602540 }, Complex { r: 0.48862124, i: -0.87249601 },
    Complex { r: 0.47715876, i: -0.87881711 }, Complex { r: 0.46561452, i: -0.88498764 },
    Complex { r: 0.45399050, i: -0.89100652 }, Complex { r: 0.44228869, i: -0.89687274 },
    Complex { r: 0.43051110, i: -0.90258528 }, Complex { r: 0.41865974, i: -0.90814317 },
    Complex { r: 0.40673664, i: -0.91354546 }, Complex { r: 0.39474386, i: -0.91879121 },
    Complex { r: 0.38268343, i: -0.92387953 }, Complex { r: 0.37055744, i: -0.92880955 },
    Complex { r: 0.35836795, i: -0.93358043 }, Complex { r: 0.34611706, i: -0.93819134 },
    Complex { r: 0.33380686, i: -0.94264149 }, Complex { r: 0.32143947, i: -0.94693013 },
    Complex { r: 0.30901699, i: -0.95105652 }, Complex { r: 0.29654157, i: -0.95501994 },
    Complex { r: 0.28401534, i: -0.95881973 }, Complex { r: 0.27144045, i: -0.96245524 },
    Complex { r: 0.25881905, i: -0.96592583 }, Complex { r: 0.24615329, i: -0.96923091 },
    Complex { r: 0.23344536, i: -0.97236992 }, Complex { r: 0.22069744, i: -0.97534232 },
    Complex { r: 0.20791169, i: -0.97814760 }, Complex { r: 0.19509032, i: -0.98078528 },
    Complex { r: 0.18223553, i: -0.98325491 }, Complex { r: 0.16934950, i: -0.98555606 },
    Complex { r: 0.15643447, i: -0.98768834 }, Complex { r: 0.14349262, i: -0.98965139 },
    Complex { r: 0.13052619, i: -0.99144486 }, Complex { r: 0.11753740, i: -0.99306846 },
    Complex { r: 0.10452846, i: -0.99452190 }, Complex { r: 0.091501619, i: -0.99580493 },
    Complex { r: 0.078459096, i: -0.99691733 }, Complex { r: 0.065403129, i: -0.99785892 },
    Complex { r: 0.052335956, i: -0.99862953 }, Complex { r: 0.039259816, i: -0.99922904 },
    Complex { r: 0.026176948, i: -0.99965732 }, Complex { r: 0.013089596, i: -0.99991433 },
    Complex { r: 6.1230318e-17, i: -1.0000000 }, Complex { r: -0.013089596, i: -0.99991433 },
    Complex { r: -0.026176948, i: -0.99965732 }, Complex { r: -0.039259816, i: -0.99922904 },
    Complex { r: -0.052335956, i: -0.99862953 }, Complex { r: -0.065403129, i: -0.99785892 },
    Complex { r: -0.078459096, i: -0.99691733 }, Complex { r: -0.091501619, i: -0.99580493 },
    Complex { r: -0.10452846, i: -0.99452190 }, Complex { r: -0.11753740, i: -0.99306846 },
    Complex { r: -0.13052619, i: -0.99144486 }, Complex { r: -0.14349262, i: -0.98965139 },
    Complex { r: -0.15643447, i: -0.98768834 }, Complex { r: -0.16934950, i: -0.98555606 },
    Complex { r: -0.18223553, i: -0.98325491 }, Complex { r: -0.19509032, i: -0.98078528 },
    Complex { r: -0.20791169, i: -0.97814760 }, Complex { r: -0.22069744, i: -0.97534232 },
    Complex { r: -0.23344536, i: -0.97236992 }, Complex { r: -0.24615329, i: -0.96923091 },
    Complex { r: -0.25881905, i: -0.96592583 }, Complex { r: -0.27144045, i: -0.96245524 },
    Complex { r: -0.28401534, i: -0.95881973 }, Complex { r: -0.29654157, i: -0.95501994 },
    Complex { r: -0.30901699, i: -0.95105652 }, Complex { r: -0.32143947, i: -0.94693013 },
    Complex { r: -0.33380686, i: -0.94264149 }, Complex { r: -0.34611706, i: -0.93819134 },
    Complex { r: -0.35836795, i: -0.93358043 }, Complex { r: -0.37055744, i: -0.92880955 },
    Complex { r: -0.38268343, i: -0.92387953 }, Complex { r: -0.39474386, i: -0.91879121 },
    Complex { r: -0.40673664, i: -0.91354546 }, Complex { r: -0.41865974, i: -0.90814317 },
    Complex { r: -0.43051110, i: -0.90258528 }, Complex { r: -0.44228869, i: -0.89687274 },
    Complex { r: -0.45399050, i: -0.89100652 }, Complex { r: -0.46561452, i: -0.88498764 },
    Complex { r: -0.47715876, i: -0.87881711 }, Complex { r: -0.48862124, i: -0.87249601 },
    Complex { r: -0.50000000, i: -0.86602540 }, Complex { r: -0.51129309, i: -0.85940641 },
    Complex { r: -0.52249856, i: -0.85264016 }, Complex { r: -0.53361452, i: -0.84572782 },
    Complex { r: -0.54463904, i: -0.83867057 }, Complex { r: -0.55557023, i: -0.83146961 },
    Complex { r: -0.56640624, i: -0.82412619 }, Complex { r: -0.57714519, i: -0.81664156 },
    Complex { r: -0.58778525, i: -0.80901699 }, Complex { r: -0.59832460, i: -0.80125381 },
    Complex { r: -0.60876143, i: -0.79335334 }, Complex { r: -0.61909395, i: -0.78531693 },
    Complex { r: -0.62932039, i: -0.77714596 }, Complex { r: -0.63943900, i: -0.76884183 },
    Complex { r: -0.64944805, i: -0.76040597 }, Complex { r: -0.65934582, i: -0.75183981 },
    Complex { r: -0.66913061, i: -0.74314483 }, Complex { r: -0.67880075, i: -0.73432251 },
    Complex { r: -0.68835458, i: -0.72537437 }, Complex { r: -0.69779046, i: -0.71630194 },
    Complex { r: -0.70710678, i: -0.70710678 }, Complex { r: -0.71630194, i: -0.69779046 },
    Complex { r: -0.72537437, i: -0.68835458 }, Complex { r: -0.73432251, i: -0.67880075 },
    Complex { r: -0.74314483, i: -0.66913061 }, Complex { r: -0.75183981, i: -0.65934582 },
    Complex { r: -0.76040597, i: -0.64944805 }, Complex { r: -0.76884183, i: -0.63943900 },
    Complex { r: -0.77714596, i: -0.62932039 }, Complex { r: -0.78531693, i: -0.61909395 },
    Complex { r: -0.79335334, i: -0.60876143 }, Complex { r: -0.80125381, i: -0.59832460 },
    Complex { r: -0.80901699, i: -0.58778525 }, Complex { r: -0.81664156, i: -0.57714519 },
    Complex { r: -0.82412619, i: -0.56640624 }, Complex { r: -0.83146961, i: -0.55557023 },
    Complex { r: -0.83867057, i: -0.54463904 }, Complex { r: -0.84572782, i: -0.53361452 },
    Complex { r: -0.85264016, i: -0.52249856 }, Complex { r: -0.85940641, i: -0.51129309 },
    Complex { r: -0.86602540, i: -0.50000000 }, Complex { r: -0.87249601, i: -0.48862124 },
    Complex { r: -0.87881711, i: -0.47715876 }, Complex { r: -0.88498764, i: -0.46561452 },
    Complex { r: -0.89100652, i: -0.45399050 }, Complex { r: -0.89687274, i: -0.44228869 },
    Complex { r: -0.90258528, i: -0.43051110 }, Complex { r: -0.90814317, i: -0.41865974 },
    Complex { r: -0.91354546, i: -0.40673664 }, Complex { r: -0.91879121, i: -0.39474386 },
    Complex { r: -0.92387953, i: -0.38268343 }, Complex { r: -0.92880955, i: -0.37055744 },
    Complex { r: -0.93358043, i: -0.35836795 }, Complex { r: -0.93819134, i: -0.34611706 },
    Complex { r: -0.94264149, i: -0.33380686 }, Complex { r: -0.94693013, i: -0.32143947 },
    Complex { r: -0.95105652, i: -0.30901699 }, Complex { r: -0.95501994, i: -0.29654157 },
    Complex { r: -0.95881973, i: -0.28401534 }, Complex { r: -0.96245524, i: -0.27144045 },
    Complex { r: -0.96592583, i: -0.25881905 }, Complex { r: -0.96923091, i: -0.24615329 },
    Complex { r: -0.97236992, i: -0.23344536 }, Complex { r: -0.97534232, i: -0.22069744 },
    Complex { r: -0.97814760, i: -0.20791169 }, Complex { r: -0.98078528, i: -0.19509032 },
    Complex { r: -0.98325491, i: -0.18223553 }, Complex { r: -0.98555606, i: -0.16934950 },
    Complex { r: -0.98768834, i: -0.15643447 }, Complex { r: -0.98965139, i: -0.14349262 },
    Complex { r: -0.99144486, i: -0.13052619 }, Complex { r: -0.99306846, i: -0.11753740 },
    Complex { r: -0.99452190, i: -0.10452846 }, Complex { r: -0.99580493, i: -0.091501619 },
    Complex { r: -0.99691733, i: -0.078459096 }, Complex { r: -0.99785892, i: -0.065403129 },
    Complex { r: -0.99862953, i: -0.052335956 }, Complex { r: -0.99922904, i: -0.039259816 },
    Complex { r: -0.99965732, i: -0.026176948 }, Complex { r: -0.99991433, i: -0.013089596 },
    Complex { r: -1.0000000, i: -1.2246064e-16 }, Complex { r: -0.99991433, i: 0.013089596 },
    Complex { r: -0.99965732, i: 0.026176948 }, Complex { r: -0.99922904, i: 0.039259816 },
    Complex { r: -0.99862953, i: 0.052335956 }, Complex { r: -0.99785892, i: 0.065403129 },
    Complex { r: -0.99691733, i: 0.078459096 }, Complex { r: -0.99580493, i: 0.091501619 },
    Complex { r: -0.99452190, i: 0.10452846 }, Complex { r: -0.99306846, i: 0.11753740 },
    Complex { r: -0.99144486, i: 0.13052619 }, Complex { r: -0.98965139, i: 0.14349262 },
    Complex { r: -0.98768834, i: 0.15643447 }, Complex { r: -0.98555606, i: 0.16934950 },
    Complex { r: -0.98325491, i: 0.18223553 }, Complex { r: -0.98078528, i: 0.19509032 },
    Complex { r: -0.97814760, i: 0.20791169 }, Complex { r: -0.97534232, i: 0.22069744 },
    Complex { r: -0.97236992, i: 0.23344536 }, Complex { r: -0.96923091, i: 0.24615329 },
    Complex { r: -0.96592583, i: 0.25881905 }, Complex { r: -0.96245524, i: 0.27144045 },
    Complex { r: -0.95881973, i: 0.28401534 }, Complex { r: -0.95501994, i: 0.29654157 },
    Complex { r: -0.95105652, i: 0.30901699 }, Complex { r: -0.94693013, i: 0.32143947 },
    Complex { r: -0.94264149, i: 0.33380686 }, Complex { r: -0.93819134, i: 0.34611706 },
    Complex { r: -0.93358043, i: 0.35836795 }, Complex { r: -0.92880955, i: 0.37055744 },
    Complex { r: -0.92387953, i: 0.38268343 }, Complex { r: -0.91879121, i: 0.39474386 },
    Complex { r: -0.91354546, i: 0.40673664 }, Complex { r: -0.90814317, i: 0.41865974 },
    Complex { r: -0.90258528, i: 0.43051110 }, Complex { r: -0.89687274, i: 0.44228869 },
    Complex { r: -0.89100652, i: 0.45399050 }, Complex { r: -0.88498764, i: 0.46561452 },
    Complex { r: -0.87881711, i: 0.47715876 }, Complex { r: -0.87249601, i: 0.48862124 },
    Complex { r: -0.86602540, i: 0.50000000 }, Complex { r: -0.85940641, i: 0.51129309 },
    Complex { r: -0.85264016, i: 0.52249856 }, Complex { r: -0.84572782, i: 0.53361452 },
    Complex { r: -0.83867057, i: 0.54463904 }, Complex { r: -0.83146961, i: 0.55557023 },
    Complex { r: -0.82412619, i: 0.56640624 }, Complex { r: -0.81664156, i: 0.57714519 },
    Complex { r: -0.80901699, i: 0.58778525 }, Complex { r: -0.80125381, i: 0.59832460 },
    Complex { r: -0.79335334, i: 0.60876143 }, Complex { r: -0.78531693, i: 0.61909395 },
    Complex { r: -0.77714596, i: 0.62932039 }, Complex { r: -0.76884183, i: 0.63943900 },
    Complex { r: -0.76040597, i: 0.64944805 }, Complex { r: -0.75183981, i: 0.65934582 },
    Complex { r: -0.74314483, i: 0.66913061 }, Complex { r: -0.73432251, i: 0.67880075 },
    Complex { r: -0.72537437, i: 0.68835458 }, Complex { r: -0.71630194, i: 0.69779046 },
    Complex { r: -0.70710678, i: 0.70710678 }, Complex { r: -0.69779046, i: 0.71630194 },
    Complex { r: -0.68835458, i: 0.72537437 }, Complex { r: -0.67880075, i: 0.73432251 },
    Complex { r: -0.66913061, i: 0.74314483 }, Complex { r: -0.65934582, i: 0.75183981 },
    Complex { r: -0.64944805, i: 0.76040597 }, Complex { r: -0.63943900, i: 0.76884183 },
    Complex { r: -0.62932039, i: 0.77714596 }, Complex { r: -0.61909395, i: 0.78531693 },
    Complex { r: -0.60876143, i: 0.79335334 }, Complex { r: -0.59832460, i: 0.80125381 },
    Complex { r: -0.58778525, i: 0.80901699 }, Complex { r: -0.57714519, i: 0.81664156 },
    Complex { r: -0.56640624, i: 0.82412619 }, Complex { r: -0.55557023, i: 0.83146961 },
    Complex { r: -0.54463904, i: 0.83867057 }, Complex { r: -0.53361452, i: 0.84572782 },
    Complex { r: -0.52249856, i: 0.85264016 }, Complex { r: -0.51129309, i: 0.85940641 },
    Complex { r: -0.50000000, i: 0.86602540 }, Complex { r: -0.48862124, i: 0.87249601 },
    Complex { r: -0.47715876, i: 0.87881711 }, Complex { r: -0.46561452, i: 0.88498764 },
    Complex { r: -0.45399050, i: 0.89100652 }, Complex { r: -0.44228869, i: 0.89687274 },
    Complex { r: -0.43051110, i: 0.90258528 }, Complex { r: -0.41865974, i: 0.90814317 },
    Complex { r: -0.40673664, i: 0.91354546 }, Complex { r: -0.39474386, i: 0.91879121 },
    Complex { r: -0.38268343, i: 0.92387953 }, Complex { r: -0.37055744, i: 0.92880955 },
    Complex { r: -0.35836795, i: 0.93358043 }, Complex { r: -0.34611706, i: 0.93819134 },
    Complex { r: -0.33380686, i: 0.94264149 }, Complex { r: -0.32143947, i: 0.94693013 },
    Complex { r: -0.30901699, i: 0.95105652 }, Complex { r: -0.29654157, i: 0.95501994 },
    Complex { r: -0.28401534, i: 0.95881973 }, Complex { r: -0.27144045, i: 0.96245524 },
    Complex { r: -0.25881905, i: 0.96592583 }, Complex { r: -0.24615329, i: 0.96923091 },
    Complex { r: -0.23344536, i: 0.97236992 }, Complex { r: -0.22069744, i: 0.97534232 },
    Complex { r: -0.20791169, i: 0.97814760 }, Complex { r: -0.19509032, i: 0.98078528 },
    Complex { r: -0.18223553, i: 0.98325491 }, Complex { r: -0.16934950, i: 0.98555606 },
    Complex { r: -0.15643447, i: 0.98768834 }, Complex { r: -0.14349262, i: 0.98965139 },
    Complex { r: -0.13052619, i: 0.99144486 }, Complex { r: -0.11753740, i: 0.99306846 },
    Complex { r: -0.10452846, i: 0.99452190 }, Complex { r: -0.091501619, i: 0.99580493 },
    Complex { r: -0.078459096, i: 0.99691733 }, Complex { r: -0.065403129, i: 0.99785892 },
    Complex { r: -0.052335956, i: 0.99862953 }, Complex { r: -0.039259816, i: 0.99922904 },
    Complex { r: -0.026176948, i: 0.99965732 }, Complex { r: -0.013089596, i: 0.99991433 },
    Complex { r: -1.8369095e-16, i: 1.0000000 }, Complex { r: 0.013089596, i: 0.99991433 },
    Complex { r: 0.026176948, i: 0.99965732 }, Complex { r: 0.039259816, i: 0.99922904 },
    Complex { r: 0.052335956, i: 0.99862953 }, Complex { r: 0.065403129, i: 0.99785892 },
    Complex { r: 0.078459096, i: 0.99691733 }, Complex { r: 0.091501619, i: 0.99580493 },
    Complex { r: 0.10452846, i: 0.99452190 }, Complex { r: 0.11753740, i: 0.99306846 },
    Complex { r: 0.13052619, i: 0.99144486 }, Complex { r: 0.14349262, i: 0.98965139 },
    Complex { r: 0.15643447, i: 0.98768834 }, Complex { r: 0.16934950, i: 0.98555606 },
    Complex { r: 0.18223553, i: 0.98325491 }, Complex { r: 0.19509032, i: 0.98078528 },
    Complex { r: 0.20791169, i: 0.97814760 }, Complex { r: 0.22069744, i: 0.97534232 },
    Complex { r: 0.23344536, i: 0.97236992 }, Complex { r: 0.24615329, i: 0.96923091 },
    Complex { r: 0.25881905, i: 0.96592583 }, Complex { r: 0.27144045, i: 0.96245524 },
    Complex { r: 0.28401534, i: 0.95881973 }, Complex { r: 0.29654157, i: 0.95501994 },
    Complex { r: 0.30901699, i: 0.95105652 }, Complex { r: 0.32143947, i: 0.94693013 },
    Complex { r: 0.33380686, i: 0.94264149 }, Complex { r: 0.34611706, i: 0.93819134 },
    Complex { r: 0.35836795, i: 0.93358043 }, Complex { r: 0.37055744, i: 0.92880955 },
    Complex { r: 0.38268343, i: 0.92387953 }, Complex { r: 0.39474386, i: 0.91879121 },
    Complex { r: 0.40673664, i: 0.91354546 }, Complex { r: 0.41865974, i: 0.90814317 },
    Complex { r: 0.43051110, i: 0.90258528 }, Complex { r: 0.44228869, i: 0.89687274 },
    Complex { r: 0.45399050, i: 0.89100652 }, Complex { r: 0.46561452, i: 0.88498764 },
    Complex { r: 0.47715876, i: 0.87881711 }, Complex { r: 0.48862124, i: 0.87249601 },
    Complex { r: 0.50000000, i: 0.86602540 }, Complex { r: 0.51129309, i: 0.85940641 },
    Complex { r: 0.52249856, i: 0.85264016 }, Complex { r: 0.53361452, i: 0.84572782 },
    Complex { r: 0.54463904, i: 0.83867057 }, Complex { r: 0.55557023, i: 0.83146961 },
    Complex { r: 0.56640624, i: 0.82412619 }, Complex { r: 0.57714519, i: 0.81664156 },
    Complex { r: 0.58778525, i: 0.80901699 }, Complex { r: 0.59832460, i: 0.80125381 },
    Complex { r: 0.60876143, i: 0.79335334 }, Complex { r: 0.61909395, i: 0.78531693 },
    Complex { r: 0.62932039, i: 0.77714596 }, Complex { r: 0.63943900, i: 0.76884183 },
    Complex { r: 0.64944805, i: 0.76040597 }, Complex { r: 0.65934582, i: 0.75183981 },
    Complex { r: 0.66913061, i: 0.74314483 }, Complex { r: 0.67880075, i: 0.73432251 },
    Complex { r: 0.68835458, i: 0.72537437 }, Complex { r: 0.69779046, i: 0.71630194 },
    Complex { r: 0.70710678, i: 0.70710678 }, Complex { r: 0.71630194, i: 0.69779046 },
    Complex { r: 0.72537437, i: 0.68835458 }, Complex { r: 0.73432251, i: 0.67880075 },
    Complex { r: 0.74314483, i: 0.66913061 }, Complex { r: 0.75183981, i: 0.65934582 },
    Complex { r: 0.76040597, i: 0.64944805 }, Complex { r: 0.76884183, i: 0.63943900 },
    Complex { r: 0.77714596, i: 0.62932039 }, Complex { r: 0.78531693, i: 0.61909395 },
    Complex { r: 0.79335334, i: 0.60876143 }, Complex { r: 0.80125381, i: 0.59832460 },
    Complex { r: 0.80901699, i: 0.58778525 }, Complex { r: 0.81664156, i: 0.57714519 },
    Complex { r: 0.82412619, i: 0.56640624 }, Complex { r: 0.83146961, i: 0.55557023 },
    Complex { r: 0.83867057, i: 0.54463904 }, Complex { r: 0.84572782, i: 0.53361452 },
    Complex { r: 0.85264016, i: 0.52249856 }, Complex { r: 0.85940641, i: 0.51129309 },
    Complex { r: 0.86602540, i: 0.50000000 }, Complex { r: 0.87249601, i: 0.48862124 },
    Complex { r: 0.87881711, i: 0.47715876 }, Complex { r: 0.88498764, i: 0.46561452 },
    Complex { r: 0.89100652, i: 0.45399050 }, Complex { r: 0.89687274, i: 0.44228869 },
    Complex { r: 0.90258528, i: 0.43051110 }, Complex { r: 0.90814317, i: 0.41865974 },
    Complex { r: 0.91354546, i: 0.40673664 }, Complex { r: 0.91879121, i: 0.39474386 },
    Complex { r: 0.92387953, i: 0.38268343 }, Complex { r: 0.92880955, i: 0.37055744 },
    Complex { r: 0.93358043, i: 0.35836795 }, Complex { r: 0.93819134, i: 0.34611706 },
    Complex { r: 0.94264149, i: 0.33380686 }, Complex { r: 0.94693013, i: 0.32143947 },
    Complex { r: 0.95105652, i: 0.30901699 }, Complex { r: 0.95501994, i: 0.29654157 },
    Complex { r: 0.95881973, i: 0.28401534 }, Complex { r: 0.96245524, i: 0.27144045 },
    Complex { r: 0.96592583, i: 0.25881905 }, Complex { r: 0.96923091, i: 0.24615329 },
    Complex { r: 0.97236992, i: 0.23344536 }, Complex { r: 0.97534232, i: 0.22069744 },
    Complex { r: 0.97814760, i: 0.20791169 }, Complex { r: 0.98078528, i: 0.19509032 },
    Complex { r: 0.98325491, i: 0.18223553 }, Complex { r: 0.98555606, i: 0.16934950 },
    Complex { r: 0.98768834, i: 0.15643447 }, Complex { r: 0.98965139, i: 0.14349262 },
    Complex { r: 0.99144486, i: 0.13052619 }, Complex { r: 0.99306846, i: 0.11753740 },
    Complex { r: 0.99452190, i: 0.10452846 }, Complex { r: 0.99580493, i: 0.091501619 },
    Complex { r: 0.99691733, i: 0.078459096 }, Complex { r: 0.99785892, i: 0.065403129 },
    Complex { r: 0.99862953, i: 0.052335956 }, Complex { r: 0.99922904, i: 0.039259816 },
    Complex { r: 0.99965732, i: 0.026176948 }, Complex { r: 0.99991433, i: 0.013089596 },
];

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use nanorand::RNG;

    use super::*;

    /// Applies the forward FFT on the given data in `input` and saved the result in `output`.
    fn forward(fft: &KissFft, input: &[Complex], output: &mut [Complex]) {
        // Bit-reverse and scale the input.
        (0..fft.nfft).into_iter().for_each(|i| {
            output[usize::from(fft.bitrev[i])] = input[i] * fft.scale;
        });

        fft.process(output);
    }

    /// Applies the inverse FFT on the given data in `input` and saved the result in `output`.
    fn inverse(fft: &KissFft, input: &[Complex], output: &mut [Complex]) {
        // Bit-reverse the input.
        (0..fft.nfft).into_iter().for_each(|i| {
            output[usize::from(fft.bitrev[i])] = input[i];
        });

        (0..fft.nfft).into_iter().for_each(|i| {
            output[i].i = -output[i].i;
        });

        fft.process(output);

        (0..fft.nfft).into_iter().for_each(|i| {
            output[i].i = -output[i].i;
        });
    }

    fn check(input: &[Complex], output: &[Complex], nfft: usize, is_inverse: bool) {
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

                ansr += fin.r as f64 * re - fin.i as f64 * im;
                ansi += fin.r as f64 * im + fin.i as f64 * re;
            });

            let difr = ansr - fout.r as f64;
            let difi = ansi - fout.i as f64;
            err_pow += difr * difr + difi * difi;
            sig_pow += ansr * ansr + ansi * ansi;
        });

        let snr = 10.0 * (sig_pow / err_pow).log10();
        assert!(
            snr > 130.0,
            "nfft={}, inverse={}, poor snr={}",
            nfft,
            is_inverse,
            snr
        );
    }

    fn test1d(nfft: usize, is_inverse: bool) {
        let mut rng = nanorand::WyRand::new_seed(42);
        let mut input = vec![Complex::default(); nfft];
        let mut output = vec![Complex::default(); nfft];

        let fft = FFT_CONFIGURATION.iter().find(|c| c.nfft == nfft).unwrap();

        input.iter_mut().for_each(|x| {
            x.r = (rng.generate_range::<u32>(0, 32767) as i16 - 16384) as f32;
            x.i = (rng.generate_range::<u32>(0, 32767) as i16 - 16384) as f32;
        });

        input.iter_mut().for_each(|x| {
            *x *= 32768.0;
        });

        if is_inverse {
            input.iter_mut().for_each(|x| {
                x.r /= nfft as f32;
                x.i /= nfft as f32;
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
