//! x87 FPU math ABI wrappers for x86_64.
//! SPDX-License-Identifier: GPL-2.0-only

// x87 control word rounding modes (bits 10-11)
const CW_ROUND_NEAREST: u16 = 0x0000;
const CW_ROUND_DOWN: u16 = 0x0400;
const CW_ROUND_UP: u16 = 0x0800;
const CW_ROUND_TRUNC: u16 = 0x0C00;

unsafe extern "C" {
    fn basaltc_x86_x87_round_mode(x: f64, mode: u16) -> f64;
    fn basaltc_x86_x87_sqrt(x: f64) -> f64;
    fn basaltc_x86_x87_sin(x: f64) -> f64;
    fn basaltc_x86_x87_cos(x: f64) -> f64;
    fn basaltc_x86_x87_tan(x: f64) -> f64;
    fn basaltc_x86_x87_atan2(y: f64, x: f64) -> f64;
    fn basaltc_x86_x87_log2(x: f64) -> f64;
    fn basaltc_x86_x87_log(x: f64) -> f64;
    fn basaltc_x86_x87_log10(x: f64) -> f64;
    fn basaltc_x86_x87_exp2(x: f64) -> f64;
    fn basaltc_x86_x87_exp(x: f64) -> f64;
    fn basaltc_x86_x87_fmod(x: f64, y: f64) -> f64;
    fn basaltc_x86_x87_remainder(x: f64, y: f64) -> f64;
    fn basaltc_x86_x87_fma(x: f64, y: f64, z: f64) -> f64;
    fn basaltc_x86_x87_scalbn(x: f64, n: i32) -> f64;
}

/// x87 frndint with specific rounding mode.
pub(crate) fn x87_round_mode(x: f64, mode: u16) -> f64 {
    unsafe { basaltc_x86_x87_round_mode(x, mode) }
}

#[inline]
pub fn floor_x87(x: f64) -> f64 {
    x87_round_mode(x, CW_ROUND_DOWN)
}

#[inline]
pub fn ceil_x87(x: f64) -> f64 {
    x87_round_mode(x, CW_ROUND_UP)
}

#[inline]
pub fn trunc_x87(x: f64) -> f64 {
    x87_round_mode(x, CW_ROUND_TRUNC)
}

#[inline]
pub fn rint_x87(x: f64) -> f64 {
    x87_round_mode(x, CW_ROUND_NEAREST)
}

#[inline]
pub fn sqrt_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_sqrt(x) }
}

#[inline]
pub fn sin_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_sin(x) }
}

#[inline]
pub fn cos_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_cos(x) }
}

#[inline]
pub fn tan_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_tan(x) }
}

#[inline]
pub fn atan2_x87(y: f64, x: f64) -> f64 {
    unsafe { basaltc_x86_x87_atan2(y, x) }
}

#[inline]
pub fn log2_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_log2(x) }
}

#[inline]
pub fn log_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_log(x) }
}

#[inline]
pub fn log10_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_log10(x) }
}

#[inline]
pub fn exp2_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_exp2(x) }
}

#[inline]
pub fn exp_x87(x: f64) -> f64 {
    unsafe { basaltc_x86_x87_exp(x) }
}

#[inline]
pub fn fmod_x87(x: f64, y: f64) -> f64 {
    unsafe { basaltc_x86_x87_fmod(x, y) }
}

#[inline]
pub fn remainder_x87(x: f64, y: f64) -> f64 {
    unsafe { basaltc_x86_x87_remainder(x, y) }
}

#[inline]
pub fn fma_x87(x: f64, y: f64, z: f64) -> f64 {
    unsafe { basaltc_x86_x87_fma(x, y, z) }
}

#[inline]
pub fn scalbn_x87(x: f64, n: i32) -> f64 {
    unsafe { basaltc_x86_x87_scalbn(x, n) }
}

#[inline]
pub fn pow_x87(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return 1.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    if x == 0.0 {
        return if y > 0.0 { 0.0 } else { f64::INFINITY };
    }
    if x < 0.0 {
        let yi = y as i64;
        if y != yi as f64 {
            return f64::NAN;
        }
        let r = exp2_x87(y * log2_x87(-x));
        return if yi & 1 != 0 { -r } else { r };
    }
    exp2_x87(y * log2_x87(x))
}
