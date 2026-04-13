//! PTY helper functions for OpenPAM/sudo-rs style consumers.
//! SPDX-License-Identifier: GPL-2.0-only

use crate::errno;

const PTMX_PATH: &[u8] = b"/dev/ptmx\0";
pub const TTY_PATH_BUF_LEN: usize = 64;

static mut PTSNAME_BUF: [u8; TTY_PATH_BUF_LEN] = [0; TTY_PATH_BUF_LEN];

#[repr(C)]
pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

fn append_u32(mut value: u32, out: &mut [u8], pos: &mut usize) {
    if value == 0 {
        if *pos < out.len() {
            out[*pos] = b'0';
            *pos += 1;
        }
        return;
    }

    let mut tmp = [0u8; 10];
    let mut len = 0usize;
    while value > 0 && len < tmp.len() {
        tmp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    while len > 0 {
        len -= 1;
        if *pos < out.len() {
            out[*pos] = tmp[len];
            *pos += 1;
        }
    }
}

fn build_pts_path(pty_no: u32, out: &mut [u8]) -> Option<usize> {
    let prefix = b"/dev/pts/";
    if out.len() < prefix.len() + 2 {
        return None;
    }

    let mut pos = 0usize;
    for &b in prefix {
        out[pos] = b;
        pos += 1;
    }
    append_u32(pty_no, out, &mut pos);
    if pos >= out.len() {
        return None;
    }
    out[pos] = 0;
    Some(pos)
}

unsafe fn query_pty_no(fd: i32, out: *mut i32) -> i32 {
    unsafe {
        if out.is_null() {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
        let ret = trona_posix::posix_ioctl(fd, trona::consts::posix::TIOCGPTN, out as u64);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grantpt(_fd: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlockpt(_fd: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptsname_r(fd: i32, buf: *mut u8, buflen: usize) -> i32 {
    unsafe {
        if buf.is_null() || buflen == 0 {
            errno::set_errno(errno::EINVAL);
            return errno::EINVAL;
        }

        let mut pty_no = 0i32;
        if query_pty_no(fd, &raw mut pty_no) != 0 {
            return errno::get_errno();
        }

        let out = core::slice::from_raw_parts_mut(buf, buflen);
        let Some(path_len) = build_pts_path(pty_no as u32, out) else {
            return errno::ERANGE;
        };
        if path_len + 1 > buflen {
            return errno::ERANGE;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptsname(fd: i32) -> *mut u8 {
    unsafe {
        let buf = &raw mut PTSNAME_BUF;
        if ptsname_r(fd, (*buf).as_mut_ptr(), (*buf).len()) != 0 {
            return core::ptr::null_mut();
        }
        (*buf).as_mut_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tcgetsid(fd: i32) -> i32 {
    unsafe {
        let ret = trona_posix::posix_ioctl(fd, trona::consts::posix::TIOCGSID, 0);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openpty(
    amaster: *mut i32,
    aslave: *mut i32,
    name: *mut u8,
    termp: *const crate::termios::Termios,
    winp: *const Winsize,
) -> i32 {
    unsafe {
        if amaster.is_null() || aslave.is_null() {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let master = trona_posix::posix_open(
            PTMX_PATH.as_ptr(),
            (trona::consts::posix::O_RDWR | trona::consts::posix::O_NOCTTY) as i32,
            0,
        );
        if master < 0 {
            errno::set_errno(-master);
            return -1;
        }

        let mut pty_no = 0i32;
        if query_pty_no(master, &raw mut pty_no) != 0 {
            let _ = trona_posix::posix_close(master);
            return -1;
        }

        let mut path = [0u8; TTY_PATH_BUF_LEN];
        let Some(path_len) = build_pts_path(pty_no as u32, &mut path) else {
            let _ = trona_posix::posix_close(master);
            errno::set_errno(errno::ERANGE);
            return -1;
        };

        let slave = trona_posix::posix_open(
            path.as_ptr(),
            (trona::consts::posix::O_RDWR | trona::consts::posix::O_NOCTTY) as i32,
            0,
        );
        if slave < 0 {
            let _ = trona_posix::posix_close(master);
            errno::set_errno(-slave);
            return -1;
        }

        if !termp.is_null() {
            let ret = crate::termios::tcsetattr(slave, crate::termios::TCSANOW, termp);
            if ret != 0 {
                let _ = trona_posix::posix_close(slave);
                let _ = trona_posix::posix_close(master);
                return -1;
            }
        }

        if !winp.is_null() {
            let ret = trona_posix::posix_ioctl(slave, trona::consts::posix::TIOCSWINSZ, winp as u64);
            if ret < 0 {
                errno::set_errno(-ret);
                let _ = trona_posix::posix_close(slave);
                let _ = trona_posix::posix_close(master);
                return -1;
            }
        }

        if !name.is_null() {
            core::ptr::copy_nonoverlapping(path.as_ptr(), name, path_len + 1);
        }

        *amaster = master;
        *aslave = slave;
        0
    }
}
