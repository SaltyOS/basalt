//! Time functions
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! POSIX time implementation with timezone support via TZ environment variable.
//! Time values use Unix epoch (seconds since 1970-01-01 00:00:00 UTC). The `Tm`
//! struct follows POSIX conventions: `tm_mon` is 0-based (0 = January),
//! `tm_year` is years since 1900, and `tm_wday` is 0 = Sunday.
//!
//! Timezone support: `tzset()` parses the POSIX TZ variable
//! (e.g., `EST5EDT,M3.2.0,M11.1.0`) to set the `timezone`, `daylight`, and
//! `tzname` globals. `localtime()` applies the timezone offset, and `mktime()`
//! interprets input as local time. DST transition rules (Mm.w.d format) are
//! fully supported.
//!
//! Non-reentrant functions (`localtime`, `gmtime`, `ctime`, `asctime`) use
//! per-thread TLS buffers (with static fallback pre-TLS). The `_r` variants
//! accept caller-provided buffers.

use crate::errno;

/// Mutex protecting timezone globals (tzname, timezone, daylight, TZ_STD_NAME,
/// TZ_DST_NAME) during tzset() and DST rule parsing.
static TZ_LOCK: trona::sync::Mutex = trona::sync::Mutex::new();

/// Get per-thread Tm buffer (TLS), falling back to global static pre-TLS.
fn get_tls_tm_buf() -> *mut Tm {
    if let Some(tls) = trona_posix::tls::current_tls() {
        unsafe { (&raw mut (*tls).libc_tm_buf) as *mut Tm }
    } else {
        unsafe { &raw mut TM_BUF }
    }
}

/// Get per-thread asctime buffer (TLS), falling back to global static pre-TLS.
fn get_tls_asctime_buf() -> *mut u8 {
    if let Some(tls) = trona_posix::tls::current_tls() {
        unsafe { (&raw mut (*tls).libc_asctime_buf) as *mut u8 }
    } else {
        unsafe { (&raw mut ASCTIME_BUF) as *mut u8 }
    }
}

/// Get per-thread ctime buffer (TLS), falling back to global static pre-TLS.
fn get_tls_ctime_buf() -> *mut u8 {
    if let Some(tls) = trona_posix::tls::current_tls() {
        unsafe { (&raw mut (*tls).libc_ctime_buf) as *mut u8 }
    } else {
        unsafe { (&raw mut CTIME_BUF) as *mut u8 }
    }
}

static mut LOGGED_CLOCK_GETTIME_CALLS: u8 = 0;

fn log_clock_gettime(clock_id: i32, ret: i32, ts: &trona::types::Timespec) {
    unsafe {
        if *(&raw const LOGGED_CLOCK_GETTIME_CALLS) >= 32 {
            return;
        }
        *(&raw mut LOGGED_CLOCK_GETTIME_CALLS) += 1;
    }
    trona::udebug!(|_lb| {
        _lb.str(b"[libc] clock_gettime id=");
        _lb.dec(clock_id as u64);
        _lb.str(b" ret=");
        if ret >= 0 {
            _lb.dec(ret as u64);
        } else {
            _lb.str(b"-");
            _lb.dec((-ret) as u64);
        }
        _lb.str(b" sec=");
        if ts.tv_sec as i64 >= 0 {
            _lb.dec(ts.tv_sec);
        } else {
            _lb.str(b"-");
            _lb.dec((-((ts.tv_sec) as i64)) as u64);
        }
        _lb.str(b" nsec=");
        if ts.tv_nsec as i64 >= 0 {
            _lb.dec(ts.tv_nsec);
        } else {
            _lb.str(b"-");
            _lb.dec((-((ts.tv_nsec) as i64)) as u64);
        }
        _lb.putc(b'\n');
    });
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub type TimeT = i64;
pub type ClockT = i64;

/// Broken-down time representation (POSIX `struct tm`).
#[repr(C)]
pub struct Tm {
    /// Seconds (0-60; 60 for leap second).
    pub tm_sec: i32,
    /// Minutes (0-59).
    pub tm_min: i32,
    /// Hours (0-23).
    pub tm_hour: i32,
    /// Day of the month (1-31).
    pub tm_mday: i32,
    /// Month (0-11; 0 = January).
    pub tm_mon: i32,
    /// Years since 1900 (e.g. 126 for year 2026).
    pub tm_year: i32,
    /// Day of the week (0-6; 0 = Sunday).
    pub tm_wday: i32,
    /// Day of the year (0-365).
    pub tm_yday: i32,
    /// Daylight saving time flag (>0 = DST, 0 = not DST, <0 = auto-detect).
    pub tm_isdst: i32,
    /// Seconds east of UTC for this broken-down time.
    pub tm_gmtoff: i64,
    /// Abbreviated timezone name for this broken-down time.
    pub tm_zone: *const u8,
}

#[repr(C)]
pub struct Tms {
    pub tms_utime: ClockT,
    pub tms_stime: ClockT,
    pub tms_cutime: ClockT,
    pub tms_cstime: ClockT,
}

#[repr(C)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

// ---------------------------------------------------------------------------
// Static buffers for non-reentrant functions
// ---------------------------------------------------------------------------

static mut TM_BUF: Tm = Tm {
    tm_sec: 0,
    tm_min: 0,
    tm_hour: 0,
    tm_mday: 0,
    tm_mon: 0,
    tm_year: 0,
    tm_wday: 0,
    tm_yday: 0,
    tm_isdst: 0,
    tm_gmtoff: 0,
    tm_zone: core::ptr::null(),
};

static mut ASCTIME_BUF: [u8; 64] = [0; 64];
static mut CTIME_BUF: [u8; 64] = [0; 64];

// ---------------------------------------------------------------------------
// POSIX timezone globals
// ---------------------------------------------------------------------------

static mut TZ_STD_NAME: [u8; 16] = [b'U', b'T', b'C', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
static mut TZ_DST_NAME: [u8; 16] = [0; 16];

/// Timezone name pair: tzname[0] = standard, tzname[1] = DST.
/// Initialized to null; tzset() sets the actual pointers.
#[unsafe(no_mangle)]
pub static mut tzname: [*mut u8; 2] = [core::ptr::null_mut(); 2];

/// Seconds west of UTC (POSIX: positive = west of prime meridian).
#[unsafe(no_mangle)]
pub static mut timezone: i64 = 0;

/// Non-zero if the timezone has a DST rule.
#[unsafe(no_mangle)]
pub static mut daylight: i32 = 0;

/// Whether tzset() has been called at least once.
static mut TZ_SET: bool = false;

/// DST offset in seconds west of UTC (typically std_offset - 3600).
static mut TZ_DST_OFFSET: i64 = 0;

/// DST transition rule (POSIX Mm.w.d/time format).
struct TzRule {
    month: i32, // 1-12
    week: i32,  // 1-5 (5 = last)
    wday: i32,  // 0-6 (0 = Sunday)
    time: i32,  // transition time in seconds from midnight (default 7200 = 02:00)
}

static mut DST_START: TzRule = TzRule { month: 0, week: 0, wday: 0, time: 7200 };
static mut DST_END: TzRule = TzRule { month: 0, week: 0, wday: 0, time: 7200 };

// ---------------------------------------------------------------------------
// Day and month name tables
// ---------------------------------------------------------------------------

static WDAY_ABBR: [&[u8]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];

static WDAY_FULL: [&[u8]; 7] = [
    b"Sunday",
    b"Monday",
    b"Tuesday",
    b"Wednesday",
    b"Thursday",
    b"Friday",
    b"Saturday",
];

static MON_ABBR: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

static MON_FULL: [&[u8]; 12] = [
    b"January",
    b"February",
    b"March",
    b"April",
    b"May",
    b"June",
    b"July",
    b"August",
    b"September",
    b"October",
    b"November",
    b"December",
];

static DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_year(year: i32) -> i32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

fn days_in_month(month: i32, year: i32) -> i32 {
    if month == 1 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[month as usize]
    }
}

/// Convert a Unix epoch timestamp to broken-down time (UTC).
unsafe fn epoch_to_tm(epoch: TimeT, tm: *mut Tm) {
    unsafe {
        let mut rem = epoch;
        let mut days: i64;

        if rem >= 0 {
            days = rem / 86400;
            rem %= 86400;
        } else {
            days = rem / 86400;
            rem %= 86400;
            if rem < 0 {
                days -= 1;
                rem += 86400;
            }
        }

        (*tm).tm_hour = (rem / 3600) as i32;
        rem %= 3600;
        (*tm).tm_min = (rem / 60) as i32;
        (*tm).tm_sec = (rem % 60) as i32;

        // Day of week: Jan 1, 1970 = Thursday (4)
        let mut wday = ((days % 7) + 4) % 7;
        if wday < 0 {
            wday += 7;
        }
        (*tm).tm_wday = wday as i32;

        // Compute year from days since epoch
        let mut year: i32 = 1970;
        if days >= 0 {
            loop {
                let dy = days_in_year(year) as i64;
                if days < dy {
                    break;
                }
                days -= dy;
                year += 1;
            }
        } else {
            loop {
                year -= 1;
                let dy = days_in_year(year) as i64;
                days += dy;
                if days >= 0 {
                    break;
                }
            }
        }

        (*tm).tm_year = year - 1900;
        (*tm).tm_yday = days as i32;

        // Compute month and day from day-of-year
        let mut mon: i32 = 0;
        let mut remaining = days as i32;
        while mon < 11 {
            let dm = days_in_month(mon, year);
            if remaining < dm {
                break;
            }
            remaining -= dm;
            mon += 1;
        }

        (*tm).tm_mon = mon;
        (*tm).tm_mday = remaining + 1;
        (*tm).tm_isdst = 0;
        (*tm).tm_gmtoff = 0;
        (*tm).tm_zone = (&raw const TZ_STD_NAME) as *const u8;
    }
}

unsafe fn tz_name_for(is_dst: bool) -> *const u8 {
    unsafe {
        let names = &raw const tzname;
        if is_dst && !(*names)[1].is_null() && *(*names)[1] != 0 {
            (*names)[1] as *const u8
        } else {
            (*names)[0] as *const u8
        }
    }
}

/// Convert broken-down time back to epoch.
fn tm_to_epoch(tm: *const Tm) -> TimeT {
    unsafe {
        let year = (*tm).tm_year + 1900;
        let mon = (*tm).tm_mon;
        let mday = (*tm).tm_mday;

        // Count days from epoch to start of this year
        let mut days: i64 = 0;
        if year >= 1970 {
            let mut y = 1970;
            while y < year {
                days += days_in_year(y) as i64;
                y += 1;
            }
        } else {
            let mut y = 1969;
            while y >= year {
                days -= days_in_year(y) as i64;
                y -= 1;
            }
        }

        // Add days for months
        let mut m: i32 = 0;
        while m < mon && m < 12 {
            days += days_in_month(m, year) as i64;
            m += 1;
        }

        // Add day of month (1-based)
        days += (mday - 1) as i64;

        days * 86400 + (*tm).tm_hour as i64 * 3600 + (*tm).tm_min as i64 * 60 + (*tm).tm_sec as i64
    }
}

/// Write a decimal number with zero-padding into buf. Returns number of bytes written.
unsafe fn write_padded(buf: *mut u8, max: usize, val: i32, width: usize) -> usize {
    unsafe {
        if max == 0 {
            return 0;
        }
        let mut tmp: [u8; 16] = [0; 16];
        let mut v = if val < 0 { -(val as i64) } else { val as i64 };
        let neg = val < 0;
        let mut i: usize = 0;

        if v == 0 {
            tmp[0] = b'0';
            i = 1;
        } else {
            while v > 0 && i < 16 {
                tmp[i] = b'0' + (v % 10) as u8;
                v /= 10;
                i += 1;
            }
        }

        let digits = i;
        let total_width = if width > digits { width } else { digits };
        let sign_len = if neg { 1 } else { 0 };
        let total = total_width + sign_len;

        if total > max {
            return 0;
        }

        let mut pos: usize = 0;
        if neg {
            *buf.add(pos) = b'-';
            pos += 1;
        }

        // Zero padding
        let pad = if total_width > digits {
            total_width - digits
        } else {
            0
        };
        let mut p = 0usize;
        while p < pad {
            *buf.add(pos) = b'0';
            pos += 1;
            p += 1;
        }

        // Digits in reverse order
        let mut d = digits;
        while d > 0 {
            d -= 1;
            *buf.add(pos) = tmp[d];
            pos += 1;
        }

        pos
    }
}

/// Copy a byte slice into buf. Returns bytes written.
unsafe fn write_str(buf: *mut u8, max: usize, s: &[u8]) -> usize {
    unsafe {
        let len = s.len();
        if len > max {
            return 0;
        }
        core::ptr::copy_nonoverlapping(s.as_ptr(), buf, len);
        len
    }
}

/// Helper to get time value from salty
unsafe fn get_epoch_secs() -> TimeT {
    unsafe {
        let mut ts = trona::types::Timespec::zeroed();
        let ret = trona_posix::posix_clock_gettime(0, &raw mut ts);
        if ret < 0 {
            return 0;
        }
        ts.tv_sec as TimeT
    }
}

// ---------------------------------------------------------------------------
// Timezone parsing and DST logic
// ---------------------------------------------------------------------------

/// Parse a timezone name from the TZ string. Names are either:
/// - 3+ alphabetic characters (e.g., EST, KST)
/// - Angle-bracket form: <...> (e.g., <+09>)
/// Returns (number of bytes consumed into dst_name, rest position in src).
unsafe fn parse_tz_name(src: *const u8, pos: usize, dst_name: *mut u8, max: usize) -> (usize, usize) {
    unsafe {
        let mut p = pos;
        let mut n = 0usize;

        if *src.add(p) == b'<' {
            // Angle-bracket form
            p += 1;
            while *src.add(p) != 0 && *src.add(p) != b'>' && n < max - 1 {
                *dst_name.add(n) = *src.add(p);
                n += 1;
                p += 1;
            }
            if *src.add(p) == b'>' {
                p += 1;
            }
        } else {
            // Alphabetic form (3+ chars)
            while *src.add(p) != 0
                && ((*src.add(p) >= b'A' && *src.add(p) <= b'Z')
                    || (*src.add(p) >= b'a' && *src.add(p) <= b'z'))
                && n < max - 1
            {
                *dst_name.add(n) = *src.add(p);
                n += 1;
                p += 1;
            }
        }
        *dst_name.add(n) = 0;
        (n, p)
    }
}

/// Parse a timezone offset: [+/-]hh[:mm[:ss]].
/// POSIX: positive = west of UTC.
/// Returns (offset in seconds, rest position).
unsafe fn parse_tz_offset(src: *const u8, pos: usize) -> (i64, usize) {
    unsafe {
        let mut p = pos;
        let mut sign: i64 = 1;

        if *src.add(p) == b'+' {
            p += 1;
        } else if *src.add(p) == b'-' {
            sign = -1;
            p += 1;
        }

        // Parse hours (required)
        let (hh, adv) = parse_digits(src.add(p), 2);
        if adv == 0 {
            return (0, pos);
        }
        p += adv;

        let mut mm = 0i32;
        let mut ss = 0i32;

        // Optional :mm
        if *src.add(p) == b':' {
            p += 1;
            let (val, adv2) = parse_digits(src.add(p), 2);
            mm = val;
            p += adv2;

            // Optional :ss
            if *src.add(p) == b':' {
                p += 1;
                let (val2, adv3) = parse_digits(src.add(p), 2);
                ss = val2;
                p += adv3;
            }
        }

        let offset = sign * (hh as i64 * 3600 + mm as i64 * 60 + ss as i64);
        (offset, p)
    }
}

/// Parse a DST transition rule: Mm.w.d[/time]
/// Returns rest position. Writes into the given TzRule.
unsafe fn parse_tz_rule(src: *const u8, pos: usize, rule: *mut TzRule) -> usize {
    unsafe {
        let mut p = pos;

        if *src.add(p) != b'M' {
            return p;
        }
        p += 1;

        // Month (1-12)
        let (month, adv) = parse_digits(src.add(p), 2);
        p += adv;
        (*rule).month = month;

        if *src.add(p) == b'.' {
            p += 1;
        }

        // Week (1-5)
        let (week, adv2) = parse_digits(src.add(p), 1);
        p += adv2;
        (*rule).week = week;

        if *src.add(p) == b'.' {
            p += 1;
        }

        // Day of week (0-6, 0=Sunday)
        let (wday, adv3) = parse_digits(src.add(p), 1);
        p += adv3;
        (*rule).wday = wday;

        // Optional /time (hh[:mm[:ss]])
        (*rule).time = 7200; // default 02:00:00
        if *src.add(p) == b'/' {
            p += 1;
            let (off, new_p) = parse_tz_offset(src, p);
            // This is time-of-day, not a UTC offset, so always positive
            (*rule).time = if off < 0 { -off as i32 } else { off as i32 };
            p = new_p;
        }

        p
    }
}

/// Compute the UTC epoch at which a DST transition occurs in a given year.
/// Uses the Mm.w.d rule: month m, week w (1-5, 5=last), day of week d.
unsafe fn rule_to_epoch(rule: *const TzRule, year: i32, utc_offset: i64) -> i64 {
    unsafe {
        let month = (*rule).month; // 1-12
        let week = (*rule).week;   // 1-5
        let wday = (*rule).wday;   // 0-6

        // Epoch of the 1st of the target month
        let mut days: i64 = 0;
        let mut y = 1970;
        while y < year {
            days += days_in_year(y) as i64;
            y += 1;
        }
        let mut m = 0;
        while m < month - 1 {
            days += days_in_month(m, year) as i64;
            m += 1;
        }

        // Day of week of the 1st (Jan 1, 1970 = Thursday = 4)
        let dow_first = ((days % 7) + 4) % 7;

        // First occurrence of target wday in this month
        let mut first_wday = wday as i64 - dow_first;
        if first_wday < 0 {
            first_wday += 7;
        }

        let target_day = if week <= 4 {
            // week 1-4: first_wday + (week-1)*7, 0-based day within month
            first_wday + (week as i64 - 1) * 7
        } else {
            // week 5 means "last": find the last occurrence
            let mut d = first_wday;
            let dim = days_in_month(month - 1, year) as i64;
            while d + 7 < dim {
                d += 7;
            }
            d
        };

        let epoch_midnight = (days + target_day) * 86400;
        // Transition time is in local wall clock time, convert to UTC
        epoch_midnight + (*rule).time as i64 + utc_offset
    }
}

/// Determine whether a given UTC epoch falls in DST.
unsafe fn is_dst(utc: i64) -> bool {
    unsafe {
        if *(&raw const daylight) == 0 {
            return false;
        }

        // Determine the year from the UTC time
        let mut tmp = core::mem::MaybeUninit::<Tm>::uninit();
        epoch_to_tm(utc, tmp.as_mut_ptr());
        let year = (*tmp.as_ptr()).tm_year + 1900;

        let std_off = *(&raw const timezone);
        let dst_off = *(&raw const TZ_DST_OFFSET);

        let start = rule_to_epoch(&raw const DST_START, year, std_off);
        let end = rule_to_epoch(&raw const DST_END, year, dst_off);

        if start < end {
            // Northern hemisphere: DST between start and end
            utc >= start && utc < end
        } else {
            // Southern hemisphere: DST outside [end, start)
            utc >= start || utc < end
        }
    }
}

/// Get the current timezone offset for a given UTC epoch (accounts for DST).
unsafe fn tz_offset_for(utc: i64) -> i64 {
    unsafe {
        if *(&raw const daylight) != 0 && is_dst(utc) {
            *(&raw const TZ_DST_OFFSET)
        } else {
            *(&raw const timezone)
        }
    }
}

/// Initialize timezone from the TZ environment variable.
/// POSIX format: stdoffset[dst[offset][,start[/time],end[/time]]]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tzset() {
    TZ_LOCK.lock();
    unsafe {
        *(&raw mut TZ_SET) = true;

        // Always set tzname pointers to our static buffers
        let tn = &raw mut tzname;
        (*tn)[0] = (&raw mut TZ_STD_NAME) as *mut u8;
        (*tn)[1] = (&raw mut TZ_DST_NAME) as *mut u8;

        unsafe extern "C" {
            fn getenv(name: *const u8) -> *const u8;
        }
        let tz = getenv(b"TZ\0".as_ptr());

        // Default to UTC if TZ is unset or empty
        if tz.is_null() || *tz == 0 {
            let std_name = &raw mut TZ_STD_NAME;
            (*std_name)[0] = b'U';
            (*std_name)[1] = b'T';
            (*std_name)[2] = b'C';
            (*std_name)[3] = 0;
            let dst_name = &raw mut TZ_DST_NAME;
            (*dst_name)[0] = 0;
            *(&raw mut timezone) = 0;
            *(&raw mut daylight) = 0;
            *(&raw mut TZ_DST_OFFSET) = 0;
            TZ_LOCK.unlock();
            return;
        }

        let mut p = 0usize;

        // Parse standard timezone name
        let (name_len, new_p) = parse_tz_name(tz, p, (&raw mut TZ_STD_NAME) as *mut u8, 16);
        p = new_p;

        if name_len == 0 {
            // Invalid TZ, fall back to UTC
            TZ_LOCK.unlock();
            return;
        }

        // Parse standard offset (required if name is not just "UTC" with implicit 0)
        if *tz.add(p) == 0 || *tz.add(p) == b',' {
            // Name only (e.g., "UTC") — offset = 0
            *(&raw mut timezone) = 0;
        } else {
            let (off, new_p2) = parse_tz_offset(tz, p);
            p = new_p2;
            *(&raw mut timezone) = off;
        }

        // Parse optional DST name
        let has_dst = *tz.add(p) != 0 && *tz.add(p) != b',';
        if has_dst {
            let (dst_len, new_p3) = parse_tz_name(tz, p, (&raw mut TZ_DST_NAME) as *mut u8, 16);
            p = new_p3;

            if dst_len == 0 {
                *(&raw mut daylight) = 0;
                TZ_LOCK.unlock();
                return;
            }

            *(&raw mut daylight) = 1;

            // Parse optional DST offset (default: std - 1 hour)
            if *tz.add(p) != 0 && *tz.add(p) != b',' {
                let (dst_off, new_p4) = parse_tz_offset(tz, p);
                p = new_p4;
                *(&raw mut TZ_DST_OFFSET) = dst_off;
            } else {
                *(&raw mut TZ_DST_OFFSET) = *(&raw const timezone) - 3600;
            }

            // Parse optional DST transition rules: ,start[/time],end[/time]
            if *tz.add(p) == b',' {
                p += 1;
                p = parse_tz_rule(tz, p, &raw mut DST_START);

                if *tz.add(p) == b',' {
                    p += 1;
                    let _ = parse_tz_rule(tz, p, &raw mut DST_END);
                }
            }
        } else {
            *(&raw mut daylight) = 0;
            let dst_name = &raw mut TZ_DST_NAME;
            (*dst_name)[0] = 0;
        }
    }
    TZ_LOCK.unlock();
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn time(t: *mut TimeT) -> TimeT {
    unsafe {
        let secs = get_epoch_secs();
        if !t.is_null() {
            *t = secs;
        }
        secs
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettimeofday(tv: *mut Timeval, _tz: *mut u8) -> i32 {
    unsafe {
        let mut stv = trona::types::Timeval::zeroed();
        let ret = trona_posix::posix_gettimeofday(&raw mut stv);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        if !tv.is_null() {
            (*tv).tv_sec = stv.tv_sec as i64;
            (*tv).tv_usec = stv.tv_usec as i64;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_gettime(clock_id: i32, tp: *mut Timespec) -> i32 {
    unsafe {
        let mut sts = trona::types::Timespec::zeroed();
        let ret = trona_posix::posix_clock_gettime(clock_id, &raw mut sts);
        log_clock_gettime(clock_id, ret, &sts);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        if !tp.is_null() {
            (*tp).tv_sec = sts.tv_sec as i64;
            (*tp).tv_nsec = sts.tv_nsec as i64;
        }
        0
    }
}

/// clock_getres — report timer resolution.
/// SaltyOS kernel timer tick is 10ms; nanosecond precision for monotonic.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_getres(clock_id: i32, res: *mut Timespec) -> i32 {
    if !res.is_null() {
        unsafe {
            (*res).tv_sec = 0;
            // Support both native SaltyOS clock IDs and the FreeBSD aliases
            // used by imported userland.
            (*res).tv_nsec = match clock_id {
                0 | 1 | 4 | 9 | 10 | 11 | 12 => 1,
                _ => 10_000_000,
            };
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock_settime(_clock_id: i32, _tp: *const Timespec) -> i32 {
    errno::set_errno(errno::EPERM);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn clock() -> ClockT {
    unsafe {
        let secs = get_epoch_secs();
        secs * 1_000_000
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn times(buf: *mut Tms) -> ClockT {
    unsafe {
        if !buf.is_null() {
            (*buf).tms_utime = 0;
            (*buf).tms_stime = 0;
            (*buf).tms_cutime = 0;
            (*buf).tms_cstime = 0;
        }
        get_epoch_secs() * 100
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn difftime(time1: TimeT, time0: TimeT) -> f64 {
    (time1 - time0) as f64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gmtime_r(timep: *const TimeT, result: *mut Tm) -> *mut Tm {
    unsafe {
        if timep.is_null() || result.is_null() {
            return core::ptr::null_mut();
        }
        epoch_to_tm(*timep, result);
        (*result).tm_isdst = 0;
        (*result).tm_gmtoff = 0;
        (*result).tm_zone = b"UTC\0".as_ptr();
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gmtime(timep: *const TimeT) -> *mut Tm {
    unsafe {
        if timep.is_null() {
            return core::ptr::null_mut();
        }
        let buf = get_tls_tm_buf();
        gmtime_r(timep, buf)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn localtime_r(timep: *const TimeT, result: *mut Tm) -> *mut Tm {
    unsafe {
        if !*(&raw const TZ_SET) {
            tzset();
        }
        if timep.is_null() || result.is_null() {
            return core::ptr::null_mut();
        }
        let utc = *timep;
        let offset = tz_offset_for(utc);
        let is_dst_now = *(&raw const daylight) != 0 && is_dst(utc);
        let local = utc - offset; // timezone is seconds WEST, subtract to get local
        epoch_to_tm(local, result);
        (*result).tm_isdst = if is_dst_now { 1 } else { 0 };
        (*result).tm_gmtoff = -offset;
        (*result).tm_zone = tz_name_for(is_dst_now);
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn localtime(timep: *const TimeT) -> *mut Tm {
    unsafe {
        if timep.is_null() {
            return core::ptr::null_mut();
        }
        let buf = get_tls_tm_buf();
        localtime_r(timep, buf)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mktime(tm: *mut Tm) -> TimeT {
    unsafe {
        if !*(&raw const TZ_SET) {
            tzset();
        }
        if tm.is_null() {
            return -1;
        }
        // Interpret the Tm as local time; convert to UTC epoch
        let local_epoch = tm_to_epoch(tm);
        let offset = if (*tm).tm_isdst > 0 {
            *(&raw const TZ_DST_OFFSET)
        } else if (*tm).tm_isdst == 0 {
            *(&raw const timezone)
        } else {
            // tm_isdst == -1: auto-detect DST from a UTC guess
            let utc_guess = local_epoch + *(&raw const timezone);
            if *(&raw const daylight) != 0 && is_dst(utc_guess) {
                *(&raw const TZ_DST_OFFSET)
            } else {
                *(&raw const timezone)
            }
        };
        let utc = local_epoch + offset;
        // Normalize the Tm struct back to local time
        epoch_to_tm(utc - tz_offset_for(utc), tm);
        let is_dst_now = *(&raw const daylight) != 0 && is_dst(utc);
        let final_off = tz_offset_for(utc);
        (*tm).tm_isdst = if is_dst_now { 1 } else { 0 };
        (*tm).tm_gmtoff = -final_off;
        (*tm).tm_zone = tz_name_for(is_dst_now);
        utc
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asctime_r(tm: *const Tm, buf: *mut u8) -> *mut u8 {
    unsafe {
        if tm.is_null() || buf.is_null() {
            return core::ptr::null_mut();
        }

        let wday = if (*tm).tm_wday >= 0 && (*tm).tm_wday < 7 {
            (*tm).tm_wday as usize
        } else {
            0
        };
        let mon = if (*tm).tm_mon >= 0 && (*tm).tm_mon < 12 {
            (*tm).tm_mon as usize
        } else {
            0
        };

        // Format: "Day Mon DD HH:MM:SS YYYY\n\0"
        let mut pos: usize = 0;

        // Day abbreviation
        pos += write_str(buf.add(pos), 64 - pos, WDAY_ABBR[wday]);
        *buf.add(pos) = b' ';
        pos += 1;

        // Month abbreviation
        pos += write_str(buf.add(pos), 64 - pos, MON_ABBR[mon]);
        *buf.add(pos) = b' ';
        pos += 1;

        // Day of month with space padding
        if (*tm).tm_mday < 10 {
            *buf.add(pos) = b' ';
            pos += 1;
            pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_mday, 1);
        } else {
            pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_mday, 2);
        }
        *buf.add(pos) = b' ';
        pos += 1;

        // HH:MM:SS
        pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_hour, 2);
        *buf.add(pos) = b':';
        pos += 1;
        pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_min, 2);
        *buf.add(pos) = b':';
        pos += 1;
        pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_sec, 2);
        *buf.add(pos) = b' ';
        pos += 1;

        // Year (4-digit)
        pos += write_padded(buf.add(pos), 64 - pos, (*tm).tm_year + 1900, 4);
        *buf.add(pos) = b'\n';
        pos += 1;
        *buf.add(pos) = 0;

        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asctime(tm: *const Tm) -> *mut u8 {
    unsafe { asctime_r(tm, get_tls_asctime_buf()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ctime_r(timep: *const TimeT, buf: *mut u8) -> *mut u8 {
    unsafe {
        let mut tmp = Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone: core::ptr::null(),
        };
        if localtime_r(timep, &raw mut tmp).is_null() {
            return core::ptr::null_mut();
        }
        asctime_r(&raw const tmp, buf)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ctime(timep: *const TimeT) -> *mut u8 {
    unsafe { ctime_r(timep, get_tls_ctime_buf()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strftime(
    buf: *mut u8,
    maxsize: usize,
    format: *const u8,
    tm: *const Tm,
) -> usize {
    unsafe {
        if buf.is_null() || format.is_null() || tm.is_null() || maxsize == 0 {
            return 0;
        }

        let fmtlen = crate::string::strlen(format);
        let mut pos: usize = 0;
        let mut fi: usize = 0;
        let remaining = maxsize - 1; // reserve space for null terminator

        while fi < fmtlen {
            if pos >= remaining {
                return 0;
            }

            let c = *format.add(fi);
            if c != b'%' {
                *buf.add(pos) = c;
                pos += 1;
                fi += 1;
                continue;
            }

            fi += 1;
            if fi >= fmtlen {
                break;
            }

            let spec = *format.add(fi);
            fi += 1;

            let avail = remaining - pos;

            let written = match spec {
                // %% — literal %
                b'%' => {
                    if avail < 1 {
                        return 0;
                    }
                    *buf.add(pos) = b'%';
                    1
                }

                // %Y — 4-digit year
                b'Y' => write_padded(buf.add(pos), avail, (*tm).tm_year + 1900, 4),

                // %m — month 01-12
                b'm' => write_padded(buf.add(pos), avail, (*tm).tm_mon + 1, 2),

                // %d — day 01-31
                b'd' => write_padded(buf.add(pos), avail, (*tm).tm_mday, 2),

                // %e — day with space-padding
                b'e' => {
                    if avail < 2 {
                        return 0;
                    }
                    if (*tm).tm_mday < 10 {
                        *buf.add(pos) = b' ';
                        1 + write_padded(buf.add(pos + 1), avail - 1, (*tm).tm_mday, 1)
                    } else {
                        write_padded(buf.add(pos), avail, (*tm).tm_mday, 2)
                    }
                }

                // %H — hour 00-23
                b'H' => write_padded(buf.add(pos), avail, (*tm).tm_hour, 2),

                // %I — hour 01-12
                b'I' => {
                    let h = (*tm).tm_hour % 12;
                    let h12 = if h == 0 { 12 } else { h };
                    write_padded(buf.add(pos), avail, h12, 2)
                }

                // %M — minute 00-59
                b'M' => write_padded(buf.add(pos), avail, (*tm).tm_min, 2),

                // %S — second 00-60
                b'S' => write_padded(buf.add(pos), avail, (*tm).tm_sec, 2),

                // %p — AM/PM
                b'p' => {
                    let s = if (*tm).tm_hour < 12 {
                        b"AM" as &[u8]
                    } else {
                        b"PM"
                    };
                    write_str(buf.add(pos), avail, s)
                }

                // %a — abbreviated weekday
                b'a' => {
                    let wday = if (*tm).tm_wday >= 0 && (*tm).tm_wday < 7 {
                        (*tm).tm_wday as usize
                    } else {
                        0
                    };
                    write_str(buf.add(pos), avail, WDAY_ABBR[wday])
                }

                // %A — full weekday
                b'A' => {
                    let wday = if (*tm).tm_wday >= 0 && (*tm).tm_wday < 7 {
                        (*tm).tm_wday as usize
                    } else {
                        0
                    };
                    write_str(buf.add(pos), avail, WDAY_FULL[wday])
                }

                // %b, %h — abbreviated month
                b'b' | b'h' => {
                    let mon = if (*tm).tm_mon >= 0 && (*tm).tm_mon < 12 {
                        (*tm).tm_mon as usize
                    } else {
                        0
                    };
                    write_str(buf.add(pos), avail, MON_ABBR[mon])
                }

                // %B — full month name
                b'B' => {
                    let mon = if (*tm).tm_mon >= 0 && (*tm).tm_mon < 12 {
                        (*tm).tm_mon as usize
                    } else {
                        0
                    };
                    write_str(buf.add(pos), avail, MON_FULL[mon])
                }

                // %c — date and time: "Day Mon DD HH:MM:SS YYYY"
                b'c' => {
                    let n = strftime(
                        buf.add(pos),
                        avail + 1,
                        b"%a %b %e %H:%M:%S %Y\0".as_ptr(),
                        tm,
                    );
                    if n == 0 && avail > 0 {
                        return 0;
                    }
                    n
                }

                // %x — date: "MM/DD/YY"
                b'x' => {
                    let n = strftime(buf.add(pos), avail + 1, b"%m/%d/%y\0".as_ptr(), tm);
                    if n == 0 && avail > 0 {
                        return 0;
                    }
                    n
                }

                // %X — time: "HH:MM:SS"
                b'X' => {
                    let n = strftime(buf.add(pos), avail + 1, b"%H:%M:%S\0".as_ptr(), tm);
                    if n == 0 && avail > 0 {
                        return 0;
                    }
                    n
                }

                // %y — 2-digit year
                b'y' => write_padded(buf.add(pos), avail, ((*tm).tm_year + 1900) % 100, 2),

                // %j — day of year 001-366
                b'j' => write_padded(buf.add(pos), avail, (*tm).tm_yday + 1, 3),

                // %w — weekday 0-6 (Sunday=0)
                b'w' => write_padded(buf.add(pos), avail, (*tm).tm_wday, 1),

                // %u — weekday 1-7 (Monday=1)
                b'u' => {
                    let u = if (*tm).tm_wday == 0 { 7 } else { (*tm).tm_wday };
                    write_padded(buf.add(pos), avail, u, 1)
                }

                // %Z — timezone name
                b'Z' => {
                    if !*(&raw const TZ_SET) {
                        tzset();
                    }
                    let names = &raw const tzname;
                    let name_ptr = if (*tm).tm_isdst > 0 {
                        (*names)[1] as *const u8
                    } else {
                        (*names)[0] as *const u8
                    };
                    // Measure name length
                    let mut nlen = 0usize;
                    while *name_ptr.add(nlen) != 0 {
                        nlen += 1;
                    }
                    if nlen > avail {
                        return 0;
                    }
                    core::ptr::copy_nonoverlapping(name_ptr, buf.add(pos), nlen);
                    nlen
                }

                // %z — timezone offset (+HHMM or -HHMM)
                b'z' => {
                    if !*(&raw const TZ_SET) {
                        tzset();
                    }
                    if avail < 5 {
                        return 0;
                    }
                    let off = if (*tm).tm_isdst > 0 {
                        *(&raw const TZ_DST_OFFSET)
                    } else {
                        *(&raw const timezone)
                    };
                    // POSIX: timezone>0 = west = negative in %z
                    let (sign, abs_off) = if off <= 0 {
                        (b'+', (-off) as u64)
                    } else {
                        (b'-', off as u64)
                    };
                    let hh = abs_off / 3600;
                    let mm = (abs_off % 3600) / 60;
                    *buf.add(pos) = sign;
                    *buf.add(pos + 1) = b'0' + (hh / 10) as u8;
                    *buf.add(pos + 2) = b'0' + (hh % 10) as u8;
                    *buf.add(pos + 3) = b'0' + (mm / 10) as u8;
                    *buf.add(pos + 4) = b'0' + (mm % 10) as u8;
                    5
                }

                // %R — %H:%M
                b'R' => {
                    let n = strftime(buf.add(pos), avail + 1, b"%H:%M\0".as_ptr(), tm);
                    if n == 0 && avail > 0 {
                        return 0;
                    }
                    n
                }

                // %T — %H:%M:%S
                b'T' => {
                    let n = strftime(buf.add(pos), avail + 1, b"%H:%M:%S\0".as_ptr(), tm);
                    if n == 0 && avail > 0 {
                        return 0;
                    }
                    n
                }

                // %n — newline
                b'n' => {
                    if avail < 1 {
                        return 0;
                    }
                    *buf.add(pos) = b'\n';
                    1
                }

                // %t — tab
                b't' => {
                    if avail < 1 {
                        return 0;
                    }
                    *buf.add(pos) = b'\t';
                    1
                }

                // Unknown specifier: output as-is
                _ => {
                    if avail < 2 {
                        return 0;
                    }
                    *buf.add(pos) = b'%';
                    *buf.add(pos + 1) = spec;
                    2
                }
            };

            if written == 0 && spec != b'%' && spec != b'n' && spec != b't' {
                // Buffer too small for this specifier
                return 0;
            }

            pos += written;
        }

        *buf.add(pos) = 0;
        pos
    }
}

// ---------------------------------------------------------------------------
// Interval timer stubs
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct Itimerval {
    pub it_interval: Timeval,
    pub it_value: Timeval,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setitimer(
    which: i32,
    new_value: *const Itimerval,
    old_value: *mut Itimerval,
) -> i32 {
    unsafe {
        if new_value.is_null() {
            errno::set_errno(errno::EFAULT);
            return -1;
        }
        if which != 0 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let nv = &*new_value;
        if nv.it_value.tv_sec < 0
            || nv.it_value.tv_usec < 0
            || nv.it_value.tv_usec >= 1_000_000
            || nv.it_interval.tv_sec < 0
            || nv.it_interval.tv_usec < 0
            || nv.it_interval.tv_usec >= 1_000_000
        {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let trona_new = trona_posix::Itimerval {
            it_interval: trona_posix::Timeval {
                tv_sec: nv.it_interval.tv_sec as u64,
                tv_usec: nv.it_interval.tv_usec as u64,
            },
            it_value: trona_posix::Timeval {
                tv_sec: nv.it_value.tv_sec as u64,
                tv_usec: nv.it_value.tv_usec as u64,
            },
        };
        let mut trona_old = trona_posix::Itimerval::zeroed();

        let ret = trona_posix::posix_setitimer(
            which,
            &raw const trona_new,
            if old_value.is_null() { core::ptr::null_mut() } else { &raw mut trona_old },
        );
        if ret != 0 {
            errno::set_errno(-ret);
            return -1;
        }

        if !old_value.is_null() {
            (*old_value).it_interval.tv_sec = trona_old.it_interval.tv_sec as i64;
            (*old_value).it_interval.tv_usec = trona_old.it_interval.tv_usec as i64;
            (*old_value).it_value.tv_sec = trona_old.it_value.tv_sec as i64;
            (*old_value).it_value.tv_usec = trona_old.it_value.tv_usec as i64;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getitimer(which: i32, curr_value: *mut Itimerval) -> i32 {
    unsafe {
        if curr_value.is_null() {
            errno::set_errno(errno::EFAULT);
            return -1;
        }
        if which != 0 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let mut trona_cur = trona_posix::Itimerval::zeroed();
        let ret = trona_posix::posix_getitimer(which, &raw mut trona_cur);
        if ret != 0 {
            errno::set_errno(-ret);
            return -1;
        }

        (*curr_value).it_interval.tv_sec = trona_cur.it_interval.tv_sec as i64;
        (*curr_value).it_interval.tv_usec = trona_cur.it_interval.tv_usec as i64;
        (*curr_value).it_value.tv_sec = trona_cur.it_value.tv_sec as i64;
        (*curr_value).it_value.tv_usec = trona_cur.it_value.tv_usec as i64;
    }
    0
}

// ---------------------------------------------------------------------------
// timegm / strptime
// ---------------------------------------------------------------------------

/// timegm — like mktime() but always interprets tm as UTC.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn timegm(tm: *mut Tm) -> TimeT {
    unsafe {
        if tm.is_null() {
            return -1;
        }
        let epoch = tm_to_epoch(tm);
        epoch_to_tm(epoch, tm);
        (*tm).tm_isdst = 0;
        (*tm).tm_gmtoff = 0;
        (*tm).tm_zone = b"UTC\0".as_ptr();
        epoch
    }
}

/// strptime — parse a time string according to a format.
/// Minimal implementation supporting: %Y %m %d %H %M %S %T %D %n %t %%
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strptime(buf: *const u8, fmt: *const u8, tm: *mut Tm) -> *mut u8 {
    unsafe {
        if buf.is_null() || fmt.is_null() || tm.is_null() {
            return core::ptr::null_mut();
        }

        let mut bi: usize = 0; // index into buf
        let mut fi: usize = 0; // index into fmt

        while *fmt.add(fi) != 0 {
            let fc = *fmt.add(fi);

            if fc == b'%' {
                fi += 1;
                if *fmt.add(fi) == 0 {
                    break;
                }
                let spec = *fmt.add(fi);
                fi += 1;

                match spec {
                    b'Y' => {
                        // 4-digit year
                        let (val, adv) = parse_digits(buf.add(bi), 4);
                        if adv != 4 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_year = val - 1900;
                        bi += adv;
                    }
                    b'm' => {
                        // 1-2 digit month (01-12)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv == 0 || val < 1 || val > 12 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_mon = val - 1;
                        bi += adv;
                    }
                    b'd' => {
                        // 1-2 digit day (01-31)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv == 0 || val < 1 || val > 31 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_mday = val;
                        bi += adv;
                    }
                    b'H' => {
                        // 1-2 digit hour (00-23)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv == 0 || val > 23 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_hour = val;
                        bi += adv;
                    }
                    b'M' => {
                        // 1-2 digit minute (00-59)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv == 0 || val > 59 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_min = val;
                        bi += adv;
                    }
                    b'S' => {
                        // 1-2 digit second (00-60, leap second)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv == 0 || val > 60 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_sec = val;
                        bi += adv;
                    }
                    b'T' => {
                        // %H:%M:%S
                        let result = strptime(buf.add(bi), b"%H:%M:%S\0".as_ptr(), tm);
                        if result.is_null() {
                            return core::ptr::null_mut();
                        }
                        bi += (result as *const u8).offset_from(buf.add(bi)) as usize;
                    }
                    b'D' => {
                        // %m/%d/%y
                        let result = strptime(buf.add(bi), b"%m/%d/%y\0".as_ptr(), tm);
                        if result.is_null() {
                            return core::ptr::null_mut();
                        }
                        bi += (result as *const u8).offset_from(buf.add(bi)) as usize;
                    }
                    b'y' => {
                        // 2-digit year (00-99, maps to 1969-2068)
                        let (val, adv) = parse_digits(buf.add(bi), 2);
                        if adv != 2 {
                            return core::ptr::null_mut();
                        }
                        (*tm).tm_year = if val >= 69 { val } else { val + 100 };
                        bi += adv;
                    }
                    b'n' | b't' => {
                        // Skip whitespace
                        while *buf.add(bi) != 0
                            && (*buf.add(bi) == b' '
                                || *buf.add(bi) == b'\t'
                                || *buf.add(bi) == b'\n')
                        {
                            bi += 1;
                        }
                    }
                    b'%' => {
                        if *buf.add(bi) != b'%' {
                            return core::ptr::null_mut();
                        }
                        bi += 1;
                    }
                    _ => {
                        // Unknown specifier, fail
                        return core::ptr::null_mut();
                    }
                }
            } else if fc == b' ' || fc == b'\t' {
                // Format whitespace matches any amount of input whitespace
                fi += 1;
                while *buf.add(bi) != 0 && (*buf.add(bi) == b' ' || *buf.add(bi) == b'\t') {
                    bi += 1;
                }
            } else {
                // Literal character match
                if *buf.add(bi) != fc {
                    return core::ptr::null_mut();
                }
                bi += 1;
                fi += 1;
            }
        }

        buf.add(bi) as *mut u8
    }
}

/// Parse up to `max_digits` decimal digits from `s`. Returns (value, chars_consumed).
unsafe fn parse_digits(s: *const u8, max_digits: usize) -> (i32, usize) {
    unsafe {
        let mut val: i32 = 0;
        let mut i: usize = 0;
        while i < max_digits && *s.add(i) >= b'0' && *s.add(i) <= b'9' {
            val = val * 10 + (*s.add(i) - b'0') as i32;
            i += 1;
        }
        (val, i)
    }
}
