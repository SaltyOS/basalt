//! ioctl system call wrapper
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Forwards ioctl requests to the VFS/TTYD via posix_ioctl.

use crate::errno;

static mut LOGGED_IOCTL_CALLS: u8 = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(fd: i32, request: u64, mut args: ...) -> i32 {
    let arg: u64 = unsafe { args.arg::<u64>() };
    unsafe {
        if *(&raw const LOGGED_IOCTL_CALLS) < 24 {
            *(&raw mut LOGGED_IOCTL_CALLS) += 1;
            trona::udebug!(|_lb| {
                _lb.str(b"[libc] ioctl fd=");
                _lb.dec(fd as u64);
                _lb.str(b" req=");
                _lb.hex(request);
                _lb.putc(b'\n');
            });
        }
    }

    let ret = unsafe { trona_posix::posix_ioctl(fd, request, arg) };
    if ret < 0 {
        errno::set_errno(-ret);
        return -1;
    }
    ret
}
