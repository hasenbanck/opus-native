//! NEON optimized version.
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "arm")]
use std::arch::arm::*;

// Pretty naive conversion from the SSE version.
// Around 5-10% speedup on RPi4 compared to the fallback.

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
        let g10v = vld1q_dup_f32(&g10 as *const f32);
        let g11v = vld1q_dup_f32(&g11 as *const f32);
        let g12v = vld1q_dup_f32(&g12 as *const f32);
        let mut x0v = vld1q_f32(x[x_offset - t - 2..].as_ptr());

        (0..n - 3).into_iter().step_by(4).for_each(|i| {
            let yi = vld1q_f32(x[x_offset + i..].as_ptr());

            let xp = x_offset + i - t - 2;
            let x4v = vld1q_f32(x[xp + 4..].as_ptr());

            let x1v = vextq_f32(x0v, x4v, 1);
            let x2v = vextq_f32(x0v, x4v, 2);
            let x3v = vextq_f32(x0v, x4v, 3);

            let yi = vaddq_f32(yi, vmulq_f32(g10v, x2v));

            let yi = vaddq_f32(yi, vmulq_f32(g11v, vaddq_f32(x3v, x1v)));
            let yi = vaddq_f32(yi, vmulq_f32(g12v, vaddq_f32(x4v, x0v)));

            x0v = x4v;
            let yi: [f32; 4] = std::mem::transmute(yi);
            y[y_offset + i..y_offset + i + 4].copy_from_slice(&yi);
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
        let g10v = vld1q_dup_f32(&g10 as *const f32);
        let g11v = vld1q_dup_f32(&g11 as *const f32);
        let g12v = vld1q_dup_f32(&g12 as *const f32);
        let mut x0v = vld1q_f32(y[y_offset - t - 2..].as_ptr());

        (0..n - 3).into_iter().step_by(4).for_each(|i| {
            let yi = vld1q_f32(y[y_offset + i..].as_ptr());

            let yp = y_offset + i - t - 2;
            let x4v = vld1q_f32(y[yp + 4..].as_ptr());

            let x1v = vextq_f32(x0v, x4v, 1);
            let x2v = vextq_f32(x0v, x4v, 2);
            let x3v = vextq_f32(x0v, x4v, 3);

            let yi = vaddq_f32(yi, vmulq_f32(g10v, x2v));

            let yi = vaddq_f32(yi, vmulq_f32(g11v, vaddq_f32(x3v, x1v)));
            let yi = vaddq_f32(yi, vmulq_f32(g12v, vaddq_f32(x4v, x0v)));

            x0v = x4v;
            let yi: [f32; 4] = std::mem::transmute(yi);
            y[y_offset + i..y_offset + i + 4].copy_from_slice(&yi);
        });
    }
}
