//! RFC 1321 MD5 message-digest algorithm
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Implements MD5Init(), MD5Update(), MD5Final(), MD5End() with standard
//! MD5_CTX. Used by sort(1) for -R (random sort key generation).

/// MD5 digest length in bytes.
pub const MD5_DIGEST_LENGTH: usize = 16;

/// MD5 context structure — matches the C `MD5_CTX` layout.
#[repr(C)]
pub struct MD5_CTX {
    pub state: [u32; 4],
    pub count: [u32; 2],
    pub buffer: [u8; 64],
}

// MD5 constants: T[i] = floor(2^32 * abs(sin(i+1))), i = 0..63
static T: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

// Per-round shift amounts
static S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

// Padding: first byte 0x80, rest zeros
static PADDING: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Encode u32 array as little-endian bytes.
fn md5_encode(output: &mut [u8], input: &[u32], len: usize) {
    let mut i = 0;
    let mut j = 0;
    while j < len {
        output[j] = (input[i] & 0xff) as u8;
        output[j + 1] = ((input[i] >> 8) & 0xff) as u8;
        output[j + 2] = ((input[i] >> 16) & 0xff) as u8;
        output[j + 3] = ((input[i] >> 24) & 0xff) as u8;
        i += 1;
        j += 4;
    }
}

/// Decode little-endian bytes into u32 array.
fn md5_decode(output: &mut [u32], input: &[u8], len: usize) {
    let mut i = 0;
    let mut j = 0;
    while j < len {
        output[i] = (input[j] as u32)
            | ((input[j + 1] as u32) << 8)
            | ((input[j + 2] as u32) << 16)
            | ((input[j + 3] as u32) << 24);
        i += 1;
        j += 4;
    }
}

/// MD5 basic transformation — processes one 64-byte block.
fn md5_transform(state: &mut [u32; 4], block: &[u8]) {
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut m = [0u32; 16];

    md5_decode(&mut m, block, 64);

    let mut i: usize = 0;
    while i < 64 {
        let f: u32;
        let g: usize;
        if i < 16 {
            f = (b & c) | (!b & d);
            g = i;
        } else if i < 32 {
            f = (d & b) | (!d & c);
            g = (5 * i + 1) % 16;
        } else if i < 48 {
            f = b ^ c ^ d;
            g = (3 * i + 5) % 16;
        } else {
            f = c ^ (b | !d);
            g = (7 * i) % 16;
        };
        let temp = f.wrapping_add(a).wrapping_add(T[i]).wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(temp.rotate_left(S[i]));
        i += 1;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialize MD5 context with magic numbers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MD5Init(ctx: *mut MD5_CTX) {
    // SAFETY: caller guarantees ctx is a valid, writable pointer.
    unsafe {
        (*ctx).count[0] = 0;
        (*ctx).count[1] = 0;
        (*ctx).state[0] = 0x67452301;
        (*ctx).state[1] = 0xefcdab89;
        (*ctx).state[2] = 0x98badcfe;
        (*ctx).state[3] = 0x10325476;
    }
}

/// Feed input data to the MD5 context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MD5Update(ctx: *mut MD5_CTX, input: *const u8, input_len: u32) {
    unsafe {
        let data = core::slice::from_raw_parts(input, input_len as usize);

        // Compute number of bytes mod 64
        let idx = (((*ctx).count[0] >> 3) & 0x3f) as usize;

        // Update bit count
        let bit_count = input_len << 3;
        (*ctx).count[0] = (*ctx).count[0].wrapping_add(bit_count);
        if (*ctx).count[0] < bit_count {
            (*ctx).count[1] = (*ctx).count[1].wrapping_add(1);
        }
        (*ctx).count[1] = (*ctx).count[1].wrapping_add(input_len >> 29);

        let part_len = 64 - idx;
        let mut i: usize;

        // Transform as many times as possible
        if (input_len as usize) >= part_len {
            // SAFETY: idx + part_len == 64, and buffer is [u8; 64]
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (*ctx).buffer.as_mut_ptr().add(idx),
                part_len,
            );
            md5_transform(&mut (*ctx).state, &(*ctx).buffer);

            i = part_len;
            while i + 63 < input_len as usize {
                md5_transform(&mut (*ctx).state, &data[i..i + 64]);
                i += 64;
            }
        } else {
            i = 0;
        }

        // Buffer remaining input
        let remaining = (input_len as usize) - i;
        if remaining > 0 {
            // SAFETY: remaining fits within buffer[idx..64] because we
            // only reach here when input_len < part_len (i.e., idx + remaining < 64)
            // or after processing full blocks.
            let idx_new = if (input_len as usize) >= part_len {
                0
            } else {
                idx
            };
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(i),
                (*ctx).buffer.as_mut_ptr().add(idx_new),
                remaining,
            );
        }
    }
}

/// Finalize the MD5 hash: pad, append bit count, and write digest.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MD5Final(digest: *mut u8, ctx: *mut MD5_CTX) {
    unsafe {
        let mut bits = [0u8; 8];

        // Save number of bits
        md5_encode(&mut bits, &(*ctx).count, 8);

        // Pad to 56 mod 64
        let idx = (((*ctx).count[0] >> 3) & 0x3f) as usize;
        let pad_len = if idx < 56 { 56 - idx } else { 120 - idx };
        MD5Update(ctx, PADDING.as_ptr(), pad_len as u32);

        // Append length (before padding)
        MD5Update(ctx, bits.as_ptr(), 8);

        // Store state in digest
        let digest_slice = core::slice::from_raw_parts_mut(digest, MD5_DIGEST_LENGTH);
        md5_encode(digest_slice, &(*ctx).state, 16);

        // Zero sensitive information
        // SAFETY: ctx points to a valid MD5_CTX
        core::ptr::write_bytes(ctx as *mut u8, 0, core::mem::size_of::<MD5_CTX>());
    }
}

/// Finalize and convert digest to 32-character hex string.
/// Returns `buf` on success, null on null `buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MD5End(ctx: *mut MD5_CTX, buf: *mut u8) -> *mut u8 {
    if buf.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let mut digest = [0u8; MD5_DIGEST_LENGTH];
        MD5Final(digest.as_mut_ptr(), ctx);

        static HEX: [u8; 16] = *b"0123456789abcdef";
        let mut i = 0;
        while i < MD5_DIGEST_LENGTH {
            *buf.add(i * 2) = HEX[(digest[i] >> 4) as usize];
            *buf.add(i * 2 + 1) = HEX[(digest[i] & 0x0f) as usize];
            i += 1;
        }
        *buf.add(MD5_DIGEST_LENGTH * 2) = 0;
    }

    buf
}
