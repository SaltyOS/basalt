//! ttyent — secure tty database compatibility
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! SaltyOS does not ship FreeBSD's tty database stack, but OpenPAM's
//! `pam_securetty` only needs `getttynam()` and the `TTY_SECURE` bit.
//! We read `/etc/ttys` directly and extract `on` / `secure` flags.

use crate::errno;

#[repr(C)]
pub struct Ttyent {
    pub ty_name: *mut u8,
    pub ty_getty: *mut u8,
    pub ty_type: *mut u8,
    pub ty_status: i32,
    pub ty_window: *mut u8,
    pub ty_comment: *mut u8,
    pub ty_group: *mut u8,
}

const TTY_ON: i32 = 0x01;
const TTY_SECURE: i32 = 0x02;
const MAX_LINE: usize = 512;

static mut NAME_BUF: [u8; 128] = [0; 128];
static mut GETTY_BUF: [u8; 128] = [0; 128];
static mut TYPE_BUF: [u8; 128] = [0; 128];
static mut COMMENT_BUF: [u8; 128] = [0; 128];
static mut TTYENT: Ttyent = Ttyent {
    ty_name: core::ptr::null_mut(),
    ty_getty: core::ptr::null_mut(),
    ty_type: core::ptr::null_mut(),
    ty_status: 0,
    ty_window: core::ptr::null_mut(),
    ty_comment: core::ptr::null_mut(),
    ty_group: core::ptr::null_mut(),
};

unsafe fn clear_buf(buf: *mut u8, len: usize) {
    unsafe {
        for i in 0..len {
            *buf.add(i) = 0;
        }
    }
}

unsafe fn copy_token(dst: *mut u8, dst_len: usize, token: *const u8, token_len: usize) {
    unsafe {
        clear_buf(dst, dst_len);
        let n = core::cmp::min(dst_len.saturating_sub(1), token_len);
        core::ptr::copy_nonoverlapping(token, dst, n);
        *dst.add(n) = 0;
    }
}

unsafe fn is_space(ch: u8) -> bool {
    matches!(ch, b' ' | b'\t' | b'\r' | b'\n')
}

unsafe fn next_token(mut p: *const u8) -> Option<(*const u8, usize, *const u8)> {
    unsafe {
        while !p.is_null() && *p != 0 && is_space(*p) {
            p = p.add(1);
        }
        if p.is_null() || *p == 0 || *p == b'#' {
            return None;
        }
        let start = p;
        while *p != 0 && !is_space(*p) && *p != b'#' {
            p = p.add(1);
        }
        Some((start, p.offset_from(start) as usize, p))
    }
}

unsafe fn line_matches(name: *const u8, line: *const u8) -> bool {
    unsafe {
        let Some((tok, len, _)) = next_token(line) else {
            return false;
        };
        crate::string::strncmp(name, tok, len) == 0 && *name.add(len) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getttynam(name: *const u8) -> *mut Ttyent {
    if name.is_null() {
        errno::set_errno(errno::EINVAL);
        return core::ptr::null_mut();
    }

    unsafe {
        let fd = crate::unistd::open(b"/etc/ttys\0".as_ptr(), trona_posix::O_RDONLY as i32);
        if fd < 0 {
            return core::ptr::null_mut();
        }

        let mut line = [0u8; MAX_LINE];
        let mut line_len = 0usize;

        loop {
            let mut ch = 0u8;
            let n = crate::unistd::read(fd, &raw mut ch, 1);
            if n <= 0 {
                break;
            }

            if ch == b'\n' || line_len + 1 >= line.len() {
                line[line_len] = 0;
                if line_matches(name, line.as_ptr()) {
                    let mut status = 0i32;
                    let mut cursor = line.as_ptr();

                    if let Some((tok, len, next)) = next_token(cursor) {
                        copy_token(core::ptr::addr_of_mut!(NAME_BUF) as *mut u8, 128, tok, len);
                        cursor = next;
                    }
                    if let Some((tok, len, next)) = next_token(cursor) {
                        copy_token(core::ptr::addr_of_mut!(GETTY_BUF) as *mut u8, 128, tok, len);
                        cursor = next;
                    }
                    if let Some((tok, len, next)) = next_token(cursor) {
                        copy_token(core::ptr::addr_of_mut!(TYPE_BUF) as *mut u8, 128, tok, len);
                        cursor = next;
                    }

                    while let Some((tok, len, next)) = next_token(cursor) {
                        if len == 2 && crate::string::strncmp(tok, b"on\0".as_ptr(), 2) == 0 {
                            status |= TTY_ON;
                        } else if len == 6
                            && crate::string::strncmp(tok, b"secure\0".as_ptr(), 6) == 0
                        {
                            status |= TTY_SECURE;
                        } else if *tok == b'#' {
                            copy_token(
                                core::ptr::addr_of_mut!(COMMENT_BUF) as *mut u8,
                                128,
                                tok.add(1),
                                len.saturating_sub(1),
                            );
                            break;
                        }
                        cursor = next;
                    }

                    crate::unistd::close(fd);
                    TTYENT.ty_name = core::ptr::addr_of_mut!(NAME_BUF) as *mut u8;
                    TTYENT.ty_getty = core::ptr::addr_of_mut!(GETTY_BUF) as *mut u8;
                    TTYENT.ty_type = core::ptr::addr_of_mut!(TYPE_BUF) as *mut u8;
                    TTYENT.ty_status = status;
                    TTYENT.ty_window = core::ptr::null_mut();
                    TTYENT.ty_comment = core::ptr::addr_of_mut!(COMMENT_BUF) as *mut u8;
                    TTYENT.ty_group = core::ptr::null_mut();
                    return &raw mut TTYENT;
                }
                line_len = 0;
                line.fill(0);
                continue;
            }

            line[line_len] = ch;
            line_len += 1;
        }

        crate::unistd::close(fd);
        core::ptr::null_mut()
    }
}
