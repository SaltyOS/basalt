// SPDX-License-Identifier: GPL-2.0-only
//! FIPS 180-4 SHA-512 implementation.
//!
//! Pure Rust, no_std, stack-only. Used by the `crypt` module for $6$ password
//! hashing. All arithmetic uses `wrapping_add` to match the spec's mod-2^64
//! semantics.

/// SHA-512 round constants (FIPS 180-4 Section 4.2.3).
const K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4a3b47, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a1a81a664b, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

/// SHA-512 initial hash values (FIPS 180-4 Section 5.3.5).
const H0: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

/// SHA-512 hasher. All state lives on the stack.
pub struct Sha512 {
    state: [u64; 8],
    buffer: [u8; 128],
    len: u64,
    buf_len: usize,
}

impl Sha512 {
    /// Create a new SHA-512 hasher with initial state.
    pub fn new() -> Sha512 {
        Sha512 {
            state: H0,
            buffer: [0u8; 128],
            len: 0,
            buf_len: 0,
        }
    }

    /// Feed data into the hasher. Compresses full 128-byte blocks as they
    /// accumulate.
    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0usize;
        let mut remaining = data.len();

        // Fill partial buffer first
        if self.buf_len > 0 {
            let space = 128 - self.buf_len;
            let fill = if remaining < space { remaining } else { space };
            self.buffer[self.buf_len..self.buf_len + fill]
                .copy_from_slice(&data[offset..offset + fill]);
            self.buf_len += fill;
            offset += fill;
            remaining -= fill;

            if self.buf_len == 128 {
                self.compress();
                self.buf_len = 0;
            }
        }

        // Process full blocks directly
        while remaining >= 128 {
            self.buffer.copy_from_slice(&data[offset..offset + 128]);
            self.compress();
            offset += 128;
            remaining -= 128;
        }

        // Buffer remainder
        if remaining > 0 {
            self.buffer[..remaining].copy_from_slice(&data[offset..offset + remaining]);
            self.buf_len = remaining;
        }

        self.len = self.len.wrapping_add(data.len() as u64);
    }

    /// Finalize the hash, applying FIPS 180-4 padding, and return the 64-byte
    /// digest.
    pub fn finalize(mut self) -> [u8; 64] {
        let bit_len = self.len.wrapping_mul(8);

        // Append the 0x80 byte
        self.buffer[self.buf_len] = 0x80;
        self.buf_len += 1;

        // If not enough room for the 16-byte length, pad and compress
        if self.buf_len > 112 {
            // Zero the rest of this block
            let mut i = self.buf_len;
            while i < 128 {
                self.buffer[i] = 0;
                i += 1;
            }
            self.compress();
            self.buf_len = 0;
        }

        // Zero up to the length field
        let mut i = self.buf_len;
        while i < 112 {
            self.buffer[i] = 0;
            i += 1;
        }

        // Append bit length as big-endian 128-bit (upper 64 bits are zero for
        // messages < 2^64 bits)
        self.buffer[112] = 0;
        self.buffer[113] = 0;
        self.buffer[114] = 0;
        self.buffer[115] = 0;
        self.buffer[116] = 0;
        self.buffer[117] = 0;
        self.buffer[118] = 0;
        self.buffer[119] = 0;
        self.buffer[120] = (bit_len >> 56) as u8;
        self.buffer[121] = (bit_len >> 48) as u8;
        self.buffer[122] = (bit_len >> 40) as u8;
        self.buffer[123] = (bit_len >> 32) as u8;
        self.buffer[124] = (bit_len >> 24) as u8;
        self.buffer[125] = (bit_len >> 16) as u8;
        self.buffer[126] = (bit_len >> 8) as u8;
        self.buffer[127] = bit_len as u8;

        self.compress();

        // Produce big-endian output
        let mut out = [0u8; 64];
        let mut j = 0;
        while j < 8 {
            let w = self.state[j];
            let base = j * 8;
            out[base] = (w >> 56) as u8;
            out[base + 1] = (w >> 48) as u8;
            out[base + 2] = (w >> 40) as u8;
            out[base + 3] = (w >> 32) as u8;
            out[base + 4] = (w >> 24) as u8;
            out[base + 5] = (w >> 16) as u8;
            out[base + 6] = (w >> 8) as u8;
            out[base + 7] = w as u8;
            j += 1;
        }
        out
    }

    /// Compress one 128-byte block from `self.buffer` into `self.state`.
    fn compress(&mut self) {
        // Parse block into 16 big-endian u64 words
        let mut w = [0u64; 80];
        let mut i = 0;
        while i < 16 {
            let base = i * 8;
            w[i] = (self.buffer[base] as u64) << 56
                | (self.buffer[base + 1] as u64) << 48
                | (self.buffer[base + 2] as u64) << 40
                | (self.buffer[base + 3] as u64) << 32
                | (self.buffer[base + 4] as u64) << 24
                | (self.buffer[base + 5] as u64) << 16
                | (self.buffer[base + 6] as u64) << 8
                | (self.buffer[base + 7] as u64);
            i += 1;
        }

        // Extend to 80 words
        while i < 80 {
            w[i] = sigma1(w[i - 2])
                .wrapping_add(w[i - 7])
                .wrapping_add(sigma0(w[i - 15]))
                .wrapping_add(w[i - 16]);
            i += 1;
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        // 80 rounds
        i = 0;
        while i < 80 {
            let t1 = h
                .wrapping_add(big_sigma1(e))
                .wrapping_add(ch(e, f, g))
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let t2 = big_sigma0(a).wrapping_add(maj(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
            i += 1;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    /// Convenience: hash a complete message in one call.
    pub fn digest(data: &[u8]) -> [u8; 64] {
        let mut h = Sha512::new();
        h.update(data);
        h.finalize()
    }
}

// FIPS 180-4 Section 4.1.3 — SHA-512 logical functions

#[inline(always)]
fn ch(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}

#[inline(always)]
fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}

/// Big Sigma 0: ROTR^28(x) XOR ROTR^34(x) XOR ROTR^39(x)
#[inline(always)]
fn big_sigma0(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}

/// Big Sigma 1: ROTR^14(x) XOR ROTR^18(x) XOR ROTR^41(x)
#[inline(always)]
fn big_sigma1(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}

/// Small sigma 0: ROTR^1(x) XOR ROTR^8(x) XOR SHR^7(x)
#[inline(always)]
fn sigma0(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}

/// Small sigma 1: ROTR^19(x) XOR ROTR^61(x) XOR SHR^6(x)
#[inline(always)]
fn sigma1(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}
