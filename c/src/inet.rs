// SPDX-License-Identifier: GPL-2.0-only
//! inet address conversion functions
//!
//! Pure string <-> binary conversions for IPv4 (and stub IPv6) addresses.
//! No IPC or networking required — these are pure computation.

use crate::errno;

/// Legacy DNS resolver error code (POSIX).
#[unsafe(no_mangle)]
pub static mut h_errno: i32 = 0;

const AF_INET: i32 = 2;

// ---------------------------------------------------------------------------
// inet_pton — parse a dotted-decimal string into a binary in_addr
// ---------------------------------------------------------------------------

/// Convert a presentation-format address string into network binary form.
///
/// Only AF_INET (IPv4) is implemented. Returns 1 on success, 0 if the input
/// string is not a valid address, -1 with errno=EAFNOSUPPORT for unknown af.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn inet_pton(af: i32, src: *const u8, dst: *mut u8) -> i32 {
    if src.is_null() || dst.is_null() {
        return 0;
    }
    if af != AF_INET {
        errno::set_errno(errno::EAFNOSUPPORT);
        return -1;
    }
    unsafe { inet_pton4(src, dst) }
}

/// Parse "a.b.c.d" into 4 bytes stored in network byte order at `dst`.
unsafe fn inet_pton4(src: *const u8, dst: *mut u8) -> i32 {
    let mut octets: [u8; 4] = [0; 4];
    let mut octet_idx = 0usize;
    let mut digit_count = 0u32;
    let mut value = 0u32;
    let mut i = 0usize;

    loop {
        // SAFETY: caller ensures src is a valid C string; we stop at NUL.
        let ch = unsafe { *src.add(i) };
        i += 1;

        match ch {
            b'0'..=b'9' => {
                value = value * 10 + (ch - b'0') as u32;
                if value > 255 {
                    return 0;
                }
                digit_count += 1;
                if digit_count > 3 {
                    return 0;
                }
            }
            b'.' | 0 => {
                if digit_count == 0 {
                    return 0;
                }
                if octet_idx >= 4 {
                    return 0;
                }
                octets[octet_idx] = value as u8;
                octet_idx += 1;
                value = 0;
                digit_count = 0;
                if ch == 0 {
                    break;
                }
            }
            _ => return 0,
        }
    }

    if octet_idx != 4 {
        return 0;
    }

    // Store in network byte order (big-endian)
    unsafe {
        *dst.add(0) = octets[0];
        *dst.add(1) = octets[1];
        *dst.add(2) = octets[2];
        *dst.add(3) = octets[3];
    }
    1
}

// ---------------------------------------------------------------------------
// inet_ntop — format a binary in_addr into a dotted-decimal string
// ---------------------------------------------------------------------------

/// Convert a binary network address into a presentation-format string.
///
/// Only AF_INET (IPv4) is implemented. Returns `dst` on success, null on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn inet_ntop(
    af: i32,
    src: *const u8,
    dst: *mut u8,
    size: u32,
) -> *const u8 {
    if src.is_null() || dst.is_null() {
        errno::set_errno(errno::EINVAL);
        return core::ptr::null();
    }
    if af != AF_INET {
        errno::set_errno(errno::EAFNOSUPPORT);
        return core::ptr::null();
    }
    unsafe { inet_ntop4(src, dst, size) }
}

/// Format 4 network-order bytes as "a.b.c.d\0" into `dst[0..size]`.
unsafe fn inet_ntop4(src: *const u8, dst: *mut u8, size: u32) -> *const u8 {
    // Maximum IPv4 string: "255.255.255.255\0" = 16 bytes
    const MAX_LEN: u32 = 16;
    if size < MAX_LEN {
        errno::set_errno(errno::ENOSPC);
        return core::ptr::null();
    }

    // SAFETY: caller checked src non-null; 4 bytes are valid for AF_INET.
    let a = unsafe { *src.add(0) };
    let b = unsafe { *src.add(1) };
    let c = unsafe { *src.add(2) };
    let d = unsafe { *src.add(3) };

    let mut pos = 0usize;

    // Write each octet, separated by dots, into dst.
    for (octet_idx, octet) in [a, b, c, d].iter().enumerate() {
        if octet_idx > 0 {
            unsafe { *dst.add(pos) = b'.' };
            pos += 1;
        }
        pos += write_u8_decimal(dst, pos, *octet);
    }

    // NUL terminator
    unsafe { *dst.add(pos) = 0 };

    dst as *const u8
}

/// Write the decimal representation of `v` into `buf` starting at `offset`.
/// Returns the number of bytes written.
fn write_u8_decimal(buf: *mut u8, offset: usize, v: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut n = 0usize;
    let mut val = v;

    if val == 0 {
        // SAFETY: offset is within the buffer (checked by caller via MAX_LEN).
        unsafe { *buf.add(offset) = b'0' };
        return 1;
    }

    while val > 0 {
        tmp[n] = b'0' + (val % 10);
        val /= 10;
        n += 1;
    }

    // Digits are in reverse; write them forward.
    for i in 0..n {
        unsafe { *buf.add(offset + i) = tmp[n - 1 - i] };
    }
    n
}

// ---------------------------------------------------------------------------
// inet_addr — convenience wrapper: "a.b.c.d" → in_addr_t (u32, network order)
// ---------------------------------------------------------------------------

/// Parse a dotted-decimal IPv4 address string into a 32-bit network-order value.
///
/// Returns the address in network byte order, or 0xffffffff (INADDR_NONE) on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn inet_addr(cp: *const u8) -> u32 {
    let mut addr: u32 = 0;
    // SAFETY: dst is a valid local u32; inet_pton4 writes exactly 4 bytes on success.
    let ret = unsafe { inet_pton(AF_INET, cp, &raw mut addr as *mut u8) };
    if ret != 1 {
        return 0xffff_ffff; // INADDR_NONE
    }
    addr
}

// ---------------------------------------------------------------------------
// inet_aton — parse dotted-decimal to in_addr
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn inet_aton(cp: *const u8, inp: *mut u32) -> i32 {
    if cp.is_null() || inp.is_null() {
        return 0;
    }
    unsafe {
        let ret = inet_pton(AF_INET, cp, inp as *mut u8);
        if ret == 1 { 1 } else { 0 }
    }
}

// ---------------------------------------------------------------------------
// inet_ntoa — format in_addr to static string
// ---------------------------------------------------------------------------

static mut NTOA_BUF: [u8; 16] = [0u8; 16];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn inet_ntoa(addr: u32) -> *const u8 {
    unsafe {
        inet_ntop(AF_INET, &raw const addr as *const u8, (&raw mut NTOA_BUF) as *mut u8, 16);
        (&raw const NTOA_BUF) as *const u8
    }
}

// ---------------------------------------------------------------------------
// FreeBSD __-prefixed aliases
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __inet_pton(af: i32, src: *const u8, dst: *mut u8) -> i32 {
    unsafe { inet_pton(af, src, dst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __inet_aton(cp: *const u8, inp: *mut u32) -> i32 {
    unsafe { inet_aton(cp, inp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __inet_ntoa(addr: u32) -> *const u8 {
    unsafe { inet_ntoa(addr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __inet_ntop(
    af: i32,
    src: *const u8,
    dst: *mut u8,
    size: u32,
) -> *const u8 {
    unsafe { inet_ntop(af, src, dst, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __inet_addr(cp: *const u8) -> u32 {
    unsafe { inet_addr(cp) }
}

// ---------------------------------------------------------------------------
// h_errno, hstrerror, gethostbyname2, gethostbyaddr
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __h_errno() -> *mut i32 {
    &raw mut h_errno
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hstrerror(err: i32) -> *const u8 {
    match err {
        1 => b"Host not found\0".as_ptr(),
        2 => b"Try again\0".as_ptr(),
        3 => b"Non-recoverable error\0".as_ptr(),
        4 => b"No address associated with hostname\0".as_ptr(),
        _ => b"Unknown resolver error\0".as_ptr(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname2(
    name: *const u8,
    af: i32,
) -> *mut crate::socket::Hostent {
    if af != AF_INET {
        return core::ptr::null_mut();
    }
    unsafe { crate::socket::gethostbyname(name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyaddr(
    addr: *const u8,
    _len: u32,
    _type_: i32,
) -> *mut crate::socket::Hostent {
    // Reverse lookup not supported — return NULL
    let _ = addr;
    core::ptr::null_mut()
}

// ---------------------------------------------------------------------------
// bcmp — BSD byte comparison (same as memcmp)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { crate::mem::memcmp(s1, s2, n) }
}

// ---------------------------------------------------------------------------
// sysctl — stub (SaltyOS does not expose kernel state via sysctl)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sysctl(
    _name: *const i32,
    _namelen: u32,
    _oldp: *mut u8,
    _oldlenp: *mut usize,
    _newp: *const u8,
    _newlen: usize,
) -> i32 {
    crate::errno::set_errno(crate::errno::ENOSYS);
    -1
}
