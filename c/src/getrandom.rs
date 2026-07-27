// SPDX-License-Identifier: GPL-2.0-only
//! getrandom(2) — fill buffer with random bytes via KernelRng.

/// Fill a buffer with random bytes from the kernel RNG source.
///
/// Returns the number of bytes written on success, or -1 on error (sets errno).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, _flags: u32) -> isize {
    if buflen == 0 {
        return 0;
    }
    if buf.is_null() {
        crate::errno::set_errno(crate::errno::EFAULT);
        return -1;
    }
    let r = trona_kernel::syscall::rng_read_bytes(
        trona_runtime::client::caps::kernel_rng_cap().addr(),
        buf,
        buflen,
    );
    if r.error != 0 {
        crate::errno::set_errno(crate::errno::EAGAIN);
        return -1;
    }
    r.value as isize
}

// ---------------------------------------------------------------------------
// getentropy — fill buffer with cryptographically secure random bytes
// ---------------------------------------------------------------------------

/// Fill a buffer with cryptographically secure random bytes via KernelRng.
///
/// Uses the kernel `KernelRng` byte-read invocation to fill the buffer directly
/// with hardware-sourced entropy. `buflen` must be <= 256 per POSIX.
///
/// Returns 0 on success, -1 on error (sets errno).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    if buflen > 256 {
        crate::errno::set_errno(crate::errno::EINVAL);
        return -1;
    }
    if buflen == 0 {
        return 0;
    }
    if buf.is_null() {
        crate::errno::set_errno(crate::errno::EFAULT);
        return -1;
    }

    let r = trona_kernel::syscall::rng_read_bytes(
        trona_runtime::client::caps::kernel_rng_cap().addr(),
        buf,
        buflen,
    );
    if r.error != 0 || r.value != buflen as u64 {
        crate::errno::set_errno(crate::errno::EIO);
        return -1;
    }
    0
}
