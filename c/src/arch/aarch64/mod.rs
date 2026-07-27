//! AArch64 architecture implementations
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Provides `AArch64Arch` which implements `ArchMath`, `ArchMem`, and
//! `ArchString`. Hardware FP instructions live in `src/arch/aarch64/math.S`;
//! Rust owns only the ABI wrappers and scalar software fallbacks.

use super::ArchMath;

pub struct AArch64Arch;

unsafe extern "C" {
    fn basaltc_aarch64_sqrt_f64(x: f64) -> f64;
    fn basaltc_aarch64_sqrt_f32(x: f32) -> f32;
    fn basaltc_aarch64_floor_f64(x: f64) -> f64;
    fn basaltc_aarch64_ceil_f64(x: f64) -> f64;
    fn basaltc_aarch64_trunc_f64(x: f64) -> f64;
    fn basaltc_aarch64_rint_f64(x: f64) -> f64;
}

// ============================================================================
// ArchMath — hardware FP instructions + scalar software transcendentals
// ============================================================================

impl super::ArchMath for AArch64Arch {
    #[inline]
    fn sqrt(x: f64) -> f64 {
        unsafe { basaltc_aarch64_sqrt_f64(x) }
    }

    #[inline]
    fn sqrtf(x: f32) -> f32 {
        unsafe { basaltc_aarch64_sqrt_f32(x) }
    }

    #[inline]
    fn floor(x: f64) -> f64 {
        unsafe { basaltc_aarch64_floor_f64(x) }
    }

    #[inline]
    fn ceil(x: f64) -> f64 {
        unsafe { basaltc_aarch64_ceil_f64(x) }
    }

    #[inline]
    fn trunc(x: f64) -> f64 {
        unsafe { basaltc_aarch64_trunc_f64(x) }
    }

    #[inline]
    fn rint(x: f64) -> f64 {
        unsafe { basaltc_aarch64_rint_f64(x) }
    }

    #[inline]
    fn sin(x: f64) -> f64 {
        sw_sin(x)
    }
    #[inline]
    fn cos(x: f64) -> f64 {
        sw_cos(x)
    }
    #[inline]
    fn tan(x: f64) -> f64 {
        sw_tan(x)
    }
    #[inline]
    fn atan2(y: f64, x: f64) -> f64 {
        sw_atan2(y, x)
    }
    #[inline]
    fn log(x: f64) -> f64 {
        sw_log(x)
    }
    #[inline]
    fn log2(x: f64) -> f64 {
        sw_log2(x)
    }
    #[inline]
    fn log10(x: f64) -> f64 {
        sw_log10(x)
    }
    #[inline]
    fn exp(x: f64) -> f64 {
        sw_exp(x)
    }
    #[inline]
    fn exp2(x: f64) -> f64 {
        sw_exp2(x)
    }
    #[inline]
    fn pow(x: f64, y: f64) -> f64 {
        sw_pow(x, y)
    }
    #[inline]
    fn fmod(x: f64, y: f64) -> f64 {
        sw_fmod(x, y)
    }
    #[inline]
    fn remainder(x: f64, y: f64) -> f64 {
        sw_remainder(x, y)
    }
    #[inline]
    fn fma(x: f64, y: f64, z: f64) -> f64 {
        sw_fma(x, y, z)
    }
    #[inline]
    fn scalbn(x: f64, n: i32) -> f64 {
        sw_scalbn(x, n)
    }
}

// ============================================================================
// ArchMem — scalar byte-by-byte implementations
// ============================================================================

impl super::ArchMem for AArch64Arch {
    #[inline]
    unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        // SAFETY: Caller guarantees dst and src are valid for n bytes and do
        // not overlap. We copy byte-by-byte in forward order.
        unsafe {
            let mut i = 0;
            while i < n {
                *dst.add(i) = *src.add(i);
                i += 1;
            }
            dst
        }
    }

    #[inline]
    unsafe fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
        // SAFETY: Caller guarantees s is valid for n bytes. We write the low
        // byte of c to each position.
        unsafe {
            let val = c as u8;
            let mut i = 0;
            while i < n {
                *s.add(i) = val;
                i += 1;
            }
            s
        }
    }

    #[inline]
    unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        // SAFETY: Caller guarantees dst and src are valid for n bytes (regions
        // may overlap). We copy forward if dst < src, backward otherwise, to
        // avoid overwriting source data before it is read.
        unsafe {
            if (dst as usize) < (src as usize) {
                let mut i = 0;
                while i < n {
                    *dst.add(i) = *src.add(i);
                    i += 1;
                }
            } else {
                let mut i = n;
                while i > 0 {
                    i -= 1;
                    *dst.add(i) = *src.add(i);
                }
            }
            dst
        }
    }

    #[inline]
    unsafe fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
        // SAFETY: Caller guarantees s1 and s2 are valid for n bytes. We compare
        // byte-by-byte and return the first difference.
        unsafe {
            let mut i = 0;
            while i < n {
                let a = *s1.add(i);
                let b = *s2.add(i);
                if a != b {
                    return a as i32 - b as i32;
                }
                i += 1;
            }
            0
        }
    }

    #[inline]
    unsafe fn memchr(s: *const u8, c: i32, n: usize) -> *mut u8 {
        // SAFETY: Caller guarantees s is valid for n bytes. We scan byte-by-byte
        // for the first occurrence of the low byte of c.
        unsafe {
            let val = c as u8;
            let mut i = 0;
            while i < n {
                if *s.add(i) == val {
                    return s.add(i) as *mut u8;
                }
                i += 1;
            }
            core::ptr::null_mut()
        }
    }
}

// ============================================================================
// ArchString — scalar implementation
// ============================================================================

impl super::ArchString for AArch64Arch {
    #[inline]
    unsafe fn strlen(s: *const u8) -> usize {
        // SAFETY: Caller guarantees s points to a NUL-terminated string. We
        // scan byte-by-byte until we find the NUL terminator.
        unsafe {
            let mut len = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            len
        }
    }
}

// ============================================================================
// Software scalar math — transcendental functions
// ============================================================================
//
// These are minimal software implementations for transcendental functions that
// have no AArch64 hardware instruction. They provide reasonable accuracy for
// most inputs but are not fully IEEE 754-compliant edge-case machines. The
// priority is compilation and correctness for common inputs.

const PI: f64 = 3.14159265358979323846;
const FRAC_PI_2: f64 = 1.5707963267948966;
const LN2: f64 = 0.6931471805599453;
const LOG2_E: f64 = 1.4426950408889634;
const LOG10_2: f64 = 0.3010299957316877;

// ---- Helpers ---------------------------------------------------------------

/// Reinterpret f64 as u64 (bit cast).
#[inline]
fn f64_to_bits(x: f64) -> u64 {
    x.to_bits()
}

/// Reinterpret u64 as f64 (bit cast).
#[inline]
fn f64_from_bits(b: u64) -> f64 {
    f64::from_bits(b)
}

/// Extract the biased exponent from a f64.
#[inline]
fn f64_exponent(x: f64) -> i32 {
    ((f64_to_bits(x) >> 52) & 0x7FF) as i32
}

/// Return true if x is NaN.
#[inline]
fn is_nan(x: f64) -> bool {
    x != x
}

/// Return true if x is +inf or -inf.
#[inline]
fn is_inf(x: f64) -> bool {
    f64_to_bits(x) & 0x7FFF_FFFF_FFFF_FFFF == 0x7FF0_0000_0000_0000
}

// ---- Range reduction for trig (Cody-Waite style) ---------------------------

/// Reduce x to [-pi/2, pi/2], returning (reduced, quadrant mod 4).
fn reduce_trig(x: f64) -> (f64, i32) {
    let dp1 = 1.5707963267948966;
    let q = (x * (1.0 / FRAC_PI_2)) as i32;
    let qf = q as f64;
    let r = x - qf * dp1;
    (r, q & 3)
}

// ---- sin -------------------------------------------------------------------

fn sw_sin(x: f64) -> f64 {
    if is_nan(x) || is_inf(x) {
        return f64::NAN;
    }
    let neg = x < 0.0;
    let ax = if neg { -x } else { x };
    let (r, q) = reduce_trig(ax);
    let r2 = r * r;

    // Minimax polynomial for sin(r) on [-pi/2, pi/2]
    let s = r
        * (1.0
            + r2 * (-0.16666666666666666
                + r2 * (0.008333333333333312
                    + r2 * (-0.0001984126984126985 + r2 * 2.7557319223985893e-06))));
    // Minimax polynomial for cos(r) on [-pi/2, pi/2]
    let c = 1.0
        + r2 * (-0.5
            + r2 * (0.041666666666666664
                + r2 * (-0.001388888888888889 + r2 * 2.48015873015873e-05)));

    let val = match q {
        0 => s,
        1 => c,
        2 => -s,
        3 => -c,
        _ => s,
    };
    if neg { -val } else { val }
}

// ---- cos -------------------------------------------------------------------

fn sw_cos(x: f64) -> f64 {
    if is_nan(x) || is_inf(x) {
        return f64::NAN;
    }
    let ax = if x < 0.0 { -x } else { x };
    let (r, q) = reduce_trig(ax);
    let r2 = r * r;

    let s = r
        * (1.0
            + r2 * (-0.16666666666666666
                + r2 * (0.008333333333333312
                    + r2 * (-0.0001984126984126985 + r2 * 2.7557319223985893e-06))));
    let c = 1.0
        + r2 * (-0.5
            + r2 * (0.041666666666666664
                + r2 * (-0.001388888888888889 + r2 * 2.48015873015873e-05)));

    match q {
        0 => c,
        1 => -s,
        2 => -c,
        3 => s,
        _ => c,
    }
}

// ---- tan -------------------------------------------------------------------

fn sw_tan(x: f64) -> f64 {
    let s = sw_sin(x);
    let c = sw_cos(x);
    if c == 0.0 {
        if s > 0.0 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        }
    } else {
        s / c
    }
}

// ---- atan2 -----------------------------------------------------------------

/// Scalar atan(x) via polynomial approximation for |x| <= 1.
fn sw_atan(x: f64) -> f64 {
    // For |x| > 1, use identity: atan(x) = pi/2 - atan(1/x)
    if is_nan(x) {
        return f64::NAN;
    }
    let neg = x < 0.0;
    let ax = if neg { -x } else { x };
    let (r, offset) = if ax > 1.0 {
        (1.0 / ax, FRAC_PI_2)
    } else {
        (ax, 0.0)
    };
    let r2 = r * r;
    // Polynomial approximation: atan(r) ~ r - r^3/3 + r^5/5 - r^7/7 + ...
    let v = r
        * (1.0
            + r2 * (-0.3333333333333333
                + r2 * (0.2
                    + r2 * (-0.14285714285714285
                        + r2 * (0.1111111111111111 + r2 * (-0.09090909090909091))))));
    let result = if ax > 1.0 { offset - v } else { v };
    if neg { -result } else { result }
}

fn sw_atan2(y: f64, x: f64) -> f64 {
    if is_nan(x) || is_nan(y) {
        return f64::NAN;
    }
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    if x > 0.0 {
        return sw_atan(y / x);
    }
    if x < 0.0 {
        if y >= 0.0 {
            return sw_atan(y / x) + PI;
        } else {
            return sw_atan(y / x) - PI;
        }
    }
    // x == 0
    if y > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 }
}

// ---- log (natural) ---------------------------------------------------------

fn sw_log(x: f64) -> f64 {
    if is_nan(x) || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if is_inf(x) {
        return f64::INFINITY;
    }
    // Decompose x = m * 2^e, where 0.5 <= m < 1.0
    // Then ln(x) = ln(m) + e*ln(2)
    let bits = f64_to_bits(x);
    let e = f64_exponent(x) - 1023;
    // Normalize mantissa to [1.0, 2.0)
    let m_bits = (bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000;
    let m = f64_from_bits(m_bits);

    // ln(m) for m in [1, 2) using the substitution t = (m-1)/(m+1)
    // ln(m) = 2*t*(1 + t^2/3 + t^4/5 + t^6/7 + ...)
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let ln_m = 2.0
        * t
        * (1.0
            + t2 * (0.3333333333333333
                + t2 * (0.2
                    + t2 * (0.14285714285714285
                        + t2 * (0.1111111111111111 + t2 * 0.09090909090909091)))));

    ln_m + (e as f64) * LN2
}

// ---- log2 ------------------------------------------------------------------

fn sw_log2(x: f64) -> f64 {
    sw_log(x) * LOG2_E
}

// ---- log10 -----------------------------------------------------------------

fn sw_log10(x: f64) -> f64 {
    sw_log(x) * LOG10_2 * LOG2_E
    // Equivalently: sw_log(x) / LN10, but LOG10_2 * LOG2_E == 1/LN10
    // Actually: log10(x) = log2(x) * log10(2) = ln(x) * log2(e) * log10(2)
}

// ---- exp -------------------------------------------------------------------

fn sw_exp(x: f64) -> f64 {
    if is_nan(x) {
        return f64::NAN;
    }
    if x > 709.0 {
        return f64::INFINITY;
    }
    if x < -745.0 {
        return 0.0;
    }
    // e^x = 2^(x * log2(e))
    sw_exp2(x * LOG2_E)
}

// ---- exp2 ------------------------------------------------------------------

fn sw_exp2(x: f64) -> f64 {
    if is_nan(x) {
        return f64::NAN;
    }
    if x > 1023.0 {
        return f64::INFINITY;
    }
    if x < -1074.0 {
        return 0.0;
    }
    // Split x into integer n and fractional f, where x = n + f, -0.5 <= f < 0.5
    // Use hardware frintx for the nearest-integer part
    let n = AArch64Arch::rint(x);
    let f = x - n;
    let ni = n as i64;

    // 2^f for |f| <= 0.5 via polynomial (minimax on [-0.5, 0.5])
    // 2^f ~ 1 + f*ln(2) + (f*ln(2))^2/2! + ...
    let ln2f = f * LN2;
    let p = 1.0
        + ln2f
            * (1.0
                + ln2f
                    * (0.5
                        + ln2f
                            * (0.16666666666666666
                                + ln2f * (0.041666666666666664 + ln2f * 0.008333333333333333))));

    // Scale by 2^n via exponent manipulation
    let bias = 1023_i64;
    let exp_bits = ((ni + bias) as u64) << 52;
    let scale = f64_from_bits(exp_bits);
    p * scale
}

// ---- pow -------------------------------------------------------------------

fn sw_pow(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return 1.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    if is_nan(x) || is_nan(y) {
        return f64::NAN;
    }
    if x == 0.0 {
        return if y > 0.0 { 0.0 } else { f64::INFINITY };
    }
    if x < 0.0 {
        let yi = y as i64;
        if y != yi as f64 {
            return f64::NAN;
        }
        let r = sw_exp2(y * sw_log2(-x));
        return if yi & 1 != 0 { -r } else { r };
    }
    sw_exp2(y * sw_log2(x))
}

// ---- fmod ------------------------------------------------------------------

fn sw_fmod(x: f64, y: f64) -> f64 {
    if y == 0.0 || is_nan(x) || is_nan(y) || is_inf(x) {
        return f64::NAN;
    }
    if is_inf(y) {
        return x;
    }
    let ay = if y < 0.0 { -y } else { y };
    let mut r = if x < 0.0 { -x } else { x };
    // Repeated subtraction (correct but slow for extreme ratios)
    while r >= ay {
        r -= ay;
    }
    if x < 0.0 { -r } else { r }
}

// ---- remainder -------------------------------------------------------------

fn sw_remainder(x: f64, y: f64) -> f64 {
    if y == 0.0 || is_nan(x) || is_nan(y) || is_inf(x) {
        return f64::NAN;
    }
    if is_inf(y) {
        return x;
    }
    // IEEE remainder: x - n*y where n = round(x/y)
    let n = AArch64Arch::rint(x / y);
    x - n * y
}

// ---- fma -------------------------------------------------------------------

fn sw_fma(x: f64, y: f64, z: f64) -> f64 {
    // Simple unfused multiply-add. Not truly fused (no extended precision for
    // the intermediate product), but sufficient for compilation and basic use.
    x * y + z
}

// ---- scalbn ----------------------------------------------------------------

fn sw_scalbn(x: f64, n: i32) -> f64 {
    if n == 0 || x == 0.0 || is_nan(x) || is_inf(x) {
        return x;
    }
    // x * 2^n via exponent manipulation
    let bits = f64_to_bits(x);
    let exp = f64_exponent(x);
    let new_exp = exp + n;
    if new_exp >= 2047 {
        return if x > 0.0 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
    }
    if new_exp <= 0 {
        return 0.0;
    }
    let new_bits = (bits & 0x800F_FFFF_FFFF_FFFF) | ((new_exp as u64) << 52);
    f64_from_bits(new_bits)
}
