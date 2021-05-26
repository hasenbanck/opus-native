//! AVX optimized version.
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
        let g10v = _mm256_set1_ps(g10);
        let g11v = _mm256_set1_ps(g11);
        let g12v = _mm256_set1_ps(g12);

        let mut j = 0;
        (0..n - 7).into_iter().step_by(8).for_each(|i| {
            let mut yi = _mm256_loadu_ps(x[x_offset + i..].as_ptr());

            let x0v = _mm256_loadu_ps(x[x_offset + i - t - 2..].as_ptr());
            let x4v = _mm256_loadu_ps(x[x_offset + i - t + 2..].as_ptr());

            let x2v = _mm256_shuffle_ps(x0v, x4v, 0b01_00_11_10);
            let x1v = _mm256_shuffle_ps(x0v, x2v, 0b10_01_10_01);
            let x3v = _mm256_shuffle_ps(x2v, x4v, 0b10_01_10_01);

            #[cfg(target_feature = "fma")]
            {
                yi = _mm256_fmadd_ps(g10v, x2v, yi);
                yi = _mm256_fmadd_ps(g11v, _mm256_add_ps(x3v, x1v), yi);
                yi = _mm256_fmadd_ps(g12v, _mm256_add_ps(x4v, x0v), yi);
            }
            #[cfg(not(target_feature = "fma"))]
            {
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g10v, x2v));
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g11v, _mm256_add_ps(x3v, x1v)));
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g12v, _mm256_add_ps(x4v, x0v)));
            }

            _mm256_storeu_ps(y[y_offset + i..].as_mut_ptr(), yi);

            j += 8;
        });

        let g10v = _mm256_castps256_ps128(g10v);
        let g11v = _mm256_castps256_ps128(g11v);
        let g12v = _mm256_castps256_ps128(g12v);

        let mut x0v = _mm_loadu_ps(x[x_offset + j - t - 2..].as_ptr());

        if (n - j) != 0 {
            let mut yi = _mm_loadu_ps(x[x_offset + j..].as_ptr());
            let x4v = _mm_loadu_ps(x[x_offset + j - t + 2..].as_ptr());

            let x2v = _mm_shuffle_ps(x0v, x4v, 0b01_00_11_10);
            let x1v = _mm_shuffle_ps(x0v, x2v, 0b10_01_10_01);
            let x3v = _mm_shuffle_ps(x2v, x4v, 0b10_01_10_01);

            #[cfg(target_feature = "fma")]
            {
                yi = _mm_fmadd_ps(g10v, x2v, yi);
                yi = _mm_fmadd_ps(g11v, _mm_add_ps(x3v, x1v), yi);
                yi = _mm_fmadd_ps(g12v, _mm_add_ps(x4v, x0v), yi);
            }
            #[cfg(not(target_feature = "fma"))]
            {
                yi = _mm_add_ps(yi, _mm_mul_ps(g10v, x2v));
                yi = _mm_add_ps(yi, _mm_mul_ps(g11v, _mm_add_ps(x3v, x1v)));
                yi = _mm_add_ps(yi, _mm_mul_ps(g12v, _mm_add_ps(x4v, x0v)));
            }

            x0v = x4v;
            _mm_storeu_ps(y[y_offset + j..].as_mut_ptr(), yi);
        };
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
    // TODO is N always n%8==0? If so, we can remove the SSE code below.

    unsafe {
        let g10v = _mm256_set1_ps(g10);
        let g11v = _mm256_set1_ps(g11);
        let g12v = _mm256_set1_ps(g12);

        let mut j = 0;
        (0..n - 7).into_iter().step_by(8).for_each(|i| {
            let mut yi = _mm256_loadu_ps(y[y_offset + i..].as_ptr());

            let x0v = _mm256_loadu_ps(y[y_offset + i - t - 2..].as_ptr());
            let x4v = _mm256_loadu_ps(y[y_offset + i - t + 2..].as_ptr());

            let x2v = _mm256_shuffle_ps(x0v, x4v, 0b01_00_11_10);
            let x1v = _mm256_shuffle_ps(x0v, x2v, 0b10_01_10_01);
            let x3v = _mm256_shuffle_ps(x2v, x4v, 0b10_01_10_01);

            #[cfg(target_feature = "fma")]
            {
                yi = _mm256_fmadd_ps(g10v, x2v, yi);
                yi = _mm256_fmadd_ps(g11v, _mm256_add_ps(x3v, x1v), yi);
                yi = _mm256_fmadd_ps(g12v, _mm256_add_ps(x4v, x0v), yi);
            }
            #[cfg(not(target_feature = "fma"))]
            {
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g10v, x2v));
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g11v, _mm256_add_ps(x3v, x1v)));
                yi = _mm256_add_ps(yi, _mm256_mul_ps(g12v, _mm256_add_ps(x4v, x0v)));
            }

            _mm256_storeu_ps(y[y_offset + i..].as_mut_ptr(), yi);

            j += 8;
        });

        let g10v = _mm256_castps256_ps128(g10v);
        let g11v = _mm256_castps256_ps128(g11v);
        let g12v = _mm256_castps256_ps128(g12v);

        let mut x0v = _mm_loadu_ps(y[y_offset + j - t - 2..].as_ptr());

        if (n - j) != 0 {
            let mut yi = _mm_loadu_ps(y[y_offset + j..].as_ptr());
            let x4v = _mm_loadu_ps(y[y_offset + j - t + 2..].as_ptr());

            let x2v = _mm_shuffle_ps(x0v, x4v, 0b01_00_11_10);
            let x1v = _mm_shuffle_ps(x0v, x2v, 0b10_01_10_01);
            let x3v = _mm_shuffle_ps(x2v, x4v, 0b10_01_10_01);

            #[cfg(target_feature = "fma")]
            {
                yi = _mm_fmadd_ps(g10v, x2v, yi);
                yi = _mm_fmadd_ps(g11v, _mm_add_ps(x3v, x1v), yi);
                yi = _mm_fmadd_ps(g12v, _mm_add_ps(x4v, x0v), yi);
            }
            #[cfg(not(target_feature = "fma"))]
            {
                yi = _mm_add_ps(yi, _mm_mul_ps(g10v, x2v));
                yi = _mm_add_ps(yi, _mm_mul_ps(g11v, _mm_add_ps(x3v, x1v)));
                yi = _mm_add_ps(yi, _mm_mul_ps(g12v, _mm_add_ps(x4v, x0v)));
            }

            x0v = x4v;
            _mm_storeu_ps(y[y_offset + j..].as_mut_ptr(), yi);
        }
    }
}
