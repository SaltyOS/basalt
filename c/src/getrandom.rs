// SPDX-License-Identifier: GPL-2.0-only
//! getrandom(2) — fill buffer with random bytes via RDRAND syscall.

/// Fill a buffer with random bytes from the kernel RDRAND/RNDR source.
///
/// Returns the number of bytes written on success, or -1 on error (sets errno).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, _flags: u32) -> isize {
    unsafe {
        if buf.is_null() {
            crate::errno::set_errno(crate::errno::EFAULT);
            return -1;
        }
        let mut filled = 0usize;
        while filled < buflen {
            let remaining = buflen - filled;
            match trona::syscall::sys_getrandom() {
                Some(val) => {
                    let bytes = val.to_le_bytes();
                    let to_copy = if remaining < 8 { remaining } else { 8 };
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(filled), to_copy);
                    filled += to_copy;
                }
                None => {
                    if filled == 0 {
                        crate::errno::set_errno(crate::errno::EAGAIN);
                        return -1;
                    }
                    break;
                }
            }
        }
        filled as isize
    }
}

// ---------------------------------------------------------------------------
// getentropy — fill buffer with cryptographically secure random bytes
// ---------------------------------------------------------------------------

/// Fill a buffer with cryptographically secure random bytes via RDRAND.
///
/// Uses the kernel `GetRandom` syscall (RDRAND/RNDR-backed) to fill the
/// buffer directly with hardware-sourced entropy. `buflen` must be <= 256
/// per POSIX.
///
/// Returns 0 on success, -1 on error (sets errno).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    if buf.is_null() || buflen > 256 {
        crate::errno::set_errno(crate::errno::EINVAL);
        return -1;
    }

    unsafe {
        let mut filled = 0usize;
        while filled < buflen {
            match trona::syscall::sys_getrandom() {
                Some(val) => {
                    let bytes = val.to_le_bytes();
                    let remaining = buflen - filled;
                    let to_copy = if remaining < 8 { remaining } else { 8 };
                    core::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        buf.add(filled),
                        to_copy,
                    );
                    filled += to_copy;
                }
                None => {
                    crate::errno::set_errno(crate::errno::EIO);
                    return -1;
                }
            }
        }
    }
    0
}
