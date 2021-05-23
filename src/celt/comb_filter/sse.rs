//! SSE optimized version.
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[inline(always)]
#[allow(unsafe_code)]
#[allow(clippy::too_many_arguments)]
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
    unsafe {
        let g10v = _mm_load1_ps(&g10 as *const f32);
        let g11v = _mm_load1_ps(&g11 as *const f32);
        let g12v = _mm_load1_ps(&g12 as *const f32);
        let mut x0v = _mm_loadu_ps(x[x_offset - t - 2..].as_ptr());

        (0..n - 3).into_iter().step_by(4).for_each(|i| {
            let yi = _mm_loadu_ps(x[x_offset + i..].as_ptr());
            let x4v = _mm_loadu_ps(x[x_offset + i - t + 2..].as_ptr());

            let x2v = _mm_shuffle_ps(x0v, x4v, 0x4e);
            let x1v = _mm_shuffle_ps(x0v, x2v, 0x99);
            let x3v = _mm_shuffle_ps(x2v, x4v, 0x99);

            let yi = _mm_add_ps(yi, _mm_mul_ps(g10v, x2v));

            let yi = _mm_add_ps(yi, _mm_mul_ps(g11v, _mm_add_ps(x3v, x1v)));
            let yi = _mm_add_ps(yi, _mm_mul_ps(g12v, _mm_add_ps(x4v, x0v)));

            x0v = x4v;
            _mm_storeu_ps(y[y_offset + i..].as_mut_ptr(), yi);
        });
    }
}

#[inline(always)]
#[allow(unsafe_code)]
pub(crate) fn comb_filter_const_inplace(
    y: &mut [f32],
    y_offset: usize,
    t: usize,
    n: usize,
    g10: f32,
    g11: f32,
    g12: f32,
) {
    unsafe {
        let g10v = _mm_load1_ps(&g10 as *const f32);
        let g11v = _mm_load1_ps(&g11 as *const f32);
        let g12v = _mm_load1_ps(&g12 as *const f32);
        let mut x0v = _mm_loadu_ps(y[y_offset - t - 2..].as_ptr());

        (0..n - 3).into_iter().step_by(4).for_each(|i| {
            let yi = _mm_loadu_ps(y[y_offset + i..].as_ptr());
            let x4v = _mm_loadu_ps(y[y_offset + i - t + 2..].as_ptr());

            let x2v = _mm_shuffle_ps(x0v, x4v, 0x4e);
            let x1v = _mm_shuffle_ps(x0v, x2v, 0x99);
            let x3v = _mm_shuffle_ps(x2v, x4v, 0x99);

            let yi = _mm_add_ps(yi, _mm_mul_ps(g10v, x2v));

            let yi = _mm_add_ps(yi, _mm_mul_ps(g11v, _mm_add_ps(x3v, x1v)));
            let yi = _mm_add_ps(yi, _mm_mul_ps(g12v, _mm_add_ps(x4v, x0v)));

            x0v = x4v;
            _mm_storeu_ps(y[y_offset + i..].as_mut_ptr(), yi);
        });
    }
}
