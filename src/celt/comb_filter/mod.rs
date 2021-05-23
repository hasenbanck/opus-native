//! Implements the comb filter.

use crate::celt::mode;

#[cfg(not(any(
    all(target_arch = "x86", target_feature = "sse"),
    all(target_arch = "x86_64", target_feature = "sse"),
    all(target_arch = "arm", target_feature = "neon", feature = "nightly"),
    all(target_arch = "aarch64", target_feature = "neon", feature = "nightly")
)))]
submodule!(pub fallback);

#[cfg(any(
    all(target_arch = "x86", target_feature = "sse"),
    all(target_arch = "x86_64", target_feature = "sse")
))]
submodule!(pub sse);

#[cfg(any(
    all(target_arch = "arm", target_feature = "neon", feature = "nightly"),
    all(target_arch = "aarch64", target_feature = "neon", feature = "nightly")
))]
submodule!(pub neon);

const COMBFILTER_MINPERIOD: usize = 15;

const GAINS: [f32; 9] = [
    0.30664063,
    0.21704102,
    0.12963867,
    0.4638672,
    0.2680664,
    0.0,
    0.7998047,
    0.100097656,
    0.0,
];

#[allow(clippy::too_many_arguments)]
#[allow(clippy::many_single_char_names)]
pub(crate) fn comb_filter(
    y: &mut [f32],
    y_offset: usize,
    x: &[f32],
    x_offset: usize,
    mut t0: usize,
    mut t1: usize,
    n: usize,
    g0: f32,
    g1: f32,
    tapset0: usize,
    tapset1: usize,
    mut overlap: usize,
) {
    if g0 == 0.0 && g1 == 0.0 {
        y[y_offset..y_offset + n].copy_from_slice(&x[x_offset..x_offset + n]);
        return;
    }

    // When the gain is zero, t0 and/or t1 is set to zero.
    // We need to have then be at least 2 to avoid processing garbage data.
    t0 = usize::max(t0, COMBFILTER_MINPERIOD);
    t1 = usize::max(t1, COMBFILTER_MINPERIOD);

    let g00 = g0 * GAINS[tapset0 * 3];
    let g01 = g0 * GAINS[tapset0 * 3 + 1];
    let g02 = g0 * GAINS[tapset0 * 3 + 2];
    let g10 = g1 * GAINS[tapset1 * 3];
    let g11 = g1 * GAINS[tapset1 * 3 + 1];
    let g12 = g1 * GAINS[tapset1 * 3 + 2];

    let mut x1 = x[x_offset - t1 + 1];
    let mut x2 = x[x_offset - t1];
    let mut x3 = x[x_offset - t1 - 1];
    let mut x4 = x[x_offset - t1 - 2];

    // If the filter didn't change, we don't need the overlap.
    if (g0 - g1).abs() < f32::EPSILON && t0 == t1 && tapset0 == tapset1 {
        overlap = 0;
    }

    let mut j = 0;
    (0..overlap).into_iter().for_each(|i| {
        let x0 = x[x_offset + i - t1 + 2];
        let f = mode::WINDOW[i] * mode::WINDOW[i];
        y[y_offset + i] = x[x_offset + i]
            + (((1.0 - f) * g00) * x[x_offset + i - t0])
            + (((1.0 - f) * g01) * (x[x_offset + i - t0 + 1] + x[x_offset + i - t0 - 1]))
            + (((1.0 - f) * g02) * (x[x_offset + i - t0 + 2] + x[x_offset + i - t0 - 2]))
            + ((f * g10) * x2)
            + ((f * g11) * (x1 + x3))
            + ((f * g12) * (x0 + x4));

        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;

        j += 1;
    });

    if g1 == 0.0 {
        y[y_offset + overlap..y_offset + n].copy_from_slice(&x[x_offset + overlap..x_offset + n]);
        return;
    }

    // Compute the part with the constant filter.
    comb_filter_const(y, y_offset + j, x, x_offset + j, t1, n - j, g10, g11, g12);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn comb_filter_inplace(
    y: &mut [f32],
    y_offset: usize,
    mut t0: usize,
    mut t1: usize,
    n: usize,
    g0: f32,
    g1: f32,
    tapset0: usize,
    tapset1: usize,
    mut overlap: usize,
) {
    if g0 == 0.0 && g1 == 0.0 {
        return;
    }

    // When the gain is zero, t0 and/or t1 is set to zero.
    // We need to have then be at least 2 to avoid processing garbage data.
    t0 = usize::max(t0, COMBFILTER_MINPERIOD);
    t1 = usize::max(t1, COMBFILTER_MINPERIOD);

    let g00 = g0 * GAINS[tapset0 * 3];
    let g01 = g0 * GAINS[tapset0 * 3 + 1];
    let g02 = g0 * GAINS[tapset0 * 3 + 2];
    let g10 = g1 * GAINS[tapset1 * 3];
    let g11 = g1 * GAINS[tapset1 * 3 + 1];
    let g12 = g1 * GAINS[tapset1 * 3 + 2];

    let mut x1 = y[y_offset - t1 + 1];
    let mut x2 = y[y_offset - t1];
    let mut x3 = y[y_offset - t1 - 1];
    let mut x4 = y[y_offset - t1 - 2];

    // If the filter didn't change, we don't need the overlap.
    if (g0 - g1).abs() < f32::EPSILON && t0 == t1 && tapset0 == tapset1 {
        overlap = 0;
    }

    let mut j = 0;
    (0..overlap).into_iter().for_each(|i| {
        let x0 = y[y_offset + i - t1 + 2];
        let f = mode::WINDOW[i] * mode::WINDOW[i];
        y[y_offset + i] = y[y_offset + i]
            + (((1.0 - f) * g00) * y[y_offset + i - t0])
            + (((1.0 - f) * g01) * (y[y_offset + i - t0 + 1] + y[y_offset + i - t0 - 1]))
            + (((1.0 - f) * g02) * (y[y_offset + i - t0 + 2] + y[y_offset + i - t0 - 2]))
            + ((f * g10) * x2)
            + ((f * g11) * (x1 + x3))
            + ((f * g12) * (x0 + x4));
        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;

        j += 1;
    });

    if g1 == 0.0 {
        return;
    }

    // Compute the part with the constant filter.
    comb_filter_const_inplace(y, y_offset + j, t1, n - j, g10, g11, g12);
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: usize = 15;
    const T1: usize = 30;
    const G0: f32 = 0.0;
    const G1: f32 = 0.75;
    const SIZE: usize = 48;
    const N: usize = 16;
    const OVERLAP: usize = 4;

    const TEST_VECTOR: &[f32; N] = &[
        32.0, 33.0, 34.00001, 35.000042, 40.5, 42.25, 44.0, 45.75, 47.5, 49.25, 51.0, 52.75, 54.5,
        56.25, 58.0, 59.75,
    ];

    #[test]
    fn test_comb_filter() {
        let mut output = [0_f32; SIZE];
        let mut input = [0_f32; SIZE];
        input
            .iter_mut()
            .enumerate()
            .for_each(|(i, x)| *x = i as f32);

        let offset = SIZE - N;
        comb_filter(
            &mut output,
            offset,
            &input,
            offset,
            T0,
            T1,
            N,
            G0,
            G1,
            0,
            0,
            OVERLAP,
        );

        (0..N).into_iter().for_each(|i| {
            assert!((output[offset + i] - TEST_VECTOR[i]).abs() < (TEST_VECTOR[i] * 0.01));
        });
    }

    #[test]
    fn test_comb_filter_inplace() {
        let mut output = [0_f32; SIZE];
        output
            .iter_mut()
            .enumerate()
            .for_each(|(i, x)| *x = i as f32);

        let offset = SIZE - N;
        comb_filter_inplace(&mut output, offset, T0, T1, N, G0, G1, 0, 0, OVERLAP);

        (0..N).into_iter().for_each(|i| {
            assert!((output[offset + i] - TEST_VECTOR[i]).abs() < (TEST_VECTOR[i] * 0.01));
        });
    }
}
