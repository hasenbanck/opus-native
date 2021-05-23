//! Pure rust version.

#[inline(always)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::many_single_char_names)]
pub(crate) fn comb_filter_const(
    y: &mut [f32],
    y_offset: usize,
    x: &[f32],
    x_offset: usize,
    t: usize,
    n: usize,
    g10: f32,
    g11: f32,
    g12: f32,
) {
    let mut x4 = x[x_offset - t - 2];
    let mut x3 = x[x_offset - t - 1];
    let mut x2 = x[x_offset - t];
    let mut x1 = x[x_offset - t + 1];
    (0..n).into_iter().for_each(|i| {
        let x0 = x[x_offset + i - t + 2];
        y[y_offset + i] = x[x_offset + i] + (g10 * x2) + (g11 * (x1 + x3)) + (g12 * (x0 + x4));
        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;
    });
}

#[inline(always)]
pub(crate) fn comb_filter_const_inplace(
    y: &mut [f32],
    y_offset: usize,
    t: usize,
    n: usize,
    g10: f32,
    g11: f32,
    g12: f32,
) {
    let mut x4 = y[y_offset - t - 2];
    let mut x3 = y[y_offset - t - 1];
    let mut x2 = y[y_offset - t];
    let mut x1 = y[y_offset - t + 1];
    (0..n).into_iter().for_each(|i| {
        let x0 = y[y_offset + i - t + 2];
        y[y_offset + i] = y[y_offset + i] + (g10 * x2) + (g11 * (x1 + x3)) + (g12 * (x0 + x4));
        x4 = x3;
        x3 = x2;
        x2 = x1;
        x1 = x0;
    });
}
