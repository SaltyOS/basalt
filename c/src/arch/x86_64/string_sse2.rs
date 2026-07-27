//! SSE2 string operations for x86_64
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! SSE2 string operation ABI wrappers for x86_64.
//! SPDX-License-Identifier: GPL-2.0-only

unsafe extern "C" {
    fn basaltc_x86_strlen_sse2(s: *const u8) -> usize;
}

/// SSE2-accelerated strlen. Aligns to 16-byte boundary then scans 16 bytes
/// at a time using `pxor`/`pcmpeqb`/`pmovmskb` to detect NUL bytes.
///
/// # Safety
/// `s` must point to a NUL-terminated string. The string must be readable
/// up to the next 16-byte aligned boundary past the NUL terminator (this
/// is guaranteed by `movdqa` operating within the same page after alignment).
#[inline]
#[target_feature(enable = "sse,sse2")]
pub unsafe fn strlen_sse2(s: *const u8) -> usize {
    unsafe { basaltc_x86_strlen_sse2(s) }
}
