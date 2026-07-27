// SPDX-License-Identifier: GPL-2.0-only
//! $6$ SHA-512 crypt — glibc sha512-crypt.c compatible password hashing.
//!
//! Implements the SHA-512 variant of the Unix crypt algorithm as specified by
//! Ulrich Drepper's sha512-crypt description and glibc's implementation.
//! Only the `$6$` prefix is supported (SHA-512). The output format is:
//!
//!     $6$[rounds=N$]salt$hash
//!
//! The hash uses a crypt-specific base64 alphabet (`./0-9A-Za-z`) with a
//! permuted byte order that differs from standard base64.

use crate::sha512::Sha512;

/// Default number of rounds when no `rounds=N` is specified.
const DEFAULT_ROUNDS: u32 = 5000;
/// Minimum allowed rounds value.
const MIN_ROUNDS: u32 = 1000;
/// Maximum allowed rounds value.
const MAX_ROUNDS: u32 = 999_999_999;
/// Maximum salt length (characters after `$6$` or `rounds=N$`).
const MAX_SALT_LEN: usize = 16;
/// Maximum supported key length. glibc has no limit but we use a stack buffer.
const MAX_KEY_LEN: usize = 256;

/// Crypt-specific base64 alphabet: `./0-9A-Za-z`.
const B64: [u8; 64] = *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Static output buffer for the non-reentrant `crypt()` function.
static mut CRYPT_BUF: [u8; 256] = [0u8; 256];

/// Invalid-hash marker used when a salt format is unsupported.
///
/// Traditional libc implementations return a non-matching marker for invalid
/// salts so callers like PAM can fail authentication without dereferencing a
/// null pointer.
const INVALID_HASH: &[u8] = b"*0\0";

/// Reentrant crypt data structure. Matches the C `struct crypt_data` layout.
#[repr(C)]
pub struct CryptData {
    pub output: [u8; 256],
    pub internal: [u8; 256],
}

// ---------------------------------------------------------------------------
// C ABI exports
// ---------------------------------------------------------------------------

/// Non-reentrant crypt(3). Returns a pointer to a static buffer.
///
/// Only `$6$` (SHA-512) salts are supported. Returns a null pointer if the
/// inputs are invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypt(key: *const u8, salt: *const u8) -> *mut u8 {
    if key.is_null() || salt.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: CRYPT_BUF is a static buffer; single-threaded access assumed for
    // the non-reentrant variant, matching POSIX crypt() semantics.
    unsafe {
        let buf = &raw mut CRYPT_BUF;
        if do_sha512_crypt(key, salt, &mut *buf) {
            (*buf).as_mut_ptr()
        } else {
            write_invalid_hash(&mut *buf);
            (*buf).as_mut_ptr()
        }
    }
}

/// Reentrant crypt_r(3). Writes the result into `data.output`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypt_r(key: *const u8, salt: *const u8, data: *mut CryptData) -> *mut u8 {
    if key.is_null() || salt.is_null() || data.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: caller guarantees `data` points to a valid, exclusively-owned
    // CryptData. We write only into `data.output`.
    unsafe {
        if do_sha512_crypt(key, salt, &mut (*data).output) {
            (*data).output.as_mut_ptr()
        } else {
            write_invalid_hash(&mut (*data).output);
            (*data).output.as_mut_ptr()
        }
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Compute the length of a NUL-terminated C string.
unsafe fn strlen(s: *const u8) -> usize {
    // SAFETY: caller guarantees `s` points to a valid NUL-terminated string.
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

fn write_invalid_hash(out: &mut [u8; 256]) {
    out[..INVALID_HASH.len()].copy_from_slice(INVALID_HASH);
}

/// Read `len` bytes starting at `ptr` into a byte slice reference.
///
/// # Safety
///
/// `ptr` must be valid for reads of `len` bytes.
unsafe fn slice_from_raw(ptr: *const u8, len: usize) -> &'static [u8] {
    // SAFETY: caller guarantees validity.
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

/// Parse the salt string. Expects it to start with `$6$`.
/// Returns `(rounds, salt_bytes, salt_len)` or `None` on parse failure.
///
/// # Safety
///
/// `salt_ptr` must point to a valid NUL-terminated string.
unsafe fn parse_salt(salt_ptr: *const u8) -> Option<(u32, *const u8, usize, bool)> {
    unsafe {
        let total = strlen(salt_ptr);
        if total < 3 {
            return None;
        }
        // Must start with "$6$"
        if *salt_ptr != b'$' || *salt_ptr.add(1) != b'6' || *salt_ptr.add(2) != b'$' {
            return None;
        }

        let mut pos = 3usize;
        let mut rounds = DEFAULT_ROUNDS;
        let mut custom_rounds = false;

        // Check for optional "rounds=N$"
        if total > pos + 7 {
            // "rounds=" is 7 chars
            let r = salt_ptr.add(pos);
            if *r == b'r'
                && *r.add(1) == b'o'
                && *r.add(2) == b'u'
                && *r.add(3) == b'n'
                && *r.add(4) == b'd'
                && *r.add(5) == b's'
                && *r.add(6) == b'='
            {
                pos += 7;
                let mut n = 0u64;
                while pos < total && *salt_ptr.add(pos) != b'$' {
                    let ch = *salt_ptr.add(pos);
                    if ch < b'0' || ch > b'9' {
                        return None;
                    }
                    n = n.wrapping_mul(10).wrapping_add((ch - b'0') as u64);
                    if n > MAX_ROUNDS as u64 {
                        n = MAX_ROUNDS as u64;
                    }
                    pos += 1;
                }
                if pos >= total || *salt_ptr.add(pos) != b'$' {
                    return None;
                }
                pos += 1; // skip '$'
                rounds = n as u32;
                if rounds < MIN_ROUNDS {
                    rounds = MIN_ROUNDS;
                }
                if rounds > MAX_ROUNDS {
                    rounds = MAX_ROUNDS;
                }
                custom_rounds = true;
            }
        }

        // Extract salt (up to MAX_SALT_LEN chars, terminated by '$' or NUL)
        let salt_start = salt_ptr.add(pos);
        let mut salt_len = 0usize;
        while salt_len < MAX_SALT_LEN && pos + salt_len < total && *salt_start.add(salt_len) != b'$'
        {
            salt_len += 1;
        }

        Some((rounds, salt_start, salt_len, custom_rounds))
    }
}

/// Perform the full $6$ SHA-512 crypt computation and write the result into
/// `out`. Returns `true` on success.
///
/// # Safety
///
/// `key_ptr` and `salt_ptr` must point to valid NUL-terminated strings.
/// `out` must be a mutable reference to a buffer of at least 123 bytes
/// (maximum output: "$6$rounds=999999999$" (20) + salt (16) + "$" (1) +
/// hash (86) + NUL (1) = 124).
unsafe fn do_sha512_crypt(key_ptr: *const u8, salt_ptr: *const u8, out: &mut [u8; 256]) -> bool {
    unsafe {
        let parsed = parse_salt(salt_ptr);
        let (rounds, salt_raw, salt_len, custom_rounds) = match parsed {
            Some(v) => v,
            None => return false,
        };

        let key_len = strlen(key_ptr);
        if key_len > MAX_KEY_LEN {
            return false;
        }
        let key = slice_from_raw(key_ptr, key_len);
        let salt = slice_from_raw(salt_raw, salt_len);

        // Step 1-3: Digest B = SHA512(key + salt + key)
        let mut ctx_b = Sha512::new();
        ctx_b.update(key);
        ctx_b.update(salt);
        ctx_b.update(key);
        let digest_b = ctx_b.finalize();

        // Step 4-8: Digest A
        let mut ctx_a = Sha512::new();
        ctx_a.update(key);
        ctx_a.update(salt);

        // Add bytes from digest B, key_len bytes total
        let mut remaining = key_len;
        while remaining > 64 {
            ctx_a.update(&digest_b);
            remaining -= 64;
        }
        ctx_a.update(&digest_b[..remaining]);

        // Bit-mixing loop: for each bit of key_len, from MSB to LSB
        let mut n = key_len;
        while n > 0 {
            if n & 1 != 0 {
                ctx_a.update(&digest_b);
            } else {
                ctx_a.update(key);
            }
            n >>= 1;
        }

        let digest_a = ctx_a.finalize();

        // Step 9-11: P-string = SHA512(key repeated key_len times), first key_len bytes
        let mut ctx_p = Sha512::new();
        let mut i = 0usize;
        while i < key_len {
            ctx_p.update(key);
            i += 1;
        }
        let digest_p_full = ctx_p.finalize();
        let mut p_bytes = [0u8; 256];
        i = 0;
        while i < key_len {
            p_bytes[i] = digest_p_full[i % 64];
            i += 1;
        }

        // Step 12-14: S-string = SHA512(salt repeated (16 + A[0]) times), first salt_len bytes
        let mut ctx_s = Sha512::new();
        let s_count = 16usize + digest_a[0] as usize;
        i = 0;
        while i < s_count {
            ctx_s.update(salt);
            i += 1;
        }
        let digest_s_full = ctx_s.finalize();
        let mut s_bytes = [0u8; 16];
        i = 0;
        while i < salt_len {
            s_bytes[i] = digest_s_full[i % 64];
            i += 1;
        }

        // Step 15-21: Main loop (rounds iterations)
        let mut digest_c = digest_a;
        let mut round = 0u32;
        while round < rounds {
            let mut ctx = Sha512::new();

            if round & 1 != 0 {
                ctx.update(&p_bytes[..key_len]);
            } else {
                ctx.update(&digest_c);
            }

            if round % 3 != 0 {
                ctx.update(&s_bytes[..salt_len]);
            }

            if round % 7 != 0 {
                ctx.update(&p_bytes[..key_len]);
            }

            if round & 1 != 0 {
                ctx.update(&digest_c);
            } else {
                ctx.update(&p_bytes[..key_len]);
            }

            digest_c = ctx.finalize();
            round += 1;
        }

        // Format output: $6$[rounds=N$]salt$hash\0
        let mut pos = 0usize;

        // "$6$"
        out[pos] = b'$';
        pos += 1;
        out[pos] = b'6';
        pos += 1;
        out[pos] = b'$';
        pos += 1;

        // Optional "rounds=N$"
        if custom_rounds {
            let prefix = b"rounds=";
            i = 0;
            while i < prefix.len() {
                out[pos] = prefix[i];
                pos += 1;
                i += 1;
            }
            pos = write_u32(out, pos, rounds);
            out[pos] = b'$';
            pos += 1;
        }

        // Salt
        i = 0;
        while i < salt_len {
            out[pos] = salt[i];
            pos += 1;
            i += 1;
        }
        out[pos] = b'$';
        pos += 1;

        // Hash — SHA-512 crypt-specific byte permutation (21 triples + 1 final)
        let h = &digest_c;

        // 21 triples, each producing 4 base64 characters
        pos = b64_from_24bit(h[0], h[21], h[42], 4, out, pos);
        pos = b64_from_24bit(h[22], h[43], h[1], 4, out, pos);
        pos = b64_from_24bit(h[44], h[2], h[23], 4, out, pos);
        pos = b64_from_24bit(h[3], h[24], h[45], 4, out, pos);
        pos = b64_from_24bit(h[25], h[46], h[4], 4, out, pos);
        pos = b64_from_24bit(h[47], h[5], h[26], 4, out, pos);
        pos = b64_from_24bit(h[6], h[27], h[48], 4, out, pos);
        pos = b64_from_24bit(h[28], h[49], h[7], 4, out, pos);
        pos = b64_from_24bit(h[50], h[8], h[29], 4, out, pos);
        pos = b64_from_24bit(h[9], h[30], h[51], 4, out, pos);
        pos = b64_from_24bit(h[31], h[52], h[10], 4, out, pos);
        pos = b64_from_24bit(h[53], h[11], h[32], 4, out, pos);
        pos = b64_from_24bit(h[12], h[33], h[54], 4, out, pos);
        pos = b64_from_24bit(h[34], h[55], h[13], 4, out, pos);
        pos = b64_from_24bit(h[56], h[14], h[35], 4, out, pos);
        pos = b64_from_24bit(h[15], h[36], h[57], 4, out, pos);
        pos = b64_from_24bit(h[37], h[58], h[16], 4, out, pos);
        pos = b64_from_24bit(h[59], h[17], h[38], 4, out, pos);
        pos = b64_from_24bit(h[18], h[39], h[60], 4, out, pos);
        pos = b64_from_24bit(h[40], h[61], h[19], 4, out, pos);
        pos = b64_from_24bit(h[62], h[20], h[41], 4, out, pos);

        // Final byte (h[63]) — 2 base64 characters
        pos = b64_from_24bit(0, 0, h[63], 2, out, pos);

        // NUL terminator
        out[pos] = 0;

        true
    }
}

/// Encode up to 24 bits (from 3 bytes) into base64 characters using the
/// crypt-specific alphabet. Writes `n` characters into `out` starting at
/// `pos`. Returns the new position.
fn b64_from_24bit(b2: u8, b1: u8, b0: u8, n: usize, out: &mut [u8; 256], mut pos: usize) -> usize {
    let mut w = (b2 as u32) << 16 | (b1 as u32) << 8 | b0 as u32;
    let mut count = n;
    while count > 0 {
        out[pos] = B64[(w & 0x3f) as usize];
        pos += 1;
        w >>= 6;
        count -= 1;
    }
    pos
}

/// Write a u32 in decimal to `out[pos..]`. Returns the new position.
fn write_u32(out: &mut [u8; 256], pos: usize, val: u32) -> usize {
    if val == 0 {
        out[pos] = b'0';
        return pos + 1;
    }

    // Format digits into a temporary buffer (reversed)
    let mut tmp = [0u8; 10];
    let mut n = val;
    let mut i = 0usize;
    while n > 0 {
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    // Write in correct order
    let mut p = pos;
    while i > 0 {
        i -= 1;
        out[p] = tmp[i];
        p += 1;
    }
    p
}
