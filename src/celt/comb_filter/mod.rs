//! Implements the comb filter.

use crate::celt::mode;

//mod avx;

#[cfg(not(any(
    all(
        target_arch = "x86",
        any(target_feature = "sse", target_feature = "avx")
    ),
    all(
        target_arch = "x86_64",
        any(target_feature = "sse", target_feature = "avx")
    ),
    all(target_arch = "arm", target_feature = "neon", feature = "nightly"),
    all(target_arch = "aarch64", target_feature = "neon", feature = "nightly")
)))]
submodule!(pub(crate) fallback);

#[cfg(any(
    all(
        target_arch = "x86",
        all(target_feature = "sse", not(target_feature = "avx"))
    ),
    all(
        target_arch = "x86_64",
        all(target_feature = "sse", not(target_feature = "avx"))
    ),
))]
submodule!(pub(crate) sse);

#[cfg(any(
    all(target_arch = "x86", target_feature = "avx"),
    all(target_arch = "x86_64", target_feature = "avx")
))]
submodule!(pub(crate) avx);

#[cfg(any(
    all(target_arch = "arm", target_feature = "neon", feature = "nightly"),
    all(target_arch = "aarch64", target_feature = "neon", feature = "nightly")
))]
submodule!(pub(crate) neon);

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
    const SIZE: usize = 256;
    const N: usize = 64;
    const OVERLAP: usize = 4;

    const TEST_VECTOR1: &[f32; N] = &[
        192.0, 193.00005, 194.00035, 195.00134, 320.5, 322.25, 324.0, 325.75, 327.5, 329.25, 331.0,
        332.75, 334.5, 336.25, 338.0, 339.75, 341.5, 343.25, 345.0, 346.75, 348.5, 350.25, 352.0,
        353.75, 355.5, 357.25, 359.0, 360.75, 362.5, 364.25, 366.0, 367.75, 369.5, 371.25, 373.0,
        374.75, 376.5, 378.25, 380.0, 381.75, 383.5, 385.25, 387.0, 388.75, 390.5, 392.25, 394.0,
        395.75, 397.5, 399.25, 401.0, 402.75, 404.5, 406.25, 408.0, 409.75, 411.5, 413.25, 415.0,
        416.75, 418.5, 420.25, 422.0, 423.75,
    ];

    const TEST_VECTOR2: &[f32; N] = &[
        192.0, 193.00005, 194.00035, 195.00134, 320.5, 322.25, 324.0, 325.75, 327.5, 329.25, 331.0,
        332.75, 334.5, 336.25, 338.0, 339.75, 341.5, 343.25, 345.0, 346.75, 348.5, 350.25, 352.0,
        353.75, 355.5, 357.25, 359.0, 360.75, 362.5, 364.25, 366.00003, 367.75018, 381.60532,
        403.69452, 434.27197, 456.65555, 471.0, 473.3125, 475.625, 477.9375, 480.25, 482.5625,
        484.875, 487.1875, 489.5, 491.8125, 494.125, 496.4375, 498.75, 501.0625, 503.375, 505.6875,
        508.0, 510.3125, 512.625, 514.9375, 517.25, 519.5625, 521.875, 524.1875, 527.677,
        533.93762, 545.14777, 560.80713,
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
            assert!((output[offset + i] - TEST_VECTOR1[i]).abs() < f32::EPSILON);
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
            dbg!(output[offset + i]);
            dbg!(TEST_VECTOR2[i]);
            assert!((output[offset + i] - TEST_VECTOR2[i]).abs() < f32::EPSILON);
        });
    }
}
