//! Miscellaneous POSIX functions
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Functions that don't fit in a specific POSIX header category:
//! `dirname`/`basename` (path decomposition), `sched_yield`, `getpagesize`,
//! `fsync`/`fdatasync` (no-ops for ramfs), `utime`/`utimes`,
//! `copy_file_range`, syslog stubs, and `pdfork`.
//!
//! BSD/FreeBSD-specific functions live in `compat::freebsd`.

use crate::errno;
use core::ffi::VaList;
use core::ptr::addr_of_mut;

// ---------------------------------------------------------------------------
// dirname / basename — POSIX string manipulation
// ---------------------------------------------------------------------------

static mut DIRNAME_BUF: [u8; 4096] = [0; 4096];
static mut BASENAME_DOT: [u8; 2] = [b'.', 0];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dirname(path: *mut u8) -> *mut u8 {
    unsafe {
        let buf = addr_of_mut!(DIRNAME_BUF) as *mut u8;

        // NULL or empty → "."
        if path.is_null() || *path == 0 {
            *buf.add(0) = b'.';
            *buf.add(1) = 0;
            return buf;
        }

        // Measure length
        let mut len = 0usize;
        while *path.add(len) != 0 {
            len += 1;
        }

        // Strip trailing slashes
        while len > 1 && *path.add(len - 1) == b'/' {
            len -= 1;
        }

        // Find last slash
        let mut last_slash: isize = -1;
        for i in (0..len).rev() {
            if *path.add(i) == b'/' {
                last_slash = i as isize;
                break;
            }
        }

        if last_slash < 0 {
            // No slash → "."
            *buf.add(0) = b'.';
            *buf.add(1) = 0;
            return buf;
        }

        if last_slash == 0 {
            // Only root slash → "/"
            *buf.add(0) = b'/';
            *buf.add(1) = 0;
            return buf;
        }

        // Strip trailing slashes from parent
        let mut end = last_slash as usize;
        while end > 1 && *path.add(end - 1) == b'/' {
            end -= 1;
        }

        let copy_len = if end < 4095 { end } else { 4095 };
        for i in 0..copy_len {
            *buf.add(i) = *path.add(i);
        }
        *buf.add(copy_len) = 0;
        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn basename(path: *mut u8) -> *mut u8 {
    unsafe {
        let dot = addr_of_mut!(BASENAME_DOT) as *mut u8;

        // NULL or empty → "."
        if path.is_null() || *path == 0 {
            *dot.add(0) = b'.';
            *dot.add(1) = 0;
            return dot;
        }

        let mut len = 0usize;
        while *path.add(len) != 0 {
            len += 1;
        }

        // Strip trailing slashes
        while len > 1 && *path.add(len - 1) == b'/' {
            len -= 1;
        }

        // All slashes → "/"
        if len == 1 && *path == b'/' {
            return path;
        }

        // Find last slash
        let mut last_slash: isize = -1;
        for i in (0..len).rev() {
            if *path.add(i) == b'/' {
                last_slash = i as isize;
                break;
            }
        }

        if last_slash < 0 {
            // Null-terminate at len (strip trailing slashes)
            *path.add(len) = 0;
            return path;
        }

        let start = (last_slash + 1) as usize;
        *path.add(len) = 0;
        path.add(start)
    }
}

// ---------------------------------------------------------------------------
// chroot — not supported
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn chroot(_path: *const u8) -> i32 {
    errno::set_errno(38); // ENOSYS
    -1
}

// ---------------------------------------------------------------------------
// sched_yield — invokes TCB_YIELD on CAP_SELF_TCB.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sched_yield() -> i32 {
    trona_kernel::syscall::yield_now();
    0
}

// ---------------------------------------------------------------------------
// getpagesize — always 4096
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpagesize() -> i32 {
    4096
}

// ---------------------------------------------------------------------------
// fsync / fdatasync — sync file data to persistent storage
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fsync(fd: i32) -> i32 {
    unsafe {
        let ret = trona_posix::posix_fsync(fd);
        if ret < 0 {
            crate::errno::set_errno(-ret);
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fdatasync(fd: i32) -> i32 {
    unsafe { fsync(fd) }
}

// ---------------------------------------------------------------------------
// utime / utimes — thin wrappers over utimensat
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct Utimbuf {
    pub actime: i64,
    pub modtime: i64,
}

#[repr(C)]
pub struct CTimeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utime(filename: *const u8, times: *const Utimbuf) -> i32 {
    unsafe {
        if filename.is_null() {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
        let (atime_sec, atime_nsec, mtime_sec, mtime_nsec) = if times.is_null() {
            // NULL → set both to current time
            let utime_now: i64 = (1 << 30) - 1;
            (0i64, utime_now, 0i64, utime_now)
        } else {
            // Utimbuf has seconds only, nsec = 0
            ((*times).actime, 0i64, (*times).modtime, 0i64)
        };
        let ret = trona_posix::posix_utimensat(
            trona_posix::consts::AT_FDCWD,
            filename,
            atime_sec,
            atime_nsec,
            mtime_sec,
            mtime_nsec,
            0,
        );
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utimes(filename: *const u8, times: *const CTimeval) -> i32 {
    unsafe {
        if filename.is_null() {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
        let (atime_sec, atime_nsec, mtime_sec, mtime_nsec) = if times.is_null() {
            let utime_now: i64 = (1 << 30) - 1;
            (0i64, utime_now, 0i64, utime_now)
        } else {
            // CTimeval has sec + usec, convert usec → nsec
            (
                (*times).tv_sec,
                (*times).tv_usec * 1000,
                (*times.add(1)).tv_sec,
                (*times.add(1)).tv_usec * 1000,
            )
        };
        let ret = trona_posix::posix_utimensat(
            trona_posix::consts::AT_FDCWD,
            filename,
            atime_sec,
            atime_nsec,
            mtime_sec,
            mtime_nsec,
            0,
        );
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

// ---------------------------------------------------------------------------
// copy_file_range — copy data between file descriptors
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_file_range(
    fd_in: i32,
    off_in: *mut i64,
    fd_out: i32,
    off_out: *mut i64,
    len: usize,
    _flags: u32,
) -> isize {
    unsafe {
        // If off_in is provided, seek to that offset (saving current pos)
        let saved_in: i64 = if !off_in.is_null() {
            let cur = trona_posix::posix_lseek(fd_in, 0, trona_posix::consts::SEEK_CUR as i32);
            if cur < 0 {
                errno::set_errno(errno::EBADF);
                return -1;
            }
            let ret =
                trona_posix::posix_lseek(fd_in, *off_in, trona_posix::consts::SEEK_SET as i32);
            if ret < 0 {
                errno::set_errno(errno::EINVAL);
                return -1;
            }
            cur
        } else {
            -1
        };

        let saved_out: i64 = if !off_out.is_null() {
            let cur = trona_posix::posix_lseek(fd_out, 0, trona_posix::consts::SEEK_CUR as i32);
            if cur < 0 {
                if !off_in.is_null() {
                    trona_posix::posix_lseek(fd_in, saved_in, trona_posix::consts::SEEK_SET as i32);
                }
                errno::set_errno(errno::EBADF);
                return -1;
            }
            let ret =
                trona_posix::posix_lseek(fd_out, *off_out, trona_posix::consts::SEEK_SET as i32);
            if ret < 0 {
                if !off_in.is_null() {
                    trona_posix::posix_lseek(fd_in, saved_in, trona_posix::consts::SEEK_SET as i32);
                }
                errno::set_errno(errno::EINVAL);
                return -1;
            }
            cur
        } else {
            -1
        };

        // Read/write loop with stack buffer
        let mut buf = [0u8; 4096];
        let mut total: usize = 0;

        while total < len {
            let chunk = if len - total < 4096 {
                len - total
            } else {
                4096
            };
            let nr = trona_posix::posix_read(fd_in, buf.as_mut_ptr(), chunk as u64);
            if nr < 0 {
                if total == 0 {
                    if !off_in.is_null() {
                        trona_posix::posix_lseek(
                            fd_in,
                            saved_in,
                            trona_posix::consts::SEEK_SET as i32,
                        );
                    }
                    if !off_out.is_null() {
                        trona_posix::posix_lseek(
                            fd_out,
                            saved_out,
                            trona_posix::consts::SEEK_SET as i32,
                        );
                    }
                    errno::set_errno(errno::EIO);
                    return -1;
                }
                break;
            }
            if nr == 0 {
                break; // EOF
            }

            let mut written: usize = 0;
            while written < nr as usize {
                let nw = trona_posix::posix_write(
                    fd_out,
                    buf.as_ptr().add(written),
                    (nr as usize - written) as u64,
                );
                if nw < 0 {
                    if total == 0 && written == 0 {
                        if !off_in.is_null() {
                            trona_posix::posix_lseek(
                                fd_in,
                                saved_in,
                                trona_posix::consts::SEEK_SET as i32,
                            );
                        }
                        if !off_out.is_null() {
                            trona_posix::posix_lseek(
                                fd_out,
                                saved_out,
                                trona_posix::consts::SEEK_SET as i32,
                            );
                        }
                        errno::set_errno(errno::EIO);
                        return -1;
                    }
                    total += written;
                    if !off_in.is_null() {
                        *off_in += total as i64;
                        trona_posix::posix_lseek(
                            fd_in,
                            saved_in,
                            trona_posix::consts::SEEK_SET as i32,
                        );
                    }
                    if !off_out.is_null() {
                        *off_out += total as i64;
                        trona_posix::posix_lseek(
                            fd_out,
                            saved_out,
                            trona_posix::consts::SEEK_SET as i32,
                        );
                    }
                    return total as isize;
                }
                if nw == 0 {
                    break;
                }
                written += nw as usize;
            }
            total += written;
        }

        // Update offset pointers and restore file positions
        if !off_in.is_null() {
            *off_in += total as i64;
            trona_posix::posix_lseek(fd_in, saved_in, trona_posix::consts::SEEK_SET as i32);
        }
        if !off_out.is_null() {
            *off_out += total as i64;
            trona_posix::posix_lseek(fd_out, saved_out, trona_posix::consts::SEEK_SET as i32);
        }

        total as isize
    }
}

// ---------------------------------------------------------------------------
// syslog stubs — route messages to stderr until a real syslog service exists.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openlog(_ident: *const u8, _option: i32, _facility: i32) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn closelog() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setlogmask(mask: i32) -> i32 {
    let _ = mask;
    0xFF // Accept all priorities
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn syslog(_priority: i32, fmt: *const u8, args: ...) {
    if fmt.is_null() {
        return;
    }
    crate::stdio::ensure_stdio_init();
    unsafe {
        let _ = crate::stdio::vfprintf(crate::stdio::stderr, fmt, args);
        let _ = trona_posix::posix_write(2, b"\n".as_ptr(), 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsyslog(_priority: i32, fmt: *const u8, ap: VaList<'_>) {
    if fmt.is_null() {
        return;
    }
    crate::stdio::ensure_stdio_init();
    unsafe {
        let _ = crate::stdio::vfprintf(crate::stdio::stderr, fmt, ap);
        let _ = trona_posix::posix_write(2, b"\n".as_ptr(), 1);
    }
}

// ---------------------------------------------------------------------------
// pdfork — FreeBSD Capsicum process descriptor fork (stub)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfork(_fdp: *mut i32, _flags: i32) -> i32 {
    errno::set_errno(38); // ENOSYS
    -1
}

// ---------------------------------------------------------------------------
// syscall — variadic raw syscall interface
// ---------------------------------------------------------------------------
//
// The historical libc `syscall(2)` symbol is tied to a multi-syscall
// kernel ABI where each kernel operation has a dedicated syscall number.
// kernite exposes a single `KERNITE_SYS_INVOKE` trap and routes every
// operation through capability invocation, so there is no stable mapping
// from a Linux/glibc syscall number to a kernite operation.
//
// The symbol is intentionally absent here. Callers that historically
// reached for `syscall(SYS_futex, ...)` (libcxxabi) must be rewired to
// the substrate futex helpers (`trona_kernel::syscall::futex_wait/wake`) or
// the relevant capability invocation wrapper directly.

// ---------------------------------------------------------------------------
// mkstemps — mkstemp with suffix length
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkstemps(template: *mut u8, suffixlen: i32) -> i32 {
    if template.is_null() {
        errno::set_errno(errno::EINVAL);
        return -1;
    }
    unsafe {
        let len = crate::string::strlen(template);
        let xs_end = if suffixlen >= 0 && (suffixlen as usize) < len {
            len - suffixlen as usize
        } else {
            len
        };
        // Find XXXXXX before suffix
        let mut xs = 0usize;
        let mut i = xs_end;
        while i > 0 && *template.add(i - 1) == b'X' {
            xs += 1;
            i -= 1;
        }
        if xs < 6 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
        let start = xs_end - xs;

        use core::sync::atomic::{AtomicU64, Ordering};
        static CTR: AtomicU64 = AtomicU64::new(0);
        const CHARS: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let max_attempts: u32 = 256;

        let mut attempt: u32 = 0;
        while attempt < max_attempts {
            let mut val = CTR.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
            let mut j = start;
            while j < xs_end {
                *template.add(j) = CHARS[(val % 36) as usize];
                val /= 36;
                j += 1;
            }
            let fd = trona_posix::posix_open(
                template,
                (trona_posix::O_RDWR | trona_posix::O_CREAT | trona_posix::O_EXCL) as i32,
                0o600,
            );
            if fd >= 0 {
                return fd as i32;
            }
            attempt += 1;
        }
        errno::set_errno(errno::EEXIST);
        -1
    }
}

// ---------------------------------------------------------------------------
// setproctitle — set process title (no-op on SaltyOS)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setproctitle(_fmt: *const u8, mut _args: ...) {}

// ---------------------------------------------------------------------------
// catopen / catgets / catclose — POSIX message catalogs (nl_types.h)
// ---------------------------------------------------------------------------

pub type NlCatd = isize;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn catopen(_name: *const u8, _oflag: i32) -> NlCatd {
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn catgets(
    _catd: NlCatd,
    _set_id: i32,
    _msg_id: i32,
    s: *const u8,
) -> *const u8 {
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn catclose(_catd: NlCatd) -> i32 {
    0
}

// ---------------------------------------------------------------------------
// __b64_ntop / __b64_pton — Base64 encode/decode (resolv.h, RFC 4648)
// ---------------------------------------------------------------------------

const B64_ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const fn build_b64_decode_table() -> [u8; 256] {
    let mut t = [0xFFu8; 256];
    let mut i = 0u8;
    loop {
        if i >= 64 {
            break;
        }
        t[B64_ENCODE[i as usize] as usize] = i;
        i += 1;
    }
    t[b'=' as usize] = 0xFE;
    t
}

const B64_DECODE: [u8; 256] = build_b64_decode_table();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __b64_ntop(
    src: *const u8,
    srclength: usize,
    target: *mut u8,
    targsize: usize,
) -> i32 {
    unsafe {
        let needed = ((srclength + 2) / 3) * 4 + 1;
        if targsize < needed {
            return -1;
        }

        let mut si = 0usize;
        let mut di = 0usize;

        while si + 2 < srclength {
            let a = *src.add(si) as u32;
            let b = *src.add(si + 1) as u32;
            let c = *src.add(si + 2) as u32;
            let n = (a << 16) | (b << 8) | c;
            *target.add(di) = B64_ENCODE[((n >> 18) & 0x3F) as usize];
            *target.add(di + 1) = B64_ENCODE[((n >> 12) & 0x3F) as usize];
            *target.add(di + 2) = B64_ENCODE[((n >> 6) & 0x3F) as usize];
            *target.add(di + 3) = B64_ENCODE[(n & 0x3F) as usize];
            si += 3;
            di += 4;
        }

        let rem = srclength - si;
        if rem == 1 {
            let a = *src.add(si) as u32;
            let n = a << 16;
            *target.add(di) = B64_ENCODE[((n >> 18) & 0x3F) as usize];
            *target.add(di + 1) = B64_ENCODE[((n >> 12) & 0x3F) as usize];
            *target.add(di + 2) = b'=';
            *target.add(di + 3) = b'=';
            di += 4;
        } else if rem == 2 {
            let a = *src.add(si) as u32;
            let b = *src.add(si + 1) as u32;
            let n = (a << 16) | (b << 8);
            *target.add(di) = B64_ENCODE[((n >> 18) & 0x3F) as usize];
            *target.add(di + 1) = B64_ENCODE[((n >> 12) & 0x3F) as usize];
            *target.add(di + 2) = B64_ENCODE[((n >> 6) & 0x3F) as usize];
            *target.add(di + 3) = b'=';
            di += 4;
        }

        *target.add(di) = 0;
        di as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __b64_pton(src: *const u8, target: *mut u8, targsize: usize) -> i32 {
    unsafe {
        let mut si = 0usize;
        let mut di = 0usize;
        let mut buf: u32 = 0;
        let mut bits: u32 = 0;

        loop {
            let ch = *src.add(si);
            if ch == 0 {
                break;
            }
            si += 1;

            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                continue;
            }
            if ch == b'=' {
                break;
            }

            let val = B64_DECODE[ch as usize];
            if val == 0xFF {
                return -1;
            }

            buf = (buf << 6) | val as u32;
            bits += 6;

            if bits >= 8 {
                bits -= 8;
                if di >= targsize {
                    return -1;
                }
                *target.add(di) = ((buf >> bits) & 0xFF) as u8;
                di += 1;
            }
        }

        di as i32
    }
}

// ---------------------------------------------------------------------------
// Unwind stubs — SaltyOS uses panic=abort so unwinding never runs,
// but Rust std's backtrace code references these symbols.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn basaltc_abort_trap() -> !;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Unwind_Backtrace(
    _trace: unsafe extern "C" fn(*mut u8, *mut u8) -> i32,
    _data: *mut u8,
) -> i32 {
    5 // _URC_END_OF_STACK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Unwind_GetIP(_context: *mut u8) -> usize {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Unwind_FindEnclosingFunction(_pc: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Unwind_RaiseException(_exception: *mut u8) -> i32 {
    unsafe { basaltc_abort_trap() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Unwind_Resume(_exception: *mut u8) {
    unsafe { basaltc_abort_trap() }
}
