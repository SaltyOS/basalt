//! iconv — character set conversion
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! POSIX iconv implementation with a 2-stage pipeline: source bytes are decoded
//! into Unicode codepoints (u32), then encoded into the destination encoding.
//!
//! Supported encodings: UTF-8, ASCII, ISO-8859-1, ISO-8859-15, UTF-16LE,
//! UTF-16BE, UTF-32LE, UTF-32BE. Encoding names are matched case-insensitively
//! with common aliases. `//IGNORE` and `//TRANSLIT` suffixes are stripped.

use crate::errno::{E2BIG, EILSEQ, EINVAL, ENOMEM, set_errno};
use crate::malloc::{free, malloc};

// ---------------------------------------------------------------------------
// Encoding enumeration
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Encoding {
    Utf8 = 0,
    Ascii = 1,
    Iso8859_1 = 2,
    Iso8859_15 = 3,
    Utf16Le = 4,
    Utf16Be = 5,
    Utf32Le = 6,
    Utf32Be = 7,
}

// ---------------------------------------------------------------------------
// Conversion descriptor
// ---------------------------------------------------------------------------

#[repr(C)]
struct IconvDescriptor {
    from: Encoding,
    to: Encoding,
    /// Partial codepoint accumulated across calls (UTF-8 multibyte state).
    state_cp: u32,
    /// Number of continuation bytes accumulated so far.
    state_bytes: u8,
    /// Total continuation bytes expected for the in-progress sequence.
    state_expected: u8,
    _pad: [u8; 2],
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

enum DecodeError {
    /// Not enough input bytes to complete a character.
    Incomplete,
    /// Invalid byte sequence.
    Invalid,
}

enum EncodeError {
    /// Output buffer too small.
    OutputFull,
    /// Codepoint cannot be represented in the target encoding.
    Unmappable,
}

// ---------------------------------------------------------------------------
// ISO-8859-15 difference table
// ---------------------------------------------------------------------------
//
// ISO-8859-15 is identical to ISO-8859-1 except at these 8 positions.

static ISO8859_15_DIFF: [(u8, u32); 8] = [
    (0xA4, 0x20AC), // EURO SIGN
    (0xA6, 0x0160), // LATIN CAPITAL LETTER S WITH CARON
    (0xA8, 0x0161), // LATIN SMALL LETTER S WITH CARON
    (0xB4, 0x017D), // LATIN CAPITAL LETTER Z WITH CARON
    (0xB8, 0x017E), // LATIN SMALL LETTER Z WITH CARON
    (0xBC, 0x0152), // LATIN CAPITAL LIGATURE OE
    (0xBD, 0x0153), // LATIN SMALL LIGATURE OE
    (0xBE, 0x0178), // LATIN CAPITAL LETTER Y WITH DIAERESIS
];

// ---------------------------------------------------------------------------
// Encoding alias table
// ---------------------------------------------------------------------------
//
// Each entry is (canonical_name_bytes, encoding). Names are NUL-terminated
// and compared case-insensitively.

struct Alias {
    name: &'static [u8],
    enc: Encoding,
}

static ALIASES: [Alias; 24] = [
    // UTF-8
    Alias {
        name: b"UTF-8\0",
        enc: Encoding::Utf8,
    },
    Alias {
        name: b"UTF8\0",
        enc: Encoding::Utf8,
    },
    // ASCII
    Alias {
        name: b"ASCII\0",
        enc: Encoding::Ascii,
    },
    Alias {
        name: b"US-ASCII\0",
        enc: Encoding::Ascii,
    },
    Alias {
        name: b"ANSI_X3.4-1968\0",
        enc: Encoding::Ascii,
    },
    Alias {
        name: b"646\0",
        enc: Encoding::Ascii,
    },
    Alias {
        name: b"ISO646-US\0",
        enc: Encoding::Ascii,
    },
    // ISO-8859-1
    Alias {
        name: b"ISO-8859-1\0",
        enc: Encoding::Iso8859_1,
    },
    Alias {
        name: b"ISO8859-1\0",
        enc: Encoding::Iso8859_1,
    },
    Alias {
        name: b"ISO_8859-1\0",
        enc: Encoding::Iso8859_1,
    },
    Alias {
        name: b"LATIN1\0",
        enc: Encoding::Iso8859_1,
    },
    Alias {
        name: b"ISO-IR-100\0",
        enc: Encoding::Iso8859_1,
    },
    Alias {
        name: b"CSISOLATIN1\0",
        enc: Encoding::Iso8859_1,
    },
    // ISO-8859-15
    Alias {
        name: b"ISO-8859-15\0",
        enc: Encoding::Iso8859_15,
    },
    Alias {
        name: b"ISO8859-15\0",
        enc: Encoding::Iso8859_15,
    },
    Alias {
        name: b"ISO_8859-15\0",
        enc: Encoding::Iso8859_15,
    },
    Alias {
        name: b"LATIN9\0",
        enc: Encoding::Iso8859_15,
    },
    // UTF-16
    Alias {
        name: b"UTF-16LE\0",
        enc: Encoding::Utf16Le,
    },
    Alias {
        name: b"UTF16LE\0",
        enc: Encoding::Utf16Le,
    },
    Alias {
        name: b"UCS-2LE\0",
        enc: Encoding::Utf16Le,
    },
    Alias {
        name: b"UTF-16BE\0",
        enc: Encoding::Utf16Be,
    },
    Alias {
        name: b"UTF16BE\0",
        enc: Encoding::Utf16Be,
    },
    Alias {
        name: b"UCS-2BE\0",
        enc: Encoding::Utf16Be,
    },
    // UTF-32 (LE only listed first for alignment; both present)
    Alias {
        name: b"UTF-32LE\0",
        enc: Encoding::Utf32Le,
    },
];

// Additional UTF-32 aliases stored separately to keep ALIASES at a fixed size
// that the compiler can handle without complex const generics.
static ALIASES_EXT: [Alias; 5] = [
    Alias {
        name: b"UTF32LE\0",
        enc: Encoding::Utf32Le,
    },
    Alias {
        name: b"UCS-4LE\0",
        enc: Encoding::Utf32Le,
    },
    Alias {
        name: b"UTF-32BE\0",
        enc: Encoding::Utf32Be,
    },
    Alias {
        name: b"UTF32BE\0",
        enc: Encoding::Utf32Be,
    },
    Alias {
        name: b"UCS-4BE\0",
        enc: Encoding::Utf32Be,
    },
];

// ---------------------------------------------------------------------------
// Helper: case-insensitive C string comparison
// ---------------------------------------------------------------------------

/// Compare two NUL-terminated byte strings case-insensitively (ASCII only).
///
/// # Safety
/// Both `a` and `b` must be valid, NUL-terminated C strings.
unsafe fn ascii_casecmp(a: *const u8, b: *const u8) -> bool {
    unsafe {
        let mut i = 0usize;
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca == 0 && cb == 0 {
                return true;
            }
            if ca == 0 || cb == 0 {
                return false;
            }
            let la = if ca >= b'A' && ca <= b'Z' {
                ca + 32
            } else {
                ca
            };
            let lb = if cb >= b'A' && cb <= b'Z' {
                cb + 32
            } else {
                cb
            };
            if la != lb {
                return false;
            }
            i += 1;
        }
    }
}

/// Compute the length of a NUL-terminated C string.
///
/// # Safety
/// `s` must be a valid, NUL-terminated C string.
unsafe fn c_strlen(s: *const u8) -> usize {
    unsafe {
        let mut len = 0usize;
        while *s.add(len) != 0 {
            len += 1;
        }
        len
    }
}

// ---------------------------------------------------------------------------
// Encoding name resolution
// ---------------------------------------------------------------------------

/// Resolve a NUL-terminated encoding name to an `Encoding` variant.
///
/// Strips `//IGNORE` and `//TRANSLIT` suffixes before matching. Comparison
/// is case-insensitive.
///
/// # Safety
/// `name` must be a valid, NUL-terminated C string.
unsafe fn resolve_encoding(name: *const u8) -> Option<Encoding> {
    if name.is_null() {
        return None;
    }

    // Copy the name to a stack buffer so we can strip suffixes.
    let len = unsafe { c_strlen(name) };
    if len == 0 || len > 63 {
        return None;
    }
    let mut buf = [0u8; 64];
    let mut copy_len = len;
    unsafe {
        let mut i = 0;
        while i < len && i < 63 {
            buf[i] = *name.add(i);
            i += 1;
        }
        buf[i] = 0;
    }

    // Strip //IGNORE and //TRANSLIT suffixes (they can appear together).
    loop {
        let stripped = strip_suffix(&buf, copy_len, b"//IGNORE");
        if let Some(new_len) = stripped {
            copy_len = new_len;
            buf[copy_len] = 0;
            continue;
        }
        let stripped = strip_suffix(&buf, copy_len, b"//TRANSLIT");
        if let Some(new_len) = stripped {
            copy_len = new_len;
            buf[copy_len] = 0;
            continue;
        }
        break;
    }

    if copy_len == 0 {
        return None;
    }

    let buf_ptr = buf.as_ptr();

    // Search the main alias table.
    let mut i = 0;
    while i < ALIASES.len() {
        if unsafe { ascii_casecmp(buf_ptr, ALIASES[i].name.as_ptr()) } {
            return Some(ALIASES[i].enc);
        }
        i += 1;
    }

    // Search the extension alias table.
    i = 0;
    while i < ALIASES_EXT.len() {
        if unsafe { ascii_casecmp(buf_ptr, ALIASES_EXT[i].name.as_ptr()) } {
            return Some(ALIASES_EXT[i].enc);
        }
        i += 1;
    }

    None
}

/// Strip a case-insensitive suffix from a byte buffer.
/// Returns the new length if the suffix was found and removed, or `None`.
fn strip_suffix(buf: &[u8; 64], len: usize, suffix: &[u8]) -> Option<usize> {
    let slen = suffix.len();
    if len < slen {
        return None;
    }
    let start = len - slen;
    let mut i = 0;
    while i < slen {
        let a = buf[start + i];
        let b = suffix[i];
        let la = if a >= b'A' && a <= b'Z' { a + 32 } else { a };
        let lb = if b >= b'A' && b <= b'Z' { b + 32 } else { b };
        if la != lb {
            return None;
        }
        i += 1;
    }
    Some(start)
}

// ---------------------------------------------------------------------------
// Decoders: source bytes -> u32 codepoint
// ---------------------------------------------------------------------------

/// Decode one UTF-8 character.
///
/// Returns `Ok((codepoint, bytes_consumed))` or a `DecodeError`.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_utf8(src: *const u8, len: usize) -> Result<(u32, usize), DecodeError> {
    if len == 0 {
        return Err(DecodeError::Incomplete);
    }

    let b0 = unsafe { *src };

    // Single-byte ASCII (0x00..0x7F)
    if b0 < 0x80 {
        return Ok((b0 as u32, 1));
    }

    // Determine sequence length from the leading byte.
    let (expected, mut cp) = if b0 & 0xE0 == 0xC0 {
        (2usize, (b0 & 0x1F) as u32)
    } else if b0 & 0xF0 == 0xE0 {
        (3usize, (b0 & 0x0F) as u32)
    } else if b0 & 0xF8 == 0xF0 {
        (4usize, (b0 & 0x07) as u32)
    } else {
        // Invalid leading byte (0x80..0xBF or 0xF8+)
        return Err(DecodeError::Invalid);
    };

    if len < expected {
        return Err(DecodeError::Incomplete);
    }

    // Read continuation bytes.
    let mut i = 1usize;
    while i < expected {
        let b = unsafe { *src.add(i) };
        if b & 0xC0 != 0x80 {
            return Err(DecodeError::Invalid);
        }
        cp = (cp << 6) | (b & 0x3F) as u32;
        i += 1;
    }

    // Reject overlong encodings.
    match expected {
        2 => {
            if cp < 0x80 {
                return Err(DecodeError::Invalid);
            }
        }
        3 => {
            if cp < 0x800 {
                return Err(DecodeError::Invalid);
            }
        }
        4 => {
            if cp < 0x10000 {
                return Err(DecodeError::Invalid);
            }
        }
        _ => {}
    }

    // Reject surrogates (U+D800..U+DFFF).
    if cp >= 0xD800 && cp <= 0xDFFF {
        return Err(DecodeError::Invalid);
    }

    // Reject codepoints above U+10FFFF.
    if cp > 0x10FFFF {
        return Err(DecodeError::Invalid);
    }

    Ok((cp, expected))
}

/// Decode one ASCII byte.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_ascii(src: *const u8, len: usize) -> Result<(u32, usize), DecodeError> {
    if len == 0 {
        return Err(DecodeError::Incomplete);
    }
    let b = unsafe { *src };
    if b >= 0x80 {
        return Err(DecodeError::Invalid);
    }
    Ok((b as u32, 1))
}

/// Decode one ISO-8859-1 byte (identity mapping to U+0000..U+00FF).
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_iso8859_1(src: *const u8, len: usize) -> Result<(u32, usize), DecodeError> {
    if len == 0 {
        return Err(DecodeError::Incomplete);
    }
    let b = unsafe { *src };
    Ok((b as u32, 1))
}

/// Decode one ISO-8859-15 byte.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_iso8859_15(src: *const u8, len: usize) -> Result<(u32, usize), DecodeError> {
    if len == 0 {
        return Err(DecodeError::Incomplete);
    }
    let b = unsafe { *src };

    // Check the difference table for the 8 positions that diverge from Latin-1.
    let mut i = 0;
    while i < ISO8859_15_DIFF.len() {
        if b == ISO8859_15_DIFF[i].0 {
            return Ok((ISO8859_15_DIFF[i].1, 1));
        }
        i += 1;
    }

    // All other positions are the same as ISO-8859-1.
    Ok((b as u32, 1))
}

/// Decode one UTF-16 code unit (or surrogate pair) with the given endianness.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_utf16(
    src: *const u8,
    len: usize,
    big_endian: bool,
) -> Result<(u32, usize), DecodeError> {
    if len < 2 {
        return Err(DecodeError::Incomplete);
    }

    let w0 = unsafe { read_u16(src, big_endian) };

    // BMP character (not a surrogate).
    if w0 < 0xD800 || w0 > 0xDFFF {
        return Ok((w0 as u32, 2));
    }

    // High surrogate (0xD800..0xDBFF) — expect a low surrogate following.
    if w0 >= 0xD800 && w0 <= 0xDBFF {
        if len < 4 {
            return Err(DecodeError::Incomplete);
        }
        let w1 = unsafe { read_u16(src.add(2), big_endian) };
        if w1 < 0xDC00 || w1 > 0xDFFF {
            return Err(DecodeError::Invalid);
        }
        let cp = 0x10000 + ((w0 as u32 - 0xD800) << 10) + (w1 as u32 - 0xDC00);
        return Ok((cp, 4));
    }

    // Lone low surrogate — invalid.
    Err(DecodeError::Invalid)
}

/// Decode one UTF-32 codepoint with the given endianness.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_utf32(
    src: *const u8,
    len: usize,
    big_endian: bool,
) -> Result<(u32, usize), DecodeError> {
    if len < 4 {
        return Err(DecodeError::Incomplete);
    }
    let cp = unsafe { read_u32(src, big_endian) };

    // Reject surrogates and out-of-range values.
    if cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF) {
        return Err(DecodeError::Invalid);
    }
    Ok((cp, 4))
}

// ---------------------------------------------------------------------------
// Encoders: u32 codepoint -> destination bytes
// ---------------------------------------------------------------------------

/// Encode a codepoint as UTF-8.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_utf8(cp: u32, dst: *mut u8, dst_len: usize) -> Result<usize, EncodeError> {
    if cp < 0x80 {
        if dst_len < 1 {
            return Err(EncodeError::OutputFull);
        }
        unsafe { *dst = cp as u8 };
        Ok(1)
    } else if cp < 0x800 {
        if dst_len < 2 {
            return Err(EncodeError::OutputFull);
        }
        unsafe {
            *dst = 0xC0 | (cp >> 6) as u8;
            *dst.add(1) = 0x80 | (cp & 0x3F) as u8;
        }
        Ok(2)
    } else if cp < 0x10000 {
        if dst_len < 3 {
            return Err(EncodeError::OutputFull);
        }
        unsafe {
            *dst = 0xE0 | (cp >> 12) as u8;
            *dst.add(1) = 0x80 | ((cp >> 6) & 0x3F) as u8;
            *dst.add(2) = 0x80 | (cp & 0x3F) as u8;
        }
        Ok(3)
    } else {
        if dst_len < 4 {
            return Err(EncodeError::OutputFull);
        }
        unsafe {
            *dst = 0xF0 | (cp >> 18) as u8;
            *dst.add(1) = 0x80 | ((cp >> 12) & 0x3F) as u8;
            *dst.add(2) = 0x80 | ((cp >> 6) & 0x3F) as u8;
            *dst.add(3) = 0x80 | (cp & 0x3F) as u8;
        }
        Ok(4)
    }
}

/// Encode a codepoint as ASCII.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_ascii(cp: u32, dst: *mut u8, dst_len: usize) -> Result<usize, EncodeError> {
    if cp > 0x7F {
        return Err(EncodeError::Unmappable);
    }
    if dst_len < 1 {
        return Err(EncodeError::OutputFull);
    }
    unsafe { *dst = cp as u8 };
    Ok(1)
}

/// Encode a codepoint as ISO-8859-1.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_iso8859_1(cp: u32, dst: *mut u8, dst_len: usize) -> Result<usize, EncodeError> {
    if cp > 0xFF {
        return Err(EncodeError::Unmappable);
    }
    if dst_len < 1 {
        return Err(EncodeError::OutputFull);
    }
    unsafe { *dst = cp as u8 };
    Ok(1)
}

/// Encode a codepoint as ISO-8859-15.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_iso8859_15(cp: u32, dst: *mut u8, dst_len: usize) -> Result<usize, EncodeError> {
    if dst_len < 1 {
        return Err(EncodeError::OutputFull);
    }

    // Check the reverse mapping for the 8 special codepoints.
    let mut i = 0;
    while i < ISO8859_15_DIFF.len() {
        if cp == ISO8859_15_DIFF[i].1 {
            unsafe { *dst = ISO8859_15_DIFF[i].0 };
            return Ok(1);
        }
        i += 1;
    }

    // For the 8 positions that were remapped, the original Latin-1 codepoints
    // at those byte values are no longer representable in ISO-8859-15.
    // Specifically: U+00A4, U+00A6, U+00A8, U+00B4, U+00B8, U+00BC, U+00BD, U+00BE
    // map to different characters in ISO-8859-15. But since the byte values
    // are the same as the codepoint values in Latin-1, and we already checked
    // the special codepoints above, we can safely encode codepoints <= 0xFF
    // that are NOT one of the remapped originals. Actually, the Latin-1
    // codepoints at the remapped positions (e.g. U+00A4 = CURRENCY SIGN) are
    // NOT encodable in ISO-8859-15 since those byte positions are used for
    // different characters. We must reject them.
    if cp <= 0xFF {
        let byte = cp as u8;
        let mut j = 0;
        while j < ISO8859_15_DIFF.len() {
            if byte == ISO8859_15_DIFF[j].0 {
                // This byte position is occupied by a different character in
                // ISO-8859-15. The original Latin-1 character is unmappable.
                return Err(EncodeError::Unmappable);
            }
            j += 1;
        }
        unsafe { *dst = byte };
        return Ok(1);
    }

    Err(EncodeError::Unmappable)
}

/// Encode a codepoint as UTF-16 with the given endianness.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_utf16(
    cp: u32,
    dst: *mut u8,
    dst_len: usize,
    big_endian: bool,
) -> Result<usize, EncodeError> {
    if cp <= 0xFFFF {
        // BMP: single 16-bit code unit.
        if dst_len < 2 {
            return Err(EncodeError::OutputFull);
        }
        unsafe { write_u16(dst, cp as u16, big_endian) };
        Ok(2)
    } else if cp <= 0x10FFFF {
        // Supplementary: surrogate pair.
        if dst_len < 4 {
            return Err(EncodeError::OutputFull);
        }
        let adj = cp - 0x10000;
        let high = 0xD800 + (adj >> 10) as u16;
        let low = 0xDC00 + (adj & 0x3FF) as u16;
        unsafe {
            write_u16(dst, high, big_endian);
            write_u16(dst.add(2), low, big_endian);
        }
        Ok(4)
    } else {
        Err(EncodeError::Unmappable)
    }
}

/// Encode a codepoint as UTF-32 with the given endianness.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_utf32(
    cp: u32,
    dst: *mut u8,
    dst_len: usize,
    big_endian: bool,
) -> Result<usize, EncodeError> {
    if dst_len < 4 {
        return Err(EncodeError::OutputFull);
    }
    unsafe { write_u32(dst, cp, big_endian) };
    Ok(4)
}

// ---------------------------------------------------------------------------
// Endian helpers
// ---------------------------------------------------------------------------

/// Read a u16 from a byte pointer with the specified endianness.
///
/// # Safety
/// `p` must point to at least 2 readable bytes.
#[inline]
unsafe fn read_u16(p: *const u8, big_endian: bool) -> u16 {
    unsafe {
        if big_endian {
            (*p as u16) << 8 | *p.add(1) as u16
        } else {
            *p as u16 | (*p.add(1) as u16) << 8
        }
    }
}

/// Read a u32 from a byte pointer with the specified endianness.
///
/// # Safety
/// `p` must point to at least 4 readable bytes.
#[inline]
unsafe fn read_u32(p: *const u8, big_endian: bool) -> u32 {
    unsafe {
        if big_endian {
            (*p as u32) << 24
                | (*p.add(1) as u32) << 16
                | (*p.add(2) as u32) << 8
                | *p.add(3) as u32
        } else {
            *p as u32
                | (*p.add(1) as u32) << 8
                | (*p.add(2) as u32) << 16
                | (*p.add(3) as u32) << 24
        }
    }
}

/// Write a u16 to a byte pointer with the specified endianness.
///
/// # Safety
/// `p` must point to at least 2 writable bytes.
#[inline]
unsafe fn write_u16(p: *mut u8, v: u16, big_endian: bool) {
    unsafe {
        if big_endian {
            *p = (v >> 8) as u8;
            *p.add(1) = v as u8;
        } else {
            *p = v as u8;
            *p.add(1) = (v >> 8) as u8;
        }
    }
}

/// Write a u32 to a byte pointer with the specified endianness.
///
/// # Safety
/// `p` must point to at least 4 writable bytes.
#[inline]
unsafe fn write_u32(p: *mut u8, v: u32, big_endian: bool) {
    unsafe {
        if big_endian {
            *p = (v >> 24) as u8;
            *p.add(1) = (v >> 16) as u8;
            *p.add(2) = (v >> 8) as u8;
            *p.add(3) = v as u8;
        } else {
            *p = v as u8;
            *p.add(1) = (v >> 8) as u8;
            *p.add(2) = (v >> 16) as u8;
            *p.add(3) = (v >> 24) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

/// Decode one codepoint from `src` using the given encoding.
///
/// # Safety
/// `src` must point to at least `len` readable bytes.
unsafe fn decode_one(
    enc: Encoding,
    src: *const u8,
    len: usize,
) -> Result<(u32, usize), DecodeError> {
    match enc {
        Encoding::Utf8 => unsafe { decode_utf8(src, len) },
        Encoding::Ascii => unsafe { decode_ascii(src, len) },
        Encoding::Iso8859_1 => unsafe { decode_iso8859_1(src, len) },
        Encoding::Iso8859_15 => unsafe { decode_iso8859_15(src, len) },
        Encoding::Utf16Le => unsafe { decode_utf16(src, len, false) },
        Encoding::Utf16Be => unsafe { decode_utf16(src, len, true) },
        Encoding::Utf32Le => unsafe { decode_utf32(src, len, false) },
        Encoding::Utf32Be => unsafe { decode_utf32(src, len, true) },
    }
}

/// Encode one codepoint to `dst` using the given encoding.
///
/// # Safety
/// `dst` must point to at least `dst_len` writable bytes.
unsafe fn encode_one(
    enc: Encoding,
    cp: u32,
    dst: *mut u8,
    dst_len: usize,
) -> Result<usize, EncodeError> {
    match enc {
        Encoding::Utf8 => unsafe { encode_utf8(cp, dst, dst_len) },
        Encoding::Ascii => unsafe { encode_ascii(cp, dst, dst_len) },
        Encoding::Iso8859_1 => unsafe { encode_iso8859_1(cp, dst, dst_len) },
        Encoding::Iso8859_15 => unsafe { encode_iso8859_15(cp, dst, dst_len) },
        Encoding::Utf16Le => unsafe { encode_utf16(cp, dst, dst_len, false) },
        Encoding::Utf16Be => unsafe { encode_utf16(cp, dst, dst_len, true) },
        Encoding::Utf32Le => unsafe { encode_utf32(cp, dst, dst_len, false) },
        Encoding::Utf32Be => unsafe { encode_utf32(cp, dst, dst_len, true) },
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

const ICONV_ERROR: *mut u8 = usize::MAX as *mut u8;
const SIZE_MAX: usize = usize::MAX;

/// `iconv_open` — allocate a conversion descriptor.
///
/// Returns an opaque descriptor on success, or `(iconv_t)-1` with `errno`
/// set to `EINVAL` if either encoding name is unrecognized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iconv_open(tocode: *const u8, fromcode: *const u8) -> *mut u8 {
    let from = match unsafe { resolve_encoding(fromcode) } {
        Some(e) => e,
        None => {
            set_errno(EINVAL);
            return ICONV_ERROR;
        }
    };
    let to = match unsafe { resolve_encoding(tocode) } {
        Some(e) => e,
        None => {
            set_errno(EINVAL);
            return ICONV_ERROR;
        }
    };

    let size = core::mem::size_of::<IconvDescriptor>();
    let ptr = unsafe { malloc(size) };
    if ptr.is_null() {
        set_errno(ENOMEM);
        return ICONV_ERROR;
    }

    let desc = ptr as *mut IconvDescriptor;
    unsafe {
        (*desc).from = from;
        (*desc).to = to;
        (*desc).state_cp = 0;
        (*desc).state_bytes = 0;
        (*desc).state_expected = 0;
        (*desc)._pad = [0; 2];
    }

    ptr
}

/// `iconv` — perform character set conversion.
///
/// Converts bytes from `*inbuf` (at most `*inbytesleft` bytes) into `*outbuf`
/// (at most `*outbytesleft` bytes), advancing both pointers and decrementing
/// both counts. Returns the number of non-reversible conversions performed
/// (always 0 in strict mode), or `(size_t)-1` on error with `errno` set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iconv(
    cd: *mut u8,
    inbuf: *mut *mut u8,
    inbytesleft: *mut usize,
    outbuf: *mut *mut u8,
    outbytesleft: *mut usize,
) -> usize {
    if cd.is_null() || cd == ICONV_ERROR {
        set_errno(EINVAL);
        return SIZE_MAX;
    }

    let desc = cd as *mut IconvDescriptor;

    // NULL inbuf: reset shift state to initial.
    if inbuf.is_null() || unsafe { (*inbuf).is_null() } {
        unsafe {
            (*desc).state_cp = 0;
            (*desc).state_bytes = 0;
            (*desc).state_expected = 0;
        }
        return 0;
    }

    let from_enc = unsafe { (*desc).from };
    let to_enc = unsafe { (*desc).to };
    let non_reversible: usize = 0;

    loop {
        let in_left = unsafe { *inbytesleft };
        if in_left == 0 {
            break;
        }

        let in_ptr = unsafe { *inbuf };
        let out_left = unsafe { *outbytesleft };
        let out_ptr = unsafe { *outbuf };

        // Stage 1: decode one codepoint from source.
        let (cp, consumed) = match unsafe { decode_one(from_enc, in_ptr as *const u8, in_left) } {
            Ok(pair) => pair,
            Err(DecodeError::Incomplete) => {
                set_errno(EINVAL);
                return SIZE_MAX;
            }
            Err(DecodeError::Invalid) => {
                set_errno(EILSEQ);
                return SIZE_MAX;
            }
        };

        // Stage 2: encode the codepoint into the destination.
        let written = match unsafe { encode_one(to_enc, cp, out_ptr, out_left) } {
            Ok(n) => n,
            Err(EncodeError::OutputFull) => {
                // Do not advance the input pointer — the caller must provide
                // more output space and retry.
                set_errno(E2BIG);
                return SIZE_MAX;
            }
            Err(EncodeError::Unmappable) => {
                // The codepoint cannot be represented in the target encoding.
                set_errno(EILSEQ);
                return SIZE_MAX;
            }
        };

        // Advance both buffers.
        unsafe {
            *inbuf = in_ptr.add(consumed);
            *inbytesleft = in_left - consumed;
            *outbuf = out_ptr.add(written);
            *outbytesleft = out_left - written;
        }

        // Identity-transcoding and lossless conversions count as reversible.
        // Non-reversible conversions would happen with //TRANSLIT substitution
        // (not implemented), so this stays 0 in strict mode.
    }

    non_reversible
}

/// `iconv_close` — deallocate a conversion descriptor.
///
/// Returns 0 on success, or -1 with `errno` set to `EINVAL` if `cd` is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn iconv_close(cd: *mut u8) -> i32 {
    if cd.is_null() || cd == ICONV_ERROR {
        set_errno(EINVAL);
        return -1;
    }
    unsafe { free(cd) };
    0
}
