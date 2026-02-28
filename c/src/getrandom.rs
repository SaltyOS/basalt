// SPDX-License-Identifier: GPL-2.0-only
//! getrandom(2) — fill buffer with random bytes via RDRAND syscall.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, _flags: u32) -> isize {
    unsafe {
        if buf.is_null() {
            return -14; // EFAULT
        }
        let mut filled = 0usize;
        while filled < buflen {
            let remaining = buflen - filled;
            match salty::syscall::sys_getrandom() {
                Some(val) => {
                    let bytes = val.to_le_bytes();
                    let to_copy = if remaining < 8 { remaining } else { 8 };
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(filled), to_copy);
                    filled += to_copy;
                }
                None => {
                    if filled == 0 {
                        return -11; // EAGAIN
                    }
                    break;
                }
            }
        }
        filled as isize
    }
}
