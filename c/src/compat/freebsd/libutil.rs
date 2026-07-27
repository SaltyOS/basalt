//! BSD libutil compatibility functions
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Real implementations of expand_number(), humanize_number(),
//! dehumanize_number(), and fgetln() used by FreeBSD's head(1), tail(1),
//! and other ported utilities.

use crate::errno;

unsafe extern "C" {
    safe fn strtoll(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64;
    fn snprintf(buf: *mut u8, size: usize, fmt: *const u8, ...) -> i32;
    fn getline(lineptr: *mut *mut u8, n: *mut usize, stream: *mut crate::stdio::FILE) -> isize;
    safe fn strlen(s: *const u8) -> usize;
}

// humanize_number flags
const HN_DECIMAL: i32 = 0x01;
const HN_NOSPACE: i32 = 0x02;
const HN_B: i32 = 0x04;
const HN_DIVISOR_1000: i32 = 0x08;
const HN_GETSCALE: i32 = 0x10;
const HN_AUTOSCALE: i32 = 0x20;

// ---------------------------------------------------------------------------
// expand_number
// ---------------------------------------------------------------------------

/// Parse human-readable size strings like "10K", "5M", "2G" into byte counts
/// using 1024-based multipliers.
///
/// Returns 0 on success, -1 on error (EINVAL for bad format, ERANGE for
/// overflow).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn expand_number(buf: *const u8, num: *mut i64) -> i32 {
    unsafe {
        if buf.is_null() || *buf == 0 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let mut endptr: *mut u8 = core::ptr::null_mut();
        let val = strtoll(buf, &raw mut endptr, 0);

        if endptr == buf as *mut u8 {
            // No digits parsed
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let mut shift: i32 = 0;
        if *endptr != 0 {
            match *endptr {
                b'k' | b'K' => shift = 10,
                b'm' | b'M' => shift = 20,
                b'g' | b'G' => shift = 30,
                b't' | b'T' => shift = 40,
                b'p' | b'P' => shift = 50,
                b'e' | b'E' => shift = 60,
                _ => {
                    errno::set_errno(errno::EINVAL);
                    return -1;
                }
            }
            endptr = endptr.add(1);
            // Allow trailing 'b' or 'B' (e.g., "10KB")
            if *endptr == b'b' || *endptr == b'B' {
                endptr = endptr.add(1);
            }
            // Nothing else allowed
            if *endptr != 0 {
                errno::set_errno(errno::EINVAL);
                return -1;
            }
        }

        let mut result = val;
        if shift > 0 {
            // Check for overflow before shifting
            if val > (i64::MAX >> shift) || val < (i64::MIN >> shift) {
                errno::set_errno(errno::ERANGE);
                return -1;
            }
            result = val << shift;
        }

        *num = result;
        0
    }
}

// ---------------------------------------------------------------------------
// humanize_number
// ---------------------------------------------------------------------------

/// Format a byte count into a human-readable string.
///
/// Converts e.g. 104857600 to "100M". Supports HN_AUTOSCALE, HN_B,
/// HN_NOSPACE, HN_DECIMAL, HN_DIVISOR_1000.
///
/// Returns the length written (not including NUL), or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn humanize_number(
    buf: *mut u8,
    len: usize,
    bytes: i64,
    suffix: *const u8,
    scale: i32,
    flags: i32,
) -> i32 {
    unsafe {
        if buf.is_null() || len < 1 || suffix.is_null() {
            return -1;
        }

        let _sufflen = strlen(suffix);
        let prefixes_1024: &[u8; 8] = b" KMGTPE\0";
        let prefixes_1000: &[u8; 8] = b" kMGTPE\0";
        let prefixes = if (flags & HN_DIVISOR_1000) != 0 {
            prefixes_1000.as_ptr()
        } else {
            prefixes_1024.as_ptr()
        };
        let divisor: i64 = if (flags & HN_DIVISOR_1000) != 0 {
            1000
        } else {
            1024
        };
        let maxidx: i32 = 6; // up to 'E' (exbi/exa)

        let sign: i64 = if bytes < 0 { -1 } else { 1 };
        let mut val: i64 = if bytes < 0 { -bytes } else { bytes };

        let idx: i32;
        if (scale & HN_AUTOSCALE) != 0 {
            let mut i = 0i32;
            while val >= 10000 && i < maxidx {
                val /= divisor;
                i += 1;
            }
            // Try to get at least a single digit before decimal
            if val >= divisor && i < maxidx {
                val /= divisor;
                i += 1;
            }
            idx = i;
        } else if (scale & HN_GETSCALE) != 0 {
            let mut i = 0i32;
            let mut tmp = val;
            while tmp >= 10000 && i < maxidx {
                tmp /= divisor;
                i += 1;
            }
            if tmp >= divisor && i < maxidx {
                // tmp /= divisor; -- not needed, just incrementing idx
                i += 1;
            }
            return i;
        } else {
            // Fixed scale
            let mut s = scale;
            if s < 0 {
                s = 0;
            }
            if s > maxidx {
                s = maxidx;
            }
            let mut i = 0;
            while i < s {
                val /= divisor;
                i += 1;
            }
            idx = s;
        };

        // Format into buffer
        let r: i32;
        if idx == 0 && *prefixes == b' ' {
            // No prefix needed -- just the number + suffix
            let space = if (flags & HN_NOSPACE) != 0 {
                b"\0".as_ptr()
            } else {
                b" \0".as_ptr()
            };
            r = snprintf(
                buf,
                len,
                b"%lld%s%s\0".as_ptr(),
                (sign * val) as i64,
                space,
                suffix,
            );
        } else {
            let mut pfx = [0u8; 3];
            pfx[0] = *prefixes.add(idx as usize);
            pfx[1] = 0;
            if (flags & HN_B) != 0 && *prefixes.add(idx as usize) != b' ' {
                pfx[1] = b'B';
                pfx[2] = 0;
            }

            let space = if (flags & HN_NOSPACE) != 0 {
                b"\0".as_ptr()
            } else {
                b" \0".as_ptr()
            };

            if (flags & HN_DECIMAL) != 0 {
                // Compute one decimal digit by re-doing the division
                let mut orig: i64 = if bytes < 0 { -bytes } else { bytes };
                let mut j = 0;
                while j < idx - 1 {
                    orig /= divisor;
                    j += 1;
                }
                let mut frac: i32 = 0;
                if idx > 0 && orig > 0 {
                    frac = ((orig * 10 / divisor) % 10) as i32;
                }
                if frac > 0 {
                    r = snprintf(
                        buf,
                        len,
                        b"%lld.%d%s%s%s\0".as_ptr(),
                        (sign * val) as i64,
                        frac,
                        space,
                        pfx.as_ptr(),
                        suffix,
                    );
                } else {
                    r = snprintf(
                        buf,
                        len,
                        b"%lld%s%s%s\0".as_ptr(),
                        (sign * val) as i64,
                        space,
                        pfx.as_ptr(),
                        suffix,
                    );
                }
            } else {
                r = snprintf(
                    buf,
                    len,
                    b"%lld%s%s%s\0".as_ptr(),
                    (sign * val) as i64,
                    space,
                    pfx.as_ptr(),
                    suffix,
                );
            }
        }

        if r < 0 || (r as usize) >= len {
            return -1;
        }

        r
    }
}

// ---------------------------------------------------------------------------
// dehumanize_number
// ---------------------------------------------------------------------------

/// Parse a humanized number string back to i64. Thin wrapper around
/// expand_number.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dehumanize_number(str: *const u8, size: *mut i64) -> i32 {
    unsafe { expand_number(str, size) }
}

// ---------------------------------------------------------------------------
// fgetln
// ---------------------------------------------------------------------------

static mut FGETLN_BUF: *mut u8 = core::ptr::null_mut();
static mut FGETLN_BUFSZ: usize = 0;

/// Read one line from a FILE stream.
///
/// Returns a pointer to a static internal buffer containing the line
/// (including the newline if present), and sets *lenp to the line length.
/// The buffer is only valid until the next call to fgetln().
///
/// Returns NULL on EOF or error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fgetln(fp: *mut crate::stdio::FILE, lenp: *mut usize) -> *mut u8 {
    if fp.is_null() || lenp.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let buf_ptr = core::ptr::addr_of_mut!(FGETLN_BUF);
        let bufsz_ptr = core::ptr::addr_of_mut!(FGETLN_BUFSZ);
        let result = getline(buf_ptr, bufsz_ptr, fp);
        if result < 0 {
            *lenp = 0;
            return core::ptr::null_mut();
        }

        *lenp = result as usize;
        *buf_ptr
    }
}

/// FreeBSD `dbopen(3)` — stub that always returns NULL.
/// SaltyOS reads /etc/passwd as a flat file; the BSD DB interface is unused.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbopen(
    _file: *const u8,
    _flags: i32,
    _mode: i32,
    _type: i32,
    _openinfo: *const u8,
) -> *mut u8 {
    core::ptr::null_mut()
}
