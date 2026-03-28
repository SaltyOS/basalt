//! Locale stubs (C locale only)
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Only the "C" / "POSIX" locale is supported. `setlocale` always returns
//! "C" regardless of the requested locale. `localeconv` returns a static
//! `Lconv` with POSIX defaults. GNU gettext functions (`gettext`, `dgettext`,
//! `ngettext`, `dcgettext`) pass through the message string unchanged.

pub const LC_CTYPE: i32 = 0;
pub const LC_NUMERIC: i32 = 1;
pub const LC_TIME: i32 = 2;
pub const LC_COLLATE: i32 = 3;
pub const LC_MONETARY: i32 = 4;
pub const LC_MESSAGES: i32 = 5;
pub const LC_ALL: i32 = 6;

#[repr(C)]
pub struct Lconv {
    pub decimal_point: *const u8,
    pub thousands_sep: *const u8,
    pub grouping: *const u8,
    pub int_curr_symbol: *const u8,
    pub currency_symbol: *const u8,
    pub mon_decimal_point: *const u8,
    pub mon_thousands_sep: *const u8,
    pub mon_grouping: *const u8,
    pub positive_sign: *const u8,
    pub negative_sign: *const u8,
    pub int_frac_digits: u8,
    pub frac_digits: u8,
    pub p_cs_precedes: u8,
    pub p_sep_by_space: u8,
    pub n_cs_precedes: u8,
    pub n_sep_by_space: u8,
    pub p_sign_posn: u8,
    pub n_sign_posn: u8,
    pub int_p_cs_precedes: u8,
    pub int_p_sep_by_space: u8,
    pub int_n_cs_precedes: u8,
    pub int_n_sep_by_space: u8,
    pub int_p_sign_posn: u8,
    pub int_n_sign_posn: u8,
}

static DECIMAL_POINT: [u8; 2] = *b".\0";
static EMPTY: [u8; 1] = *b"\0";
static C_LOCALE: [u8; 2] = *b"C\0";

/// Sentinel address returned as `locale_t` for the C locale.
/// We are C-locale-only, so all locale_t values are this address.
static LOCALE_SENTINEL: u8 = 0;

static mut LCONV: Lconv = Lconv {
    decimal_point: core::ptr::null(),
    thousands_sep: core::ptr::null(),
    grouping: core::ptr::null(),
    int_curr_symbol: core::ptr::null(),
    currency_symbol: core::ptr::null(),
    mon_decimal_point: core::ptr::null(),
    mon_thousands_sep: core::ptr::null(),
    mon_grouping: core::ptr::null(),
    positive_sign: core::ptr::null(),
    negative_sign: core::ptr::null(),
    int_frac_digits: 127,
    frac_digits: 127,
    p_cs_precedes: 127,
    p_sep_by_space: 127,
    n_cs_precedes: 127,
    n_sep_by_space: 127,
    p_sign_posn: 127,
    n_sign_posn: 127,
    int_p_cs_precedes: 127,
    int_p_sep_by_space: 127,
    int_n_cs_precedes: 127,
    int_n_sep_by_space: 127,
    int_p_sign_posn: 127,
    int_n_sign_posn: 127,
};

// ---------------------------------------------------------------------------
// POSIX extended locale API (newlocale / freelocale / uselocale)
// ---------------------------------------------------------------------------

/// Compare a C string pointer against a known ASCII literal.
/// # Safety
/// `s` must be a valid, NUL-terminated C string.
#[inline]
unsafe fn c_str_eq(s: *const u8, expected: &[u8]) -> bool {
    unsafe {
        for (i, &b) in expected.iter().enumerate() {
            if *s.add(i) != b {
                return false;
            }
        }
        true
    }
}

/// `newlocale` — create a locale object.
///
/// SaltyOS supports only the "C" / "POSIX" locale. Returns the static
/// sentinel for those names; returns NULL for anything else.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn newlocale(
    _mask: i32,
    locale: *const u8,
    _base: *mut u8,
) -> *mut u8 {
    unsafe {
        let sentinel = &raw const LOCALE_SENTINEL as *mut u8;
        if locale.is_null() {
            return sentinel;
        }
        if c_str_eq(locale, b"C\0")
            || c_str_eq(locale, b"POSIX\0")
            || c_str_eq(locale, b"\0")
        {
            return sentinel;
        }
        core::ptr::null_mut()
    }
}

/// `freelocale` — free a locale object.
///
/// No-op: LOCALE_SENTINEL is a static singleton and must not be freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn freelocale(_loc: *mut u8) {}

/// `uselocale` — set/query the calling thread's locale.
///
/// SaltyOS is C-locale-only; always returns LOCALE_SENTINEL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uselocale(_loc: *mut u8) -> *mut u8 {
    &raw const LOCALE_SENTINEL as *mut u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setlocale(_category: i32, _locale: *const u8) -> *const u8 {
    C_LOCALE.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn localeconv() -> *mut Lconv {
    unsafe {
        let lc = &raw mut LCONV;
        (*lc).decimal_point = DECIMAL_POINT.as_ptr();
        (*lc).thousands_sep = EMPTY.as_ptr();
        (*lc).grouping = EMPTY.as_ptr();
        (*lc).int_curr_symbol = EMPTY.as_ptr();
        (*lc).currency_symbol = EMPTY.as_ptr();
        (*lc).mon_decimal_point = EMPTY.as_ptr();
        (*lc).mon_thousands_sep = EMPTY.as_ptr();
        (*lc).mon_grouping = EMPTY.as_ptr();
        (*lc).positive_sign = EMPTY.as_ptr();
        (*lc).negative_sign = EMPTY.as_ptr();
        lc
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn textdomain(domain: *const u8) -> *const u8 {
    domain
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bindtextdomain(_domain: *const u8, dir: *const u8) -> *const u8 {
    dir
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettext(msgid: *const u8) -> *const u8 {
    msgid
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dgettext(_domain: *const u8, msgid: *const u8) -> *const u8 {
    msgid
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dcgettext(
    _domain: *const u8,
    msgid: *const u8,
    _category: i32,
) -> *const u8 {
    msgid
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngettext(
    msgid1: *const u8,
    msgid2: *const u8,
    n: u64,
) -> *const u8 {
    if n == 1 {
        msgid1
    } else {
        msgid2
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dngettext(
    _domain: *const u8,
    msgid1: *const u8,
    msgid2: *const u8,
    n: u64,
) -> *const u8 {
    if n == 1 {
        msgid1
    } else {
        msgid2
    }
}

// ---------------------------------------------------------------------------
// nl_langinfo — locale-specific information
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn nl_langinfo(item: i32) -> *const u8 {
    const ABMON: [&[u8]; 12] = [
        b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0",
        b"Jul\0", b"Aug\0", b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0",
    ];
    match item {
        14 => b"UTF-8\0".as_ptr(), // CODESET
        33..=44 => ABMON[(item - 33) as usize].as_ptr(),
        51 => b"md\0".as_ptr(),    // D_MD_ORDER
        _ => b"\0".as_ptr(),
    }
}
