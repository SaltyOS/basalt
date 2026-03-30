//! stdlib -- conversion, search, random numbers
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! String-to-number conversions (`atoi`, `atol`, `strtol`, `strtoul`,
//! `strtoll`, `strtoull`, `strtod`), sorting and searching (`qsort`,
//! `bsearch`), pseudo-random number generation (`rand`, `srand`, `random`,
//! `srandom`, `arc4random`), path resolution (`realpath`), and temporary
//! file creation (`mkstemp`, `mktemp`).

use crate::errno;

// ---------------------------------------------------------------------------
// String-to-number conversions
// ---------------------------------------------------------------------------

/// Skip ASCII whitespace, returning pointer to first non-whitespace byte.
unsafe fn skip_ws(mut s: *const u8) -> *const u8 {
    unsafe {
        while *s == b' ' || *s == b'\t' || *s == b'\n' || *s == b'\r' || *s == b'\x0c' || *s == b'\x0b'
        {
            s = s.add(1);
        }
        s
    }
}

/// Convert an ASCII digit character to its numeric value in the given base.
/// Returns `None` if the character is not a valid digit for that base.
fn digit_val(c: u8, base: i32) -> Option<u64> {
    let v = if c >= b'0' && c <= b'9' {
        (c - b'0') as u64
    } else if c >= b'a' && c <= b'z' {
        (c - b'a' + 10) as u64
    } else if c >= b'A' && c <= b'Z' {
        (c - b'A' + 10) as u64
    } else {
        return None;
    };
    if v < base as u64 {
        Some(v)
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atoi(s: *const u8) -> i32 {
    unsafe { strtol(s, core::ptr::null_mut(), 10) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atol(s: *const u8) -> i64 {
    unsafe { strtol(s, core::ptr::null_mut(), 10) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atoll(s: *const u8) -> i64 {
    unsafe { strtol(s, core::ptr::null_mut(), 10) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtol(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64 {
    unsafe {
        let mut p = skip_ws(s);
        let mut negative = false;

        if *p == b'+' {
            p = p.add(1);
        } else if *p == b'-' {
            negative = true;
            p = p.add(1);
        }

        let mut actual_base = base;
        if actual_base == 0 {
            if *p == b'0' {
                if *p.add(1) == b'x' || *p.add(1) == b'X' {
                    actual_base = 16;
                    p = p.add(2);
                } else {
                    actual_base = 8;
                }
            } else {
                actual_base = 10;
            }
        } else if actual_base == 16 {
            // Skip optional 0x/0X prefix for hex
            if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') {
                p = p.add(2);
            }
        }

        let mut result: u64 = 0;
        let mut overflow = false;
        let mut any_digit = false;

        let limit = if negative {
            i64::MIN as u64
        } else {
            i64::MAX as u64
        };

        while let Some(d) = digit_val(*p, actual_base) {
            any_digit = true;
            if result > limit.wrapping_sub(d) / (actual_base as u64) {
                overflow = true;
            }
            if !overflow {
                result = result * (actual_base as u64) + d;
            }
            p = p.add(1);
        }

        if !endptr.is_null() {
            if any_digit {
                *endptr = p as *mut u8;
            } else {
                *endptr = s as *mut u8;
            }
        }

        if overflow {
            errno::set_errno(errno::ERANGE);
            return if negative { i64::MIN } else { i64::MAX };
        }

        if negative {
            -(result as i64)
        } else {
            result as i64
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtoll(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64 {
    unsafe { strtol(s, endptr, base) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtoul(s: *const u8, endptr: *mut *mut u8, base: i32) -> u64 {
    unsafe {
        let mut p = skip_ws(s);
        let mut negative = false;

        if *p == b'+' {
            p = p.add(1);
        } else if *p == b'-' {
            negative = true;
            p = p.add(1);
        }

        let mut actual_base = base;
        if actual_base == 0 {
            if *p == b'0' {
                if *p.add(1) == b'x' || *p.add(1) == b'X' {
                    actual_base = 16;
                    p = p.add(2);
                } else {
                    actual_base = 8;
                }
            } else {
                actual_base = 10;
            }
        } else if actual_base == 16 {
            if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') {
                p = p.add(2);
            }
        }

        let mut result: u64 = 0;
        let mut overflow = false;
        let mut any_digit = false;

        while let Some(d) = digit_val(*p, actual_base) {
            any_digit = true;
            if result > u64::MAX.wrapping_sub(d) / (actual_base as u64) {
                overflow = true;
            }
            if !overflow {
                result = result * (actual_base as u64) + d;
            }
            p = p.add(1);
        }

        if !endptr.is_null() {
            if any_digit {
                *endptr = p as *mut u8;
            } else {
                *endptr = s as *mut u8;
            }
        }

        if overflow {
            errno::set_errno(errno::ERANGE);
            return u64::MAX;
        }

        if negative {
            result.wrapping_neg()
        } else {
            result
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtoull(s: *const u8, endptr: *mut *mut u8, base: i32) -> u64 {
    unsafe { strtoul(s, endptr, base) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtoimax(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64 {
    unsafe { strtol(s, endptr, base) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtoumax(s: *const u8, endptr: *mut *mut u8, base: i32) -> u64 {
    unsafe { strtoul(s, endptr, base) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod(s: *const u8, endptr: *mut *mut u8) -> f64 {
    unsafe {
        let mut p = skip_ws(s);
        let mut negative = false;

        if *p == b'+' {
            p = p.add(1);
        } else if *p == b'-' {
            negative = true;
            p = p.add(1);
        }

        let mut result: f64 = 0.0;
        let mut any_digit = false;

        // Integer part
        while *p >= b'0' && *p <= b'9' {
            any_digit = true;
            result = result * 10.0 + (*p - b'0') as f64;
            p = p.add(1);
        }

        // Fractional part
        if *p == b'.' {
            p = p.add(1);
            let mut frac: f64 = 0.1;
            while *p >= b'0' && *p <= b'9' {
                any_digit = true;
                result += (*p - b'0') as f64 * frac;
                frac *= 0.1;
                p = p.add(1);
            }
        }

        // Exponent part
        if *p == b'e' || *p == b'E' {
            p = p.add(1);
            let mut exp_neg = false;
            if *p == b'+' {
                p = p.add(1);
            } else if *p == b'-' {
                exp_neg = true;
                p = p.add(1);
            }

            let mut exp: i32 = 0;
            while *p >= b'0' && *p <= b'9' {
                exp = exp * 10 + (*p - b'0') as i32;
                p = p.add(1);
            }

            let mut power: f64 = 1.0;
            let mut i = 0;
            while i < exp {
                power *= 10.0;
                i += 1;
            }

            if exp_neg {
                result /= power;
            } else {
                result *= power;
            }
        }

        if !endptr.is_null() {
            if any_digit {
                *endptr = p as *mut u8;
            } else {
                *endptr = s as *mut u8;
            }
        }

        if negative {
            -result
        } else {
            result
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn atof(s: *const u8) -> f64 {
    unsafe { strtod(s, core::ptr::null_mut()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtof(s: *const u8, endptr: *mut *mut u8) -> f32 {
    unsafe { strtod(s, endptr) as f32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtold(s: *const u8, endptr: *mut *mut u8) -> f64 {
    unsafe { strtod(s, endptr) }
}

// ---------------------------------------------------------------------------
// Sorting and searching
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qsort(
    base: *mut u8,
    nmemb: usize,
    size: usize,
    compar: unsafe extern "C" fn(*const u8, *const u8) -> i32,
) {
    if nmemb <= 1 || size == 0 {
        return;
    }

    unsafe {
        // Insertion sort -- simple, no heap allocation required.
        // Use a small stack buffer for element swaps.
        const STACK_BUF: usize = 256;
        let mut buf = [0u8; STACK_BUF];
        let tmp = if size <= STACK_BUF {
            buf.as_mut_ptr()
        } else {
            // Fall back to malloc for very large elements
            crate::malloc::malloc(size)
        };

        let mut i: usize = 1;
        while i < nmemb {
            // Save element i into tmp
            let elem_i = base.add(i * size);
            core::ptr::copy_nonoverlapping(elem_i, tmp, size);

            let mut j = i;
            while j > 0 {
                let elem_j_minus_1 = base.add((j - 1) * size);
                if compar(elem_j_minus_1, tmp) <= 0 {
                    break;
                }
                // Shift element j-1 right to position j
                let elem_j = base.add(j * size);
                core::ptr::copy_nonoverlapping(elem_j_minus_1, elem_j, size);
                j -= 1;
            }

            // Insert saved element at position j
            let dest = base.add(j * size);
            core::ptr::copy_nonoverlapping(tmp, dest, size);
            i += 1;
        }

        // Free heap buffer if we used malloc
        if size > STACK_BUF && !tmp.is_null() {
            crate::malloc::free(tmp);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bsearch(
    key: *const u8,
    base: *const u8,
    nmemb: usize,
    size: usize,
    compar: unsafe extern "C" fn(*const u8, *const u8) -> i32,
) -> *mut u8 {
    if nmemb == 0 {
        return core::ptr::null_mut();
    }

    unsafe {
        let mut lo: usize = 0;
        let mut hi: usize = nmemb;

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let elem = base.add(mid * size);
            let cmp = compar(key, elem);
            if cmp < 0 {
                hi = mid;
            } else if cmp > 0 {
                lo = mid + 1;
            } else {
                return elem as *mut u8;
            }
        }

        core::ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// Math utilities
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn abs(x: i32) -> i32 {
    if x < 0 { -x } else { x }
}

#[unsafe(no_mangle)]
pub extern "C" fn labs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}

#[unsafe(no_mangle)]
pub extern "C" fn llabs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}

#[repr(C)]
pub struct DivT {
    pub quot: i32,
    pub rem: i32,
}

#[repr(C)]
pub struct LdivT {
    pub quot: i64,
    pub rem: i64,
}

#[repr(C)]
pub struct LldivT {
    pub quot: i64,
    pub rem: i64,
}

#[unsafe(no_mangle)]
pub extern "C" fn div(numer: i32, denom: i32) -> DivT {
    DivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ldiv(numer: i64, denom: i64) -> LdivT {
    LdivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn lldiv(numer: i64, denom: i64) -> LldivT {
    LldivT {
        quot: numer / denom,
        rem: numer % denom,
    }
}

// ---------------------------------------------------------------------------
// Random numbers
// ---------------------------------------------------------------------------

static mut RAND_SEED: u64 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn srand(seed: u32) {
    unsafe {
        *(&raw mut RAND_SEED) = seed as u64;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn srandom(seed: u32) {
    unsafe {
        srand(seed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rand() -> i32 {
    unsafe {
        let seed = &raw mut RAND_SEED;
        *seed = (*seed).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*seed) >> 33) as i32 & 0x7FFFFFFF
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn random() -> i64 {
    unsafe { rand() as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rand_r(seedp: *mut u32) -> i32 {
    unsafe {
        let mut s = *seedp as u64;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *seedp = s as u32;
        (s >> 33) as i32 & 0x7FFFFFFF
    }
}

// ---------------------------------------------------------------------------
// arc4random — ChaCha20-based CSPRNG
// ---------------------------------------------------------------------------

// CSPRNG state: seeded from RDRAND, generates keystream via ChaCha20.
// Reseeds every 1.6 MB of output and on fork (PID change).
static mut ARC4_KEY: [u8; 32] = [0u8; 32];
static mut ARC4_CTR: u64 = 0;
static mut ARC4_BUF: [u8; 64] = [0u8; 64];
static mut ARC4_POS: usize = 64;
static mut ARC4_RESEED_LEFT: usize = 0;
static mut ARC4_PID: u32 = 0;

const ARC4_RESEED_BYTES: usize = 1_600_000;
const CHACHA20_SIGMA: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

/// ChaCha20 quarter round on a local working state.
#[inline(always)]
fn chacha_qr(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(7);
}

/// Generate one 64-byte ChaCha20 keystream block (RFC 7539).
///
/// # Safety
/// `key` must point to 32 valid bytes. `out` must point to 64 writable bytes.
unsafe fn chacha20_block(key: *const u8, counter: u64, out: *mut u8) {
    let mut state = [0u32; 16];

    state[0] = CHACHA20_SIGMA[0];
    state[1] = CHACHA20_SIGMA[1];
    state[2] = CHACHA20_SIGMA[2];
    state[3] = CHACHA20_SIGMA[3];

    unsafe {
        let mut i = 0;
        while i < 8 {
            let off = i * 4;
            state[4 + i] = u32::from_le_bytes([
                *key.add(off),
                *key.add(off + 1),
                *key.add(off + 2),
                *key.add(off + 3),
            ]);
            i += 1;
        }
    }

    state[12] = counter as u32;
    state[13] = (counter >> 32) as u32;
    state[14] = 0;
    state[15] = 0;

    let initial = state;

    // 20 rounds (10 column + 10 diagonal)
    let mut r = 0;
    while r < 10 {
        chacha_qr(&mut state, 0, 4, 8, 12);
        chacha_qr(&mut state, 1, 5, 9, 13);
        chacha_qr(&mut state, 2, 6, 10, 14);
        chacha_qr(&mut state, 3, 7, 11, 15);
        chacha_qr(&mut state, 0, 5, 10, 15);
        chacha_qr(&mut state, 1, 6, 11, 12);
        chacha_qr(&mut state, 2, 7, 8, 13);
        chacha_qr(&mut state, 3, 4, 9, 14);
        r += 1;
    }

    unsafe {
        let mut i = 0;
        while i < 16 {
            let val = state[i].wrapping_add(initial[i]);
            let bytes = val.to_le_bytes();
            *out.add(i * 4) = bytes[0];
            *out.add(i * 4 + 1) = bytes[1];
            *out.add(i * 4 + 2) = bytes[2];
            *out.add(i * 4 + 3) = bytes[3];
            i += 1;
        }
    }
}

/// Seed (or reseed) the CSPRNG from hardware RNG (RDRAND/RNDR).
unsafe fn arc4_stir() {
    unsafe {
        let key = &raw mut ARC4_KEY as *mut u8;
        let mut filled = 0usize;
        while filled < 32 {
            match trona::syscall::sys_getrandom() {
                Some(val) => {
                    let bytes = val.to_le_bytes();
                    let remain = 32 - filled;
                    let n = if remain < 8 { remain } else { 8 };
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), key.add(filled), n);
                    filled += n;
                }
                None => {
                    // RDRAND unavailable — fall back to clock mixing
                    let mut ts = trona::types::Timespec::zeroed();
                    trona_posix::posix_clock_gettime(0, &mut ts);
                    let v = ts
                        .tv_nsec
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(ts.tv_sec);
                    let bytes = v.to_le_bytes();
                    let remain = 32 - filled;
                    let n = if remain < 8 { remain } else { 8 };
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), key.add(filled), n);
                    filled += n;
                }
            }
        }
        *(&raw mut ARC4_CTR) = 0;
        *(&raw mut ARC4_POS) = 64;
        *(&raw mut ARC4_RESEED_LEFT) = ARC4_RESEED_BYTES;
        *(&raw mut ARC4_PID) = trona_posix::posix_getpid() as u32;
    }
}

/// Ensure CSPRNG is seeded and fresh (handles first-use, fork, reseed).
unsafe fn arc4_ensure() {
    unsafe {
        let reseed_left = *(&raw const ARC4_RESEED_LEFT);
        let pid = *(&raw const ARC4_PID);
        if reseed_left == 0 || pid != trona_posix::posix_getpid() as u32 {
            arc4_stir();
        }
    }
}

/// Fill the internal keystream buffer with the next ChaCha20 block.
unsafe fn arc4_refill() {
    unsafe {
        let ctr = &raw mut ARC4_CTR;
        chacha20_block(
            &raw const ARC4_KEY as *const u8,
            *ctr,
            &raw mut ARC4_BUF as *mut u8,
        );
        *ctr = (*ctr).wrapping_add(1);
        *(&raw mut ARC4_POS) = 0;
    }
}

/// Extract `n` bytes from the ChaCha20 keystream into `buf`.
unsafe fn arc4_bytes(buf: *mut u8, n: usize) {
    unsafe {
        arc4_ensure();

        let mut written = 0usize;
        while written < n {
            let pos = *(&raw const ARC4_POS);
            if pos >= 64 {
                arc4_refill();
                continue;
            }

            let available = 64 - pos;
            let need = n - written;
            let chunk = if need < available { need } else { available };

            core::ptr::copy_nonoverlapping(
                (&raw const ARC4_BUF as *const u8).add(pos),
                buf.add(written),
                chunk,
            );

            *(&raw mut ARC4_POS) = pos + chunk;

            let reseed = &raw mut ARC4_RESEED_LEFT;
            if *reseed >= chunk {
                *reseed -= chunk;
            } else {
                *reseed = 0;
            }

            written += chunk;
        }
    }
}

/// Return a single uniformly-distributed 32-bit random value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arc4random() -> u32 {
    unsafe {
        let mut val = 0u32;
        arc4_bytes(&raw mut val as *mut u8, 4);
        val
    }
}

/// Fill `buf` with `n` cryptographically secure random bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arc4random_buf(buf: *mut u8, n: usize) {
    if buf.is_null() || n == 0 {
        return;
    }
    unsafe {
        arc4_bytes(buf, n);
    }
}

/// Return a uniformly-distributed random value in [0, upper_bound).
/// Uses rejection sampling to eliminate modulo bias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arc4random_uniform(upper_bound: u32) -> u32 {
    if upper_bound < 2 {
        return 0;
    }

    // Minimum acceptable value to avoid modulo bias
    let min = (u32::MAX - upper_bound + 1) % upper_bound;

    unsafe {
        loop {
            let r = arc4random();
            if r >= min {
                return r % upper_bound;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Other
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn system(command: *const u8) -> i32 {
    unsafe {
        // system(NULL) returns nonzero to indicate a shell is available
        if command.is_null() {
            return 1;
        }

        let pid = crate::process::fork();
        if pid < 0 {
            return -1;
        }

        if pid == 0 {
            // Child: exec /bin/sh -c <command>
            crate::process::execl(
                b"/bin/sh\0".as_ptr(),
                b"sh\0".as_ptr(),
                b"-c\0".as_ptr(),
                command,
                core::ptr::null::<u8>(),
            );
            crate::crt::_exit(127);
        }

        // Parent: wait for child
        let mut status: i32 = 0;
        crate::process::waitpid(pid, &mut status, 0);
        status
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realpath(path: *const u8, resolved: *mut u8) -> *mut u8 {
    if path.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let len = crate::string::strlen(path);

        if !resolved.is_null() {
            core::ptr::copy_nonoverlapping(path, resolved, len + 1);
            return resolved;
        }

        // resolved is null: allocate buffer
        let buf = crate::malloc::malloc(len + 1);
        if buf.is_null() {
            errno::set_errno(errno::ENOMEM);
            return core::ptr::null_mut();
        }
        core::ptr::copy_nonoverlapping(path, buf, len + 1);
        buf
    }
}

/// Monotonic counter for mktemp uniqueness.
static mut MKTEMP_COUNTER: u64 = 0;

/// Replace trailing 'X' characters in `template` with alphanumeric chars
/// derived from a counter, producing a unique filename.  Deprecated POSIX
/// function — does NOT create the file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mktemp(template: *mut u8) -> *mut u8 {
    const CHARS: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    if template.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let len = crate::string::strlen(template);
        if len == 0 {
            return template;
        }

        // Find how many trailing X's there are
        let mut xs: usize = 0;
        let mut i = len;
        while i > 0 && *template.add(i - 1) == b'X' {
            xs += 1;
            i -= 1;
        }

        if xs < 6 {
            // POSIX requires at least 6 X's; return empty string on error
            *template = 0;
            errno::set_errno(errno::EINVAL);
            return template;
        }

        let ctr = &raw mut MKTEMP_COUNTER;
        *ctr = (*ctr).wrapping_add(1);
        let mut val = *ctr;

        let start = len - xs;
        let mut j = start;
        while j < len {
            *template.add(j) = CHARS[(val % 36) as usize];
            val /= 36;
            j += 1;
        }

        template
    }
}

/// mkdtemp — create a unique temporary directory
///
/// Replaces trailing XXXXXX in `template` with unique characters and creates
/// the directory with mode 0700.  Returns `template` on success, NULL on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkdtemp(template: *mut u8) -> *mut u8 {
    const CHARS: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    const MAX_ATTEMPTS: u32 = 256;

    if template.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let len = crate::string::strlen(template);
        if len < 6 {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        // Count trailing X's
        let mut xs: usize = 0;
        let mut i = len;
        while i > 0 && *template.add(i - 1) == b'X' {
            xs += 1;
            i -= 1;
        }

        if xs < 6 {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        let start = len - xs;

        use core::sync::atomic::{AtomicU64, Ordering};
        static MKDTEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

        let mut attempt: u32 = 0;
        while attempt < MAX_ATTEMPTS {
            let mut val = MKDTEMP_COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);

            let mut j = start;
            while j < len {
                *template.add(j) = CHARS[(val % 36) as usize];
                val /= 36;
                j += 1;
            }

            let ret = trona_posix::posix_mkdir(template, 0o700);
            if ret == 0 {
                return template;
            }
            // posix_mkdir returns -1 on any error; retry with next name
            attempt += 1;
        }

        errno::set_errno(errno::EEXIST);
        *template.add(start) = 0;
        core::ptr::null_mut()
    }
}

