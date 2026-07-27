//! SSE2 math ABI wrappers for x86_64.
//! SPDX-License-Identifier: GPL-2.0-only

unsafe extern "C" {
    fn basaltc_x86_sqrt_sse2_f64(x: f64) -> f64;
    fn basaltc_x86_sqrt_sse2_f32(x: f32) -> f32;
}

/// Double-precision square root via `sqrtsd`.
///
/// # Safety
/// Caller must ensure SSE2 is available (guaranteed on all x86_64 CPUs).
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn sqrt_sse2(x: f64) -> f64 {
    unsafe { basaltc_x86_sqrt_sse2_f64(x) }
}

/// Single-precision square root via `sqrtss`.
///
/// # Safety
/// Caller must ensure SSE2 is available (guaranteed on all x86_64 CPUs).
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn sqrtf_sse2(x: f32) -> f32 {
    unsafe { basaltc_x86_sqrt_sse2_f32(x) }
}
