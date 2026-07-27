//! SSE2 memory operations for x86_64
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! SSE2 memory operation ABI wrappers for x86_64.
//! SPDX-License-Identifier: GPL-2.0-only

unsafe extern "C" {
    fn basaltc_x86_memcpy_sse2(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn basaltc_x86_memset_sse2(s: *mut u8, c: i32, n: usize) -> *mut u8;
    fn basaltc_x86_memmove_sse2(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn basaltc_x86_memcmp_sse2(s1: *const u8, s2: *const u8, n: usize) -> i32;
    fn basaltc_x86_memchr_sse2(s: *const u8, c: i32, n: usize) -> *mut u8;
}

/// SSE2-accelerated memcpy. Uses 16-byte `movdqu` loads/stores for the
/// bulk of the copy, with byte-level handling for the tail.
///
/// # Safety
/// `dst` and `src` must be valid for `n` bytes. Regions must not overlap
/// (use `memmove_sse2` for overlapping regions).
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn memcpy_sse2(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe { basaltc_x86_memcpy_sse2(dst, src, n) }
}

/// SSE2-accelerated memset. Broadcasts the byte value to a 128-bit XMM
/// register and writes 16 bytes at a time.
///
/// # Safety
/// `s` must be valid for `n` bytes.
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn memset_sse2(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    unsafe { basaltc_x86_memset_sse2(s, c, n) }
}

/// SSE2-accelerated memmove. Forward copy when dst < src (uses memcpy_sse2),
/// backward 16-byte copy otherwise.
///
/// # Safety
/// `dst` and `src` must be valid for `n` bytes. Regions may overlap.
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn memmove_sse2(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe { basaltc_x86_memmove_sse2(dst, src, n) }
}

/// SSE2-accelerated memcmp. Compares 16 bytes at a time using `pcmpeqb`
/// + `pmovmskb`, falling back to byte comparison on mismatch.
///
/// # Safety
/// `s1` and `s2` must be valid for `n` bytes.
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn memcmp_sse2(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { basaltc_x86_memcmp_sse2(s1, s2, n) }
}

/// SSE2-accelerated memchr. Broadcasts the search byte and uses `pcmpeqb`
/// + `pmovmskb` to scan 16 bytes at a time.
///
/// # Safety
/// `s` must be valid for `n` bytes.
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn memchr_sse2(s: *const u8, c: i32, n: usize) -> *mut u8 {
    unsafe { basaltc_x86_memchr_sse2(s, c, n) }
}
