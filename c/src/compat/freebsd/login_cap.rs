//! FreeBSD login_cap(3) compatibility for SaltyOS.
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Implements the FreeBSD login class capability API against an in-memory
//! `/etc/login.conf` database. This is the authoritative implementation for
//! the SaltyOS system — basaltc provides it to both programs that link
//! `-lutil` (via libutil.so which no longer ships its own login_cap) and
//! programs that do not.
//!
//! The parser ports FreeBSD's `cgetent`/`cgetcap`/`cgetstr` semantics
//! precisely: `tc=` inheritance, `@` boolean negation, first-match precedence,
//! and escape sequence decoding for string values. All storage is static —
//! no heap allocation is required.

use core::ptr;

use crate::pwd::Passwd;

// ===========================================================================
// Configuration constants
// ===========================================================================

const MAX_LOGIN_CLASSES: usize = 16;
/// Maximum size of a single record after `tc=` expansion.
const MAX_CAP_RECORD: usize = 4096;
/// Maximum size of `/etc/login.conf` file buffer.
const LOGIN_CONF_BUF: usize = 16384;
/// Maximum length of a single capability string value (after escape decoding).
const MAX_CAP_VALUE: usize = 1024;
/// Maximum `tc=` recursion depth (matches FreeBSD's MAX_RECURSION = 32, but
/// bounded tighter here since we have no runtime chain growth).
const MAX_TC_DEPTH: usize = 8;
/// Maximum length of a class name.
const MAX_CLASS_NAME: usize = 64;
/// Maximum number of entries in an allow/deny list.
const MAX_LIST_ENTRIES: usize = 16;

// ===========================================================================
// C ABI types
// ===========================================================================

/// FreeBSD `login_cap_t` — three C strings. Instances live in a static pool
/// tied to a parsed login class entry. Pointer returned by `login_getclass`
/// remains valid until process exit; `login_close()` is a no-op.
#[repr(C)]
pub struct LoginCap {
    pub lc_class: *const u8,
    pub lc_cap: *const u8,
    pub lc_style: *const u8,
}

// ===========================================================================
// Internal storage
// ===========================================================================

/// A parsed login class entry.
///
/// `record` holds the full `name:cap1:cap2:...` record after `tc=` expansion,
/// matching the buffer format that FreeBSD's `cgetcap`/`cgetstr` expect. The
/// `lc` field has `lc_class` and `lc_cap` pointers set into the other fields
/// of this same entry.
struct LoginClassEntry {
    active: bool,
    name: [u8; MAX_CLASS_NAME],
    record: [u8; MAX_CAP_RECORD],
    record_len: usize,
    lc: LoginCap,
}

const ZERO_CLASS: LoginClassEntry = LoginClassEntry {
    active: false,
    name: [0; MAX_CLASS_NAME],
    record: [0; MAX_CAP_RECORD],
    record_len: 0,
    lc: LoginCap {
        lc_class: ptr::null(),
        lc_cap: ptr::null(),
        lc_style: ptr::null(),
    },
};

static mut LOGIN_CLASSES: [LoginClassEntry; MAX_LOGIN_CLASSES] =
    [ZERO_CLASS; MAX_LOGIN_CLASSES];
static mut LOGIN_CLASS_COUNT: usize = 0;
static mut LOGIN_CONF_LOADED: bool = false;

/// Fallback LoginCap returned when the requested class is not found.
static mut EMPTY_LC: LoginCap = LoginCap {
    lc_class: ptr::null(),
    lc_cap: ptr::null(),
    lc_style: ptr::null(),
};

/// Scratch buffer for `login_getcapstr` return values. NUL-terminated.
/// Single shared slot — callers must copy if they need the value to outlive
/// the next `login_getcapstr` call. Matches the "transient return" pattern
/// used throughout basaltc's non-reentrant POSIX APIs.
static mut CAPSTR_BUF: [u8; MAX_CAP_VALUE] = [0; MAX_CAP_VALUE];

// ===========================================================================
// Low-level helpers
// ===========================================================================

unsafe fn cstr_len(p: *const u8) -> usize {
    unsafe {
        let mut i = 0usize;
        while *p.add(i) != 0 {
            i += 1;
        }
        i
    }
}

unsafe fn cstr_eq(a: *const u8, b: *const u8) -> bool {
    unsafe {
        let mut i = 0usize;
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
    }
}

/// Case-insensitive C string equality.
unsafe fn cstr_eq_icase(a: *const u8, b: *const u8) -> bool {
    unsafe {
        let mut i = 0usize;
        loop {
            let ca = to_lower(*a.add(i));
            let cb = to_lower(*b.add(i));
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
    }
}

fn to_lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        c + 32
    } else {
        c
    }
}

/// Match a target class name against the `name|alias1|alias2:` header of
/// a login.conf record. Supports FreeBSD's `|`-separated alias syntax.
///
/// `target` is a NUL-terminated C string. `record` is the full record
/// beginning with the name field and terminated by NUL or end-of-slice.
/// Comparison is case-sensitive (FreeBSD getcap uses strcmp, not strcasecmp).
unsafe fn class_name_matches(target: *const u8, record: &[u8]) -> bool {
    unsafe {
        if target.is_null() || *target == 0 {
            return false;
        }
        let tlen = cstr_len(target);
        if tlen == 0 {
            return false;
        }
        let target_bytes = core::slice::from_raw_parts(target, tlen);

        // Scan the name field, which ends at the first `:` (or end).
        let mut i = 0usize;
        while i < record.len() && record[i] != b':' && record[i] != 0 {
            // Delimit this alias: up to `|` or `:` or NUL.
            let start = i;
            while i < record.len()
                && record[i] != b'|'
                && record[i] != b':'
                && record[i] != 0
            {
                i += 1;
            }
            let alias = &record[start..i];
            if alias.len() == target_bytes.len() {
                let mut eq = true;
                for k in 0..alias.len() {
                    if alias[k] != target_bytes[k] {
                        eq = false;
                        break;
                    }
                }
                if eq {
                    return true;
                }
            }
            if i < record.len() && record[i] == b'|' {
                i += 1;
            } else {
                break;
            }
        }
        false
    }
}

/// Compare a C string against a capability name (up to first terminator
/// in `name_bytes` — NUL, `:`, `=`, `#`, or `@`).
fn cap_name_matches(needle: &[u8], haystack: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    for i in 0..needle.len() {
        if haystack[i] != needle[i] {
            return false;
        }
    }
    // After the matched prefix, the next byte must be a terminator.
    if haystack.len() == needle.len() {
        return true;
    }
    let term = haystack[needle.len()];
    matches!(term, 0 | b':' | b'=' | b'#' | b'@')
}

// ===========================================================================
// File reading
// ===========================================================================

unsafe fn read_login_conf(buf: &mut [u8]) -> usize {
    unsafe {
        let fd = trona_posix::posix_open(
            b"/etc/login.conf\0".as_ptr(),
            trona_posix::O_RDONLY as i32,
            0,
        );
        if fd < 0 {
            return 0;
        }
        let mut total: usize = 0;
        while total < buf.len() {
            let remain = buf.len() - total;
            let n = trona_posix::posix_read(fd, buf.as_mut_ptr().add(total), remain as u64);
            if n <= 0 {
                break;
            }
            total += n as usize;
        }
        trona_posix::posix_close(fd);
        total
    }
}

// ===========================================================================
// login.conf parser
// ===========================================================================

/// Parse `/etc/login.conf` into `LOGIN_CLASSES`.
///
/// getcap-compatible parsing:
/// - Lines ending with `\` before `\n` are joined with the next line
/// - Leading whitespace on continuation lines is dropped
/// - Lines starting with `#` are comments
/// - Blank lines are skipped
/// - Each logical line is a record: `name|alias|...:cap1:cap2:...`
///
/// `tc=` expansion is performed as a separate pass after all records are
/// loaded, so forward references work correctly.
unsafe fn load_login_conf() {
    unsafe {
        *(&raw mut LOGIN_CONF_LOADED) = true;
        *(&raw mut LOGIN_CLASS_COUNT) = 0;

        let mut file_buf = [0u8; LOGIN_CONF_BUF];
        let n = read_login_conf(&mut file_buf);
        if n == 0 {
            return;
        }

        parse_records(&file_buf[..n]);
        expand_all_tc();
    }
}

/// First pass: split the file into logical records and store name + raw caps.
unsafe fn parse_records(data: &[u8]) {
    unsafe {
        // Scratch buffer for a single logical line after continuation joining.
        let mut line_buf = [0u8; MAX_CAP_RECORD];
        let mut line_len = 0usize;
        let mut i = 0usize;

        while i < data.len() {
            // Gather one logical line (with continuation handling).
            line_len = 0;
            loop {
                // Find end of physical line.
                let line_start = i;
                let mut j = i;
                while j < data.len() && data[j] != b'\n' {
                    j += 1;
                }
                let mut end = j;
                let mut continues = false;
                if end > line_start && data[end - 1] == b'\\' {
                    // Strip the backslash; mark continuation.
                    end -= 1;
                    continues = true;
                }

                // On continuation, skip leading whitespace of this physical
                // line to match FreeBSD's behavior (indented continuation).
                let mut seg_start = line_start;
                if line_len > 0 {
                    while seg_start < end
                        && (data[seg_start] == b' ' || data[seg_start] == b'\t')
                    {
                        seg_start += 1;
                    }
                }

                // Append segment to line_buf.
                let seg_len = end - seg_start;
                if line_len + seg_len > line_buf.len() {
                    // Overflow: truncate and stop continuation.
                    let avail = line_buf.len() - line_len;
                    for k in 0..avail {
                        line_buf[line_len + k] = data[seg_start + k];
                    }
                    line_len += avail;
                    // Advance past the rest of this physical line.
                    i = if j < data.len() { j + 1 } else { data.len() };
                    break;
                }
                for k in 0..seg_len {
                    line_buf[line_len + k] = data[seg_start + k];
                }
                line_len += seg_len;

                // Advance past the newline (if any).
                i = if j < data.len() { j + 1 } else { data.len() };

                if !continues {
                    break;
                }
                if i >= data.len() {
                    break;
                }
            }

            // Trim trailing whitespace (but keep ':').
            while line_len > 0
                && (line_buf[line_len - 1] == b' ' || line_buf[line_len - 1] == b'\t')
            {
                line_len -= 1;
            }

            if line_len == 0 {
                continue;
            }
            if line_buf[0] == b'#' {
                continue;
            }

            // Must contain at least one ':'.
            let mut colon_at = None;
            for k in 0..line_len {
                if line_buf[k] == b':' {
                    colon_at = Some(k);
                    break;
                }
            }
            let colon = match colon_at {
                Some(c) => c,
                None => continue,
            };

            // Extract the primary name (first segment before `|` or `:`).
            let mut name_end = colon;
            for k in 0..colon {
                if line_buf[k] == b'|' {
                    name_end = k;
                    break;
                }
            }
            if name_end == 0 {
                continue;
            }

            // Store this record.
            let idx = *(&raw const LOGIN_CLASS_COUNT);
            if idx >= MAX_LOGIN_CLASSES {
                return;
            }
            let classes = &raw mut LOGIN_CLASSES;
            let e = &mut (*classes)[idx];
            e.active = true;

            // Copy primary name (NUL-terminated).
            let name_len = if name_end < MAX_CLASS_NAME - 1 {
                name_end
            } else {
                MAX_CLASS_NAME - 1
            };
            for k in 0..name_len {
                e.name[k] = line_buf[k];
            }
            e.name[name_len] = 0;

            // Copy the full record (name|aliases:caps...), NUL-terminated.
            let rec_len = if line_len < MAX_CAP_RECORD - 1 {
                line_len
            } else {
                MAX_CAP_RECORD - 1
            };
            for k in 0..rec_len {
                e.record[k] = line_buf[k];
            }
            e.record[rec_len] = 0;
            e.record_len = rec_len;

            // Set up the LoginCap with pointers into this entry.
            e.lc.lc_class = e.name.as_ptr();
            e.lc.lc_cap = e.record.as_ptr();
            e.lc.lc_style = ptr::null();

            *(&raw mut LOGIN_CLASS_COUNT) = idx + 1;
        }
        let _ = line_len; // silence unused warning if last iteration skipped
    }
}

/// Second pass: resolve `tc=name` references by splicing the referenced
/// record's capability portion in place of the `tc=...` field. Matches
/// FreeBSD's getent() behavior: inserted content is scanned for further
/// `tc=` references (bounded by MAX_TC_DEPTH).
///
/// First-match precedence is preserved because the original record's caps
/// appear textually before the spliced-in inherited caps.
unsafe fn expand_all_tc() {
    unsafe {
        let count = *(&raw const LOGIN_CLASS_COUNT);
        for i in 0..count {
            expand_tc_for(i, 0);
        }
    }
}

unsafe fn expand_tc_for(idx: usize, depth: usize) {
    unsafe {
        if depth >= MAX_TC_DEPTH {
            return;
        }

        let classes = &raw mut LOGIN_CLASSES;

        // Copy the current record into a local buffer so no reference into
        // LOGIN_CLASSES is held across the recursive call or splice_record
        // below. This keeps borrow aliasing trivially safe.
        let mut local_rec = [0u8; MAX_CAP_RECORD];
        let rec_len = (*classes)[idx].record_len;
        for k in 0..rec_len {
            local_rec[k] = (*classes)[idx].record[k];
        }

        // Find a `tc=` field in the local copy.
        let mut tc_start: Option<usize> = None;
        let mut tc_end: usize = 0;
        let mut pos = 0;
        while pos < rec_len {
            if local_rec[pos] == b':' {
                let field_start = pos + 1;
                if field_start + 3 <= rec_len
                    && local_rec[field_start] == b't'
                    && local_rec[field_start + 1] == b'c'
                    && local_rec[field_start + 2] == b'='
                {
                    let val_start = field_start + 3;
                    let mut val_end = val_start;
                    while val_end < rec_len && local_rec[val_end] != b':' {
                        val_end += 1;
                    }
                    tc_start = Some(pos);
                    tc_end = val_end;
                    break;
                }
                pos = field_start;
            } else {
                pos += 1;
            }
        }

        let (ts, te) = match tc_start {
            Some(s) => (s, tc_end),
            None => return,
        };

        // Extract the tc= target name into a local buffer.
        let val_start = ts + 1 + 3; // past ":tc="
        let mut tgt_name = [0u8; MAX_CLASS_NAME];
        let name_len = te - val_start;
        let copy_len = if name_len < MAX_CLASS_NAME - 1 {
            name_len
        } else {
            MAX_CLASS_NAME - 1
        };
        for k in 0..copy_len {
            tgt_name[k] = local_rec[val_start + k];
        }
        tgt_name[copy_len] = 0;

        // Find target class by name (alias-aware: the referenced class may
        // be declared via any of its `|`-separated aliases).
        let total = *(&raw const LOGIN_CLASS_COUNT);
        let mut tgt_idx: Option<usize> = None;
        for i in 0..total {
            if i == idx {
                continue;
            }
            if !(*classes)[i].active {
                continue;
            }
            // Extract the name field length only — we only need to look at
            // the part of the record before the first `:`. Copy via raw
            // pointer to avoid autoref through the raw LOGIN_CLASSES pointer.
            let rec_len_i = (*classes)[i].record_len;
            let rec_ptr = (&raw const (*classes)[i].record) as *const u8;
            let record = core::slice::from_raw_parts(rec_ptr, rec_len_i);
            if class_name_matches(tgt_name.as_ptr(), record) {
                tgt_idx = Some(i);
                break;
            }
        }
        let ti = match tgt_idx {
            Some(i) => i,
            None => {
                // Target not found: drop the tc= field to avoid rescanning.
                splice_record(idx, ts, te, &[]);
                return;
            }
        };

        // Resolve the target's own tc= chain first.
        expand_tc_for(ti, depth + 1);

        // Copy the target's capability portion (everything from its first
        // `:` onward) into a local scratch buffer. Using a copy avoids any
        // aliasing with the splice destination which lives in the same array.
        let mut scratch = [0u8; MAX_CAP_RECORD];
        let mut slen = 0usize;
        {
            let tgt_rec_len = (*classes)[ti].record_len;
            let mut start = 0usize;
            while start < tgt_rec_len && (*classes)[ti].record[start] != b':' {
                start += 1;
            }
            while start < tgt_rec_len && slen < scratch.len() {
                scratch[slen] = (*classes)[ti].record[start];
                slen += 1;
                start += 1;
            }
        }

        splice_record(idx, ts, te, &scratch[..slen]);

        // After splicing, the inherited content may contain further tc=
        // references. Walk them until none remain or depth is exhausted.
        expand_tc_for(idx, depth + 1);
    }
}

/// Replace `record[start..end]` with `replacement`, truncating if the result
/// would exceed `MAX_CAP_RECORD`. Maintains NUL termination.
unsafe fn splice_record(idx: usize, start: usize, end: usize, replacement: &[u8]) {
    unsafe {
        let classes = &raw mut LOGIN_CLASSES;
        let e = &mut (*classes)[idx];
        let tail_start = end;
        let tail_len = e.record_len - tail_start;

        let new_len = start + replacement.len() + tail_len;
        if new_len >= MAX_CAP_RECORD {
            // Truncate: keep as much as fits, preserving NUL.
            let max_rep = if MAX_CAP_RECORD - 1 - start >= tail_len {
                MAX_CAP_RECORD - 1 - start - tail_len
            } else {
                0
            };
            let rep_len = if replacement.len() < max_rep {
                replacement.len()
            } else {
                max_rep
            };

            // Save tail to scratch (reverse order moves would need care).
            let mut tail = [0u8; MAX_CAP_RECORD];
            for k in 0..tail_len {
                tail[k] = e.record[tail_start + k];
            }
            for k in 0..rep_len {
                e.record[start + k] = replacement[k];
            }
            for k in 0..tail_len {
                e.record[start + rep_len + k] = tail[k];
            }
            e.record_len = start + rep_len + tail_len;
            e.record[e.record_len] = 0;
            return;
        }

        // Save tail to scratch.
        let mut tail = [0u8; MAX_CAP_RECORD];
        for k in 0..tail_len {
            tail[k] = e.record[tail_start + k];
        }

        // Write replacement.
        for k in 0..replacement.len() {
            e.record[start + k] = replacement[k];
        }
        // Write tail.
        for k in 0..tail_len {
            e.record[start + replacement.len() + k] = tail[k];
        }
        e.record_len = new_len;
        e.record[new_len] = 0;
    }
}

// ===========================================================================
// cgetcap / cgetstr — ported from FreeBSD lib/libc/gen/getcap.c
// ===========================================================================

/// Find a capability in a NUL-terminated getcap record.
///
/// Mirrors FreeBSD's `cgetcap()`. `type_char` is one of:
/// - `b':'` — boolean (presence check)
/// - `b'='` — string value
/// - `b'#'` — numeric value
///
/// Returns the byte offset into `record` just past the type delimiter
/// (or past the name for type `:`), or `None` if the capability is not
/// present or is negated with `@`.
///
/// First-match precedence: the scan runs left-to-right from the first `:`,
/// so the first occurrence wins. This is load-bearing for `tc=` inheritance
/// because base caps appear before inherited ones.
unsafe fn cgetcap(record: &[u8], cap: &[u8], type_char: u8) -> Option<usize> {
    // Skip past the name field to the first `:`.
    let mut bp = 0usize;
    loop {
        if bp >= record.len() || record[bp] == 0 {
            return None;
        }
        let c = record[bp];
        bp += 1;
        if c == b':' {
            break;
        }
    }

    // Scan capabilities.
    loop {
        // Skip any leading whitespace inside a field.
        while bp < record.len() && record[bp] != 0
            && (record[bp] == b' ' || record[bp] == b'\t')
        {
            bp += 1;
        }
        if bp >= record.len() || record[bp] == 0 {
            return None;
        }

        // Match capability name.
        if record.len() - bp < cap.len() {
            // Skip to next field.
            while bp < record.len() && record[bp] != 0 && record[bp] != b':' {
                bp += 1;
            }
            if bp < record.len() && record[bp] == b':' {
                bp += 1;
                continue;
            }
            return None;
        }

        let mut matched = true;
        for k in 0..cap.len() {
            if record[bp + k] != cap[k] {
                matched = false;
                break;
            }
        }

        if matched {
            let after = bp + cap.len();
            if after < record.len() {
                let t = record[after];
                // `@` negation takes precedence — capability is explicitly
                // absent regardless of type.
                if t == b'@' {
                    return None;
                }
                match type_char {
                    b':' => {
                        // Boolean: accept if next char is `:`, `\0`, or end.
                        if t == b':' || t == 0 {
                            return Some(after);
                        }
                    }
                    b'=' => {
                        if t == b'=' {
                            // Check for `=@` (FreeBSD treats this as negated).
                            if after + 1 < record.len() && record[after + 1] == b'@' {
                                return None;
                            }
                            return Some(after + 1);
                        }
                    }
                    b'#' => {
                        if t == b'#' {
                            if after + 1 < record.len() && record[after + 1] == b'@' {
                                return None;
                            }
                            return Some(after + 1);
                        }
                    }
                    _ => {}
                }
            } else if type_char == b':' {
                return Some(after);
            }
        }

        // Move to next field.
        while bp < record.len() && record[bp] != 0 && record[bp] != b':' {
            bp += 1;
        }
        if bp < record.len() && record[bp] == b':' {
            bp += 1;
            continue;
        }
        return None;
    }
}

/// Extract a string capability value, decoding escape sequences per
/// FreeBSD getcap.c:782-896.
///
/// Writes the NUL-terminated result to `out`. Returns the string length
/// (excluding NUL) on success, `-1` if the capability is absent, or `-2`
/// if `out` is too small.
unsafe fn cgetstr(record: &[u8], cap: &[u8], out: &mut [u8]) -> i32 {
    if out.is_empty() {
        return -2;
    }
    let start = match cgetcap(record, cap, b'=') {
        Some(s) => s,
        None => return -1,
    };

    let mut sp = start;
    let mut mp = 0usize;
    let max_out = out.len() - 1; // reserve space for NUL

    while sp < record.len() && record[sp] != 0 && record[sp] != b':' {
        let c = record[sp];
        sp += 1;

        if c == b'^' {
            // Control character escape: ^X → X & 0x1F, ^? → 0x7F.
            if sp >= record.len() || record[sp] == 0 || record[sp] == b':' {
                break;
            }
            let nc = record[sp];
            sp += 1;
            let val = if nc == b'?' { 0x7F } else { nc & 0x1F };
            if mp >= max_out {
                out[out.len() - 1] = 0;
                return -2;
            }
            out[mp] = val;
            mp += 1;
            continue;
        }

        if c == b'\\' {
            if sp >= record.len() || record[sp] == 0 {
                break;
            }
            let nc = record[sp];
            sp += 1;

            let decoded = match nc {
                b'b' | b'B' => 0x08u8,
                b't' | b'T' => b'\t',
                b'n' | b'N' => b'\n',
                b'f' | b'F' => 0x0Cu8,
                b'r' | b'R' => b'\r',
                b'e' | b'E' => 0x1Bu8,
                b'c' | b'C' => b':',
                b'0'..=b'7' => {
                    // Octal escape: up to 3 digits including this one.
                    let mut val: u8 = nc - b'0';
                    let mut cnt = 1;
                    while cnt < 3
                        && sp < record.len()
                        && record[sp] >= b'0'
                        && record[sp] <= b'7'
                    {
                        val = val.wrapping_mul(8).wrapping_add(record[sp] - b'0');
                        sp += 1;
                        cnt += 1;
                    }
                    val
                }
                _ => nc,
            };
            if mp >= max_out {
                out[out.len() - 1] = 0;
                return -2;
            }
            out[mp] = decoded;
            mp += 1;
            continue;
        }

        if mp >= max_out {
            out[out.len() - 1] = 0;
            return -2;
        }
        out[mp] = c;
        mp += 1;
    }

    out[mp] = 0;
    mp as i32
}

/// Extract a numeric capability (type `#`).
/// Returns the parsed number, or `None` if absent.
unsafe fn cgetnum_raw(record: &[u8], cap: &[u8]) -> Option<i64> {
    let start = cgetcap(record, cap, b'#')?;
    let bytes = &record[start..];
    Some(parse_signed_decimal(bytes))
}

fn parse_signed_decimal(s: &[u8]) -> i64 {
    let mut i = 0usize;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    let neg;
    if i < s.len() && s[i] == b'-' {
        neg = true;
        i += 1;
    } else if i < s.len() && s[i] == b'+' {
        neg = false;
        i += 1;
    } else {
        neg = false;
    }
    let mut val: i64 = 0;
    while i < s.len() && s[i] >= b'0' && s[i] <= b'9' {
        val = val.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        -val
    } else {
        val
    }
}

// ===========================================================================
// Time / size parsing
// ===========================================================================

/// Check whether a C string equals one of FreeBSD's infinity aliases:
/// "infinity", "inf", "unlimited", "unlimit", "-1".
unsafe fn is_infinity_str(buf: &[u8]) -> bool {
    let aliases: [&[u8]; 5] = [
        b"infinity",
        b"inf",
        b"unlimited",
        b"unlimit",
        b"-1",
    ];
    for alias in aliases.iter() {
        if buf.len() >= alias.len() {
            let mut ok = true;
            for k in 0..alias.len() {
                if to_lower(buf[k]) != alias[k] {
                    ok = false;
                    break;
                }
            }
            if ok {
                // Must be terminated by NUL.
                if alias.len() == buf.len() || buf[alias.len()] == 0 {
                    return true;
                }
            }
        }
    }
    false
}

/// Parse a time value per FreeBSD login_cap.c:634-711.
/// Accepts concatenated units: "10h3m2s", "30d", "1w", "-1" (infinity).
fn parse_time_value(buf: &[u8]) -> i64 {
    let mut total: i64 = 0;
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < buf.len() && buf[i] != 0 && (buf[i] == b' ' || buf[i] == b'\t') {
        i += 1;
    }

    while i < buf.len() && buf[i] != 0 {
        // Parse numeric component.
        let mut neg = false;
        if buf[i] == b'-' {
            neg = true;
            i += 1;
        }
        let mut n: i64 = 0;
        let mut any = false;
        while i < buf.len() && buf[i] != 0 && buf[i] >= b'0' && buf[i] <= b'9' {
            n = n.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
            i += 1;
            any = true;
        }
        if !any {
            break;
        }
        if neg {
            n = -n;
        }

        // Parse optional unit suffix.
        let mult: i64 = if i < buf.len() && buf[i] != 0 {
            let u = to_lower(buf[i]);
            let m = match u {
                b'y' => 31_536_000,
                b'w' => 604_800,
                b'd' => 86_400,
                b'h' => 3_600,
                b'm' => 60,
                b's' => 1,
                _ => 1,
            };
            if matches!(u, b'y' | b'w' | b'd' | b'h' | b'm' | b's') {
                i += 1;
            }
            m
        } else {
            1
        };

        total = total.wrapping_add(n.wrapping_mul(mult));
    }

    total
}

/// Parse a size value per FreeBSD login_cap.c:820-881 (b/B/k/K/m/M/g/G/t/T).
fn parse_size_value(buf: &[u8]) -> i64 {
    let mut total: i64 = 0;
    let mut i = 0usize;

    while i < buf.len() && buf[i] != 0 {
        let mut n: i64 = 0;
        let mut any = false;
        while i < buf.len() && buf[i] != 0 && buf[i] >= b'0' && buf[i] <= b'9' {
            n = n.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
            i += 1;
            any = true;
        }
        if !any {
            break;
        }

        let mult: i64 = if i < buf.len() && buf[i] != 0 {
            let u = to_lower(buf[i]);
            let m: i64 = match u {
                b'b' => 512,
                b'k' => 1024,
                b'm' => 1024 * 1024,
                b'g' => 1024 * 1024 * 1024,
                b't' => 1024i64 * 1024 * 1024 * 1024,
                _ => 1,
            };
            if matches!(u, b'b' | b'k' | b'm' | b'g' | b't') {
                i += 1;
            }
            m
        } else {
            1
        };

        total = total.wrapping_add(n.wrapping_mul(mult));
    }

    total
}

// ===========================================================================
// Public C API — class lookup
// ===========================================================================

unsafe fn ensure_loaded() {
    unsafe {
        if !*(&raw const LOGIN_CONF_LOADED) {
            load_login_conf();
        }
    }
}

unsafe fn find_class_by_name(name: *const u8) -> *mut LoginCap {
    unsafe {
        if name.is_null() {
            return &raw mut EMPTY_LC;
        }
        ensure_loaded();
        let classes = &raw mut LOGIN_CLASSES;
        let count = *(&raw const LOGIN_CLASS_COUNT);
        for i in 0..count {
            let e = &mut (*classes)[i];
            if !e.active {
                continue;
            }
            // Match the primary name OR any `|`-separated alias in the
            // record's name field (FreeBSD login.conf alias syntax).
            let record = &e.record[..e.record_len];
            if class_name_matches(name, record) {
                return &raw mut e.lc;
            }
        }
        // Class not found — return the empty LoginCap so all subsequent
        // getcap calls return defaults.
        &raw mut EMPTY_LC
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getclass(cls: *const u8) -> *mut LoginCap {
    unsafe {
        if cls.is_null() || *cls == 0 {
            return find_class_by_name(b"default\0".as_ptr());
        }
        find_class_by_name(cls)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getclassbyname(
    cls: *const u8,
    pwd: *const Passwd,
) -> *mut LoginCap {
    unsafe {
        if !cls.is_null() && *cls != 0 {
            return find_class_by_name(cls);
        }
        if !pwd.is_null() {
            let class_ptr = (*pwd).pw_class;
            if !class_ptr.is_null() && *class_ptr != 0 {
                return find_class_by_name(class_ptr);
            }
            if (*pwd).pw_uid == 0 {
                return find_class_by_name(b"root\0".as_ptr());
            }
        }
        find_class_by_name(b"default\0".as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getpwclass(pwd: *const Passwd) -> *mut LoginCap {
    unsafe {
        if pwd.is_null() {
            return find_class_by_name(b"default\0".as_ptr());
        }
        let class_ptr = (*pwd).pw_class;
        if !class_ptr.is_null() && *class_ptr != 0 {
            return find_class_by_name(class_ptr);
        }
        if (*pwd).pw_uid == 0 {
            return find_class_by_name(b"root\0".as_ptr());
        }
        find_class_by_name(b"default\0".as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getuserclass(pwd: *const Passwd) -> *mut LoginCap {
    // FreeBSD login_getuserclass reads ~/.login_conf as well, which we don't
    // support — fall through to system login.conf via login_getpwclass.
    unsafe { login_getpwclass(pwd) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_close(_lc: *mut LoginCap) {
    // No-op: LoginCap entries live in the static LOGIN_CLASSES pool.
}

// ===========================================================================
// Public C API — capability getters
// ===========================================================================

/// Convert a C string capability name into a byte slice up to the NUL.
unsafe fn cap_slice<'a>(cap: *const u8) -> &'a [u8] {
    unsafe {
        let len = cstr_len(cap);
        core::slice::from_raw_parts(cap, len)
    }
}

unsafe fn record_slice(lc: *mut LoginCap) -> Option<&'static [u8]> {
    unsafe {
        if lc.is_null() || (*lc).lc_cap.is_null() {
            return None;
        }
        let start = (*lc).lc_cap;
        let len = cstr_len(start);
        Some(core::slice::from_raw_parts(start, len))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getcapstr(
    lc: *mut LoginCap,
    cap: *const u8,
    def: *const u8,
    err: *const u8,
) -> *const u8 {
    unsafe {
        if cap.is_null() {
            return def;
        }
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return def,
        };
        let cap_bytes = cap_slice(cap);
        let buf = &raw mut CAPSTR_BUF;
        let result = cgetstr(rec, cap_bytes, &mut *buf);
        if result == -1 {
            def
        } else if result < 0 {
            err
        } else {
            (*buf).as_ptr()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getcapbool(
    lc: *mut LoginCap,
    cap: *const u8,
    def: i32,
) -> i32 {
    // FreeBSD semantics (lib/libutil/login_cap.c):
    //   if (lc == NULL || lc->lc_cap == NULL) return def;
    //   return (cgetcap(lc->lc_cap, cap, ':') != NULL);
    //
    // `def` is used only when no record is loaded at all. When a record
    // exists, absent or `@`-negated capabilities return 0 (not def).
    unsafe {
        if cap.is_null() {
            return def;
        }
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return def,
        };
        let cap_bytes = cap_slice(cap);
        if cgetcap(rec, cap_bytes, b':').is_some() {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getcaptime(
    lc: *mut LoginCap,
    cap: *const u8,
    def: i64,
    err: i64,
) -> i64 {
    unsafe {
        if cap.is_null() {
            return def;
        }
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return def,
        };
        let cap_bytes = cap_slice(cap);
        let buf = &raw mut CAPSTR_BUF;
        let result = cgetstr(rec, cap_bytes, &mut *buf);
        if result == -1 {
            return def;
        }
        if result < 0 {
            return err;
        }
        // Build the value slice via from_raw_parts to avoid autoref on a
        // dereferenced raw pointer (denied by dangerous_implicit_autorefs).
        let value = core::slice::from_raw_parts(buf as *const u8, result as usize);
        if is_infinity_str(value) {
            return i64::MAX;
        }
        parse_time_value(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getcapnum(
    lc: *mut LoginCap,
    cap: *const u8,
    def: i64,
    err: i64,
) -> i64 {
    unsafe {
        if cap.is_null() {
            return def;
        }
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return def,
        };
        let cap_bytes = cap_slice(cap);

        // Try numeric form first (cap#value).
        if let Some(n) = cgetnum_raw(rec, cap_bytes) {
            return n;
        }

        // Fall back to string form (cap=value).
        let buf = &raw mut CAPSTR_BUF;
        let result = cgetstr(rec, cap_bytes, &mut *buf);
        if result == -1 {
            return def;
        }
        if result < 0 {
            return err;
        }
        // Build the value slice via from_raw_parts to avoid autoref on a
        // dereferenced raw pointer (denied by dangerous_implicit_autorefs).
        let value = core::slice::from_raw_parts(buf as *const u8, result as usize);
        if is_infinity_str(value) {
            return i64::MAX;
        }
        parse_signed_decimal(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getcapsize(
    lc: *mut LoginCap,
    cap: *const u8,
    def: i64,
    err: i64,
) -> i64 {
    unsafe {
        if cap.is_null() {
            return def;
        }
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return def,
        };
        let cap_bytes = cap_slice(cap);
        let buf = &raw mut CAPSTR_BUF;
        let result = cgetstr(rec, cap_bytes, &mut *buf);
        if result == -1 {
            return def;
        }
        if result < 0 {
            return err;
        }
        // Build the value slice via from_raw_parts to avoid autoref on a
        // dereferenced raw pointer (denied by dangerous_implicit_autorefs).
        let value = core::slice::from_raw_parts(buf as *const u8, result as usize);
        if is_infinity_str(value) {
            return i64::MAX;
        }
        parse_size_value(value)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_getpath(
    lc: *mut LoginCap,
    cap: *const u8,
    def: *const u8,
) -> *const u8 {
    unsafe { login_getcapstr(lc, cap, def, def) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn login_setcryptfmt(
    lc: *mut LoginCap,
    def: *const u8,
    err: *const u8,
) -> *const u8 {
    unsafe { login_getcapstr(lc, b"passwd_format\0".as_ptr(), def, err) }
}

// ===========================================================================
// Allow/deny list checking for auth_ttyok / auth_hostok
// ===========================================================================

/// Check whether `subject` (NUL-terminated) appears in a comma-separated
/// capability list value. Matching is case-insensitive. Empty list = no match.
unsafe fn list_contains(
    lc: *mut LoginCap,
    cap_name: &[u8],
    subject: *const u8,
) -> (bool, bool) {
    // Returns (list_exists, match_found).
    unsafe {
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return (false, false),
        };
        let mut scratch = [0u8; MAX_CAP_VALUE];
        let result = cgetstr(rec, cap_name, &mut scratch);
        if result < 0 {
            return (false, false);
        }
        let list = &scratch[..result as usize];
        if list.is_empty() {
            return (true, false);
        }

        // Split on comma, trim whitespace, compare against subject.
        let subject_len = cstr_len(subject);
        let subject_slice = core::slice::from_raw_parts(subject, subject_len);

        let mut i = 0usize;
        while i < list.len() {
            // Skip leading whitespace.
            while i < list.len() && (list[i] == b' ' || list[i] == b'\t') {
                i += 1;
            }
            let start = i;
            while i < list.len() && list[i] != b',' {
                i += 1;
            }
            let mut end = i;
            while end > start && (list[end - 1] == b' ' || list[end - 1] == b'\t') {
                end -= 1;
            }

            if end > start {
                let entry = &list[start..end];
                if entry.len() == subject_slice.len() {
                    let mut eq = true;
                    for k in 0..entry.len() {
                        if to_lower(entry[k]) != to_lower(subject_slice[k]) {
                            eq = false;
                            break;
                        }
                    }
                    if eq {
                        return (true, true);
                    }
                }
            }

            if i < list.len() {
                i += 1; // skip comma
            }
        }

        (true, false)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn auth_ttyok(lc: *mut LoginCap, tty: *const u8) -> i32 {
    unsafe {
        if lc.is_null() || tty.is_null() || *tty == 0 {
            return 1;
        }
        // ttys.allow
        let (allow_exists, allow_match) = list_contains(lc, b"ttys.allow", tty);
        if allow_exists && !allow_match {
            return 0;
        }
        // ttys.deny
        let (deny_exists, deny_match) = list_contains(lc, b"ttys.deny", tty);
        if deny_exists && deny_match {
            return 0;
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn auth_hostok(lc: *mut LoginCap, host: *const u8) -> i32 {
    unsafe {
        if lc.is_null() || host.is_null() || *host == 0 {
            return 1;
        }
        let (allow_exists, allow_match) = list_contains(lc, b"host.allow", host);
        if allow_exists && !allow_match {
            return 0;
        }
        let (deny_exists, deny_match) = list_contains(lc, b"host.deny", host);
        if deny_exists && deny_match {
            return 0;
        }
        1
    }
}

// ===========================================================================
// Time-of-day access control for auth_timeok()
// ===========================================================================

// Day-of-week bits matching FreeBSD login_cap.h LTM_* flags.
const LTM_SUN: u8 = 0x01;
const LTM_MON: u8 = 0x02;
const LTM_TUE: u8 = 0x04;
const LTM_WED: u8 = 0x08;
const LTM_THU: u8 = 0x10;
const LTM_FRI: u8 = 0x20;
const LTM_SAT: u8 = 0x40;
const LTM_ANY: u8 = 0x7F;
const LTM_WK: u8 = LTM_MON | LTM_TUE | LTM_WED | LTM_THU | LTM_FRI;
const LTM_WD: u8 = LTM_SUN | LTM_SAT;

/// A parsed login time range, matching FreeBSD's `login_time_t`.
/// Example: `"MoTuWeThFr0800-1700"` → `{ dow: WK, start: 480, end: 1020 }`
/// where start/end are minutes since midnight.
#[derive(Clone, Copy)]
struct LoginTime {
    dow: u8,
    start_min: u16,
    end_min: u16,
}

/// Parse a single DOW token (2 characters or `Any`/`Wk`/`Wd`). Consumes
/// characters from `bytes[i..]` and returns `(mask, new_i)`. Returns
/// `(0, i)` if no valid token is found.
fn parse_dow_token(bytes: &[u8], i: usize) -> (u8, usize) {
    if i + 2 > bytes.len() {
        return (0, i);
    }
    let a = to_lower(bytes[i]);
    let b = to_lower(bytes[i + 1]);
    let mask = match (a, b) {
        (b's', b'u') => LTM_SUN,
        (b'm', b'o') => LTM_MON,
        (b't', b'u') => LTM_TUE,
        (b'w', b'e') => LTM_WED,
        (b't', b'h') => LTM_THU,
        (b'f', b'r') => LTM_FRI,
        (b's', b'a') => LTM_SAT,
        (b'a', b'n') => {
            // "Any" — 3 characters
            if i + 3 <= bytes.len() && to_lower(bytes[i + 2]) == b'y' {
                return (LTM_ANY, i + 3);
            }
            return (0, i);
        }
        (b'w', b'k') => LTM_WK,
        (b'w', b'd') => LTM_WD,
        _ => return (0, i),
    };
    (mask, i + 2)
}

/// Parse a four-digit `HHMM` time into minutes since midnight.
fn parse_hhmm(bytes: &[u8], i: usize) -> Option<(u16, usize)> {
    if i + 4 > bytes.len() {
        return None;
    }
    for k in 0..4 {
        if bytes[i + k] < b'0' || bytes[i + k] > b'9' {
            return None;
        }
    }
    let h = (bytes[i] - b'0') as u16 * 10 + (bytes[i + 1] - b'0') as u16;
    let m = (bytes[i + 2] - b'0') as u16 * 10 + (bytes[i + 3] - b'0') as u16;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some((h * 60 + m, i + 4))
}

/// Parse a single `login_time_t` spec: `<DOW><DOW>...HHMM-HHMM`.
/// Example: `MoTuWe0900-1700` → dow mask, start 540, end 1020.
fn parse_login_time(bytes: &[u8]) -> Option<LoginTime> {
    let mut i = 0usize;
    let mut dow: u8 = 0;

    // Read one or more DOW tokens.
    loop {
        let (mask, new_i) = parse_dow_token(bytes, i);
        if mask == 0 {
            break;
        }
        dow |= mask;
        i = new_i;
    }
    if dow == 0 {
        return None;
    }

    // Parse HHMM-HHMM.
    let (start_min, i) = parse_hhmm(bytes, i)?;
    if i >= bytes.len() || bytes[i] != b'-' {
        return None;
    }
    let (end_min, _) = parse_hhmm(bytes, i + 1)?;

    Some(LoginTime {
        dow,
        start_min,
        end_min,
    })
}

/// Break a UTC Unix timestamp into local-time `(dow_mask, minute_of_day)`.
///
/// Delegates to basaltc's `localtime_r()`, which honors the `TZ` environment
/// variable and DST rules (POSIX `EST5EDT,M3.2.0,M11.1.0` syntax). This
/// matches FreeBSD's `login_ok.c:220` contract where the caller passes UTC
/// epoch and the library performs the timezone conversion internally.
unsafe fn broken_down_time(now: i64) -> (u8, u16) {
    unsafe {
        let timep: crate::time::TimeT = now;
        let mut tm: crate::time::Tm = core::mem::zeroed();
        let result = crate::time::localtime_r(&timep as *const _, &mut tm as *mut _);
        if result.is_null() {
            // Fall back to UTC on conversion failure.
            let secs = if now >= 0 { now as u64 } else { 0u64 };
            let days = secs / 86_400;
            let day_of_week = ((days + 4) % 7) as u8;
            let minute = ((secs % 86_400) / 60) as u16;
            return (1u8 << day_of_week, minute);
        }
        // tm_wday is 0..=6 where 0 = Sunday, matching LTM_SUN..LTM_SAT bit order.
        let dow_bit = ((*result).tm_wday & 0x7) as u8;
        let dow_mask = 1u8 << dow_bit;
        let minute = ((*result).tm_hour * 60 + (*result).tm_min) as u16;
        (dow_mask, minute)
    }
}

/// Shift a single-day bitmap back by one day, wrapping Sunday → Saturday.
/// Used for cross-midnight matching: a time like `Fr2300-0200` should
/// also match Saturday 00:00–01:59, so we test whether "yesterday" was in
/// the spec's day set.
fn prev_day_bit(dow_mask: u8) -> u8 {
    // LTM_SUN is bit 0, LTM_SAT is bit 6. 7-bit right rotate.
    ((dow_mask >> 1) | ((dow_mask & LTM_SUN) << 6)) & LTM_ANY
}

/// Check whether a single time spec contains `(dow, minute)`.
///
/// For same-day ranges (start <= end), the current day must be in `spec.dow`
/// and minute in `[start, end)`.
///
/// For cross-midnight ranges (start > end), two cases match:
/// 1. Today ∈ spec.dow and minute ≥ start (the pre-midnight portion)
/// 2. Yesterday ∈ spec.dow and minute < end (the post-midnight carry-over)
fn time_in_spec(spec: &LoginTime, dow_mask: u8, minute: u16) -> bool {
    if spec.start_min <= spec.end_min {
        // Same-day window.
        (spec.dow & dow_mask) != 0
            && minute >= spec.start_min
            && minute < spec.end_min
    } else {
        // Cross-midnight window.
        if (spec.dow & dow_mask) != 0 && minute >= spec.start_min {
            return true;
        }
        let prev = prev_day_bit(dow_mask);
        if (spec.dow & prev) != 0 && minute < spec.end_min {
            return true;
        }
        false
    }
}

/// Scan a comma-separated list of time specs and return whether any of
/// them matches `(dow_mask, minute)`. Returns `(list_present, matched)`.
unsafe fn times_list_match(
    lc: *mut LoginCap,
    cap_name: &[u8],
    dow_mask: u8,
    minute: u16,
) -> (bool, bool) {
    unsafe {
        let rec = match record_slice(lc) {
            Some(r) => r,
            None => return (false, false),
        };
        let mut scratch = [0u8; MAX_CAP_VALUE];
        let len = cgetstr(rec, cap_name, &mut scratch);
        if len < 0 {
            return (false, false);
        }
        let list = &scratch[..len as usize];
        if list.is_empty() {
            return (true, false);
        }

        let mut i = 0usize;
        while i < list.len() {
            while i < list.len() && (list[i] == b' ' || list[i] == b'\t') {
                i += 1;
            }
            let start = i;
            while i < list.len() && list[i] != b',' {
                i += 1;
            }
            let mut end = i;
            while end > start && (list[end - 1] == b' ' || list[end - 1] == b'\t') {
                end -= 1;
            }
            if end > start {
                if let Some(spec) = parse_login_time(&list[start..end]) {
                    if time_in_spec(&spec, dow_mask, minute) {
                        return (true, true);
                    }
                }
            }
            if i < list.len() {
                i += 1;
            }
        }
        (true, false)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn auth_timeok(lc: *mut LoginCap, now: i64) -> i32 {
    // FreeBSD login_ok.c:217-248 semantics:
    //   1. If lc is NULL or now <= 0 → allow.
    //   2. If times.allow is set and current time NOT in any entry → deny.
    //   3. If times.deny is set and current time IS in any entry → deny.
    //   4. Otherwise allow.
    //
    // `now` is a UTC Unix timestamp (as returned by `time(NULL)`). We run
    // it through basaltc's `localtime_r()` before comparing against the
    // time ranges, matching FreeBSD's internal behavior. The timezone is
    // taken from the `TZ` environment variable via basaltc's tzset().
    unsafe {
        if lc.is_null() || now <= 0 {
            return 1;
        }

        let (dow_mask, minute) = broken_down_time(now);

        let (allow_exists, allow_match) =
            times_list_match(lc, b"times.allow", dow_mask, minute);
        if allow_exists && !allow_match {
            return 0;
        }

        let (deny_exists, deny_match) =
            times_list_match(lc, b"times.deny", dow_mask, minute);
        if deny_exists && deny_match {
            return 0;
        }

        1
    }
}

// ===========================================================================
// setusercontext — apply login class to the current process
// ===========================================================================

/// FreeBSD flag values for setusercontext(). Must match `login_cap.h` exactly.
const LOGIN_SETGROUP: u32 = 0x0001;
const LOGIN_SETLOGIN: u32 = 0x0002;
const LOGIN_SETPATH: u32 = 0x0004;
const LOGIN_SETPRIORITY: u32 = 0x0008;
const LOGIN_SETRESOURCES: u32 = 0x0010;
const LOGIN_SETUMASK: u32 = 0x0020;
const LOGIN_SETUSER: u32 = 0x0040;
const LOGIN_SETENV: u32 = 0x0080;

/// Parse an octal umask string (e.g. "022", "0077"). Invalid → return 0o022.
fn parse_octal_umask(buf: &[u8]) -> u32 {
    let mut i = 0usize;
    if i < buf.len() && buf[i] == b'0' {
        i += 1;
    }
    let mut val: u32 = 0;
    let mut any = false;
    while i < buf.len() && buf[i] >= b'0' && buf[i] <= b'7' {
        val = (val << 3) | (buf[i] - b'0') as u32;
        i += 1;
        any = true;
    }
    if !any {
        0o022
    } else {
        val & 0o777
    }
}

/// Set `PATH` from the login class "path" capability. The value is a
/// space-separated list of directories which must be converted into the
/// colon-separated form expected by the shell.
unsafe fn apply_login_path(lc: *mut LoginCap) {
    unsafe {
        let default_path = b"/usr/bin:/bin:/usr/local/bin\0".as_ptr();
        let path_ptr = login_getcapstr(lc, b"path\0".as_ptr(), default_path, default_path);
        if path_ptr.is_null() {
            return;
        }

        // Convert space-separated to colon-separated in a local buffer.
        let src_len = cstr_len(path_ptr);
        let mut buf = [0u8; 1024];
        let max = buf.len() - 1;
        let mut j = 0usize;
        for i in 0..src_len {
            if j >= max {
                break;
            }
            let c = *path_ptr.add(i);
            if c == b' ' || c == b'\t' {
                // Collapse runs of whitespace into a single `:`.
                if j == 0 || buf[j - 1] == b':' {
                    continue;
                }
                buf[j] = b':';
                j += 1;
            } else {
                buf[j] = c;
                j += 1;
            }
        }
        // Trim trailing colon.
        while j > 0 && buf[j - 1] == b':' {
            j -= 1;
        }
        buf[j] = 0;

        let _ = crate::env::setenv(b"PATH\0".as_ptr(), buf.as_ptr(), 1);
    }
}

/// Apply the "setenv" capability: comma-separated list of `VAR=value` pairs.
/// Example: `setenv=MAIL=/var/mail/$,BLOCKSIZE=K`
unsafe fn apply_login_env(lc: *mut LoginCap, pwd: *const Passwd) {
    unsafe {
        let null = ptr::null();
        let val_ptr = login_getcapstr(lc, b"setenv\0".as_ptr(), null, null);
        if val_ptr.is_null() {
            return;
        }
        let val_len = cstr_len(val_ptr);
        if val_len == 0 {
            return;
        }
        let bytes = core::slice::from_raw_parts(val_ptr, val_len);

        let mut i = 0usize;
        let mut name_buf = [0u8; 128];
        let mut value_buf = [0u8; 512];

        while i < bytes.len() {
            // Skip leading whitespace and commas.
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b',') {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }

            // Read NAME up to '=' or ','.
            let name_start = i;
            while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b',' {
                i += 1;
            }
            let name_end = i;
            if name_end == name_start {
                continue;
            }

            // Extract name.
            let n_len = name_end - name_start;
            let copy_n = if n_len < name_buf.len() - 1 {
                n_len
            } else {
                name_buf.len() - 1
            };
            for k in 0..copy_n {
                name_buf[k] = bytes[name_start + k];
            }
            name_buf[copy_n] = 0;

            // Read value if '=' present.
            let mut v_len = 0usize;
            if i < bytes.len() && bytes[i] == b'=' {
                i += 1;
                while i < bytes.len() && bytes[i] != b',' {
                    let c = bytes[i];
                    i += 1;
                    // FreeBSD substitutes `$` with the user's home.
                    if c == b'$' && !pwd.is_null() {
                        let home = (*pwd).pw_dir;
                        if !home.is_null() {
                            let hlen = cstr_len(home);
                            for k in 0..hlen {
                                if v_len >= value_buf.len() - 1 {
                                    break;
                                }
                                value_buf[v_len] = *home.add(k);
                                v_len += 1;
                            }
                            continue;
                        }
                    }
                    if v_len >= value_buf.len() - 1 {
                        break;
                    }
                    value_buf[v_len] = c;
                    v_len += 1;
                }
            }
            value_buf[v_len] = 0;

            let _ = crate::env::setenv(name_buf.as_ptr(), value_buf.as_ptr(), 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setclassenvironment(
    lc: *mut LoginCap,
    pwd: *const Passwd,
    paths: i32,
) {
    unsafe {
        if paths != 0 {
            apply_login_path(lc);
        }
        apply_login_env(lc, pwd);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setclassresources(_lc: *mut LoginCap) {
    // SaltyOS has no rlimit(2) — resources (cputime, datasize, stacksize,
    // filesize, coredumpsize, memorylocked, memoryuse, vmemoryuse, sbsize,
    // openfiles, maxproc, umtxp, pseudoterminals, swapuse) are not enforced.
    // Intentional no-op.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setusercontext(
    lc: *mut LoginCap,
    pwd: *const Passwd,
    uid: u32,
    flags: u32,
) -> i32 {
    unsafe {
        // If no LoginCap was provided, synthesize one from pwd so callers
        // that pass NULL still get class-aware behavior (matches FreeBSD).
        let lc_eff = if lc.is_null() && !pwd.is_null() {
            login_getpwclass(pwd)
        } else {
            lc
        };

        // LOGIN_SETGROUP: initgroups + setgid
        if (flags & LOGIN_SETGROUP) != 0 && !pwd.is_null() {
            if crate::process::initgroups((*pwd).pw_name, (*pwd).pw_gid) < 0 {
                return -1;
            }
            if crate::process::setgid((*pwd).pw_gid) < 0 {
                return -1;
            }
        }

        // LOGIN_SETLOGIN: setlogin() unsupported on SaltyOS — silently skip.
        // FreeBSD stores the login name in the kernel for getlogin(); basaltc
        // returns "root" from getlogin() via compat/freebsd/bsd_misc.
        let _ = LOGIN_SETLOGIN;

        // LOGIN_SETPRIORITY: SaltyOS uses EDF scheduling, not nice values.
        let _ = LOGIN_SETPRIORITY;

        // LOGIN_SETRESOURCES: no rlimit support.
        if (flags & LOGIN_SETRESOURCES) != 0 {
            setclassresources(lc_eff);
        }

        // LOGIN_SETUMASK: read "umask" capability and apply.
        if (flags & LOGIN_SETUMASK) != 0 {
            let mut scratch = [0u8; 16];
            let rec = record_slice(lc_eff);
            let mask = if let Some(r) = rec {
                let len = cgetstr(r, b"umask", &mut scratch);
                if len > 0 {
                    parse_octal_umask(&scratch[..len as usize])
                } else {
                    0o022
                }
            } else {
                0o022
            };
            let _ = crate::unistd::umask(mask);
        }

        // LOGIN_SETPATH / LOGIN_SETENV: apply class environment.
        if (flags & (LOGIN_SETPATH | LOGIN_SETENV)) != 0 {
            setclassenvironment(
                lc_eff,
                pwd,
                if (flags & LOGIN_SETPATH) != 0 { 1 } else { 0 },
            );
        }

        // LOGIN_SETUSER: setuid.
        if (flags & LOGIN_SETUSER) != 0 {
            if crate::process::setuid(uid) < 0 {
                return -1;
            }
        }

        0
    }
}

/// FreeBSD `_secure_path` — check that a file is owned by root and not
/// writable by group/other. SaltyOS currently does not enforce permission
/// semantics on config files at path lookup time, so this returns 0
/// (secure). Retained for linker compatibility with callers in libutil.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _secure_path(_path: *const u8, _uid: u32, _gid: u32) -> i32 {
    0
}
