//! ioctl system call wrapper
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Forwards ioctl requests to the VFS/TTYD via posix_ioctl.

use crate::errno;
use trona::consts::posix::TIOCSPGRP;

static mut LOGGED_IOCTL_CALLS: u8 = 0;

#[inline(always)]
unsafe fn ioctl_request_arg(request: u64, raw_arg: u64) -> u64 {
    match request {
        TIOCSPGRP if raw_arg != 0 => unsafe {
            // SAFETY: libc callers pass TIOCSPGRP as a pointer to pid_t.
            let tty_pgrp = *(raw_arg as *const i32);
            tty_pgrp as u64
        },
        _ => raw_arg,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(fd: i32, request: u64, mut args: ...) -> i32 {
    let raw_arg: u64 = unsafe { args.arg::<u64>() };
    let request_arg = unsafe { ioctl_request_arg(request, raw_arg) };
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

    let ret = unsafe { trona_posix::posix_ioctl(fd, request, request_arg) };
    if ret < 0 {
        errno::set_errno(-ret);
        return -1;
    }
    ret
}
