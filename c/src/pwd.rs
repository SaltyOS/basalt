//! Password, group, and shadow database — file-based parsing
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Parses `/etc/passwd`, `/etc/group`, and `/etc/shadow` into static caches on
//! first access (lazy loading). Falls back to a single hardcoded root entry if
//! the files cannot be opened (backward compatibility for early boot before VFS
//! creates the files).
//!
//! All returned pointers are to static data — standard POSIX non-reentrant
//! semantics. The `_r` variants copy into caller-supplied buffers.

use core::ptr;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_USERS: usize = 32;
const MAX_GROUPS: usize = 32;
const MAX_SHADOW: usize = 32;
const FILE_BUF_SIZE: usize = 8192;
const MAX_GROUP_MEMBERS: usize = 8;
const MAX_MEMBER_NAME: usize = 32;

// ---------------------------------------------------------------------------
// Internal storage types
// ---------------------------------------------------------------------------

struct PasswdEntry {
    active: bool,
    pw_name: [u8; 64],
    pw_passwd: [u8; 256], // widened to accept real hashes from /etc/master.passwd
    pw_uid: u32,
    pw_gid: u32,
    pw_change: i64,
    pw_class: [u8; 32],
    pw_gecos: [u8; 128],
    pw_dir: [u8; 64],
    pw_shell: [u8; 64],
    pw_expire: i64,
    pw_fields: i32,
}

struct GroupEntry {
    active: bool,
    gr_name: [u8; 64],
    gr_passwd: [u8; 4],
    gr_gid: u32,
    gr_members: [[u8; MAX_MEMBER_NAME]; MAX_GROUP_MEMBERS],
    gr_member_count: usize,
}

struct SpwdEntry {
    active: bool,
    sp_namp: [u8; 64],
    sp_pwdp: [u8; 256],
    sp_lstchg: i64,
    sp_min: i64,
    sp_max: i64,
    sp_warn: i64,
    sp_inact: i64,
    sp_expire: i64,
}

// ---------------------------------------------------------------------------
// C ABI structures (match pwd.h / grp.h / shadow.h)
// ---------------------------------------------------------------------------

/// C ABI `struct passwd` — matches FreeBSD 11-field layout exactly.
///
/// Layout is critical: must match FreeBSD's `<pwd.h>` so that compiled code
/// from the freebsd-utils port (su, pam_unix, pam_nologin, libutil.so) reads
/// the fields from the correct offsets. Total size = 80 bytes on x86_64.
#[repr(C)]
pub struct Passwd {
    pub pw_name: *const u8,   // offset 0
    pub pw_passwd: *const u8, // offset 8
    pub pw_uid: u32,          // offset 16
    pub pw_gid: u32,          // offset 20
    pub pw_change: i64,       // offset 24 (time_t)
    pub pw_class: *const u8,  // offset 32
    pub pw_gecos: *const u8,  // offset 40
    pub pw_dir: *const u8,    // offset 48
    pub pw_shell: *const u8,  // offset 56
    pub pw_expire: i64,       // offset 64 (time_t)
    pub pw_fields: i32,       // offset 72
}

// Compile-time layout guard: ABI mismatch would be catastrophic.
const _: () = {
    assert!(core::mem::size_of::<Passwd>() == 80);
    assert!(core::mem::offset_of!(Passwd, pw_change) == 24);
    assert!(core::mem::offset_of!(Passwd, pw_class) == 32);
    assert!(core::mem::offset_of!(Passwd, pw_gecos) == 40);
    assert!(core::mem::offset_of!(Passwd, pw_dir) == 48);
    assert!(core::mem::offset_of!(Passwd, pw_shell) == 56);
    assert!(core::mem::offset_of!(Passwd, pw_expire) == 64);
    assert!(core::mem::offset_of!(Passwd, pw_fields) == 72);
};

// Field presence bitmask values matching FreeBSD's _PWF_* macros in <pwd.h>.
pub(crate) const PWF_NAME: i32 = 1 << 0;
pub(crate) const PWF_PASSWD: i32 = 1 << 1;
pub(crate) const PWF_UID: i32 = 1 << 2;
pub(crate) const PWF_GID: i32 = 1 << 3;
pub(crate) const PWF_CHANGE: i32 = 1 << 4;
pub(crate) const PWF_CLASS: i32 = 1 << 5;
pub(crate) const PWF_GECOS: i32 = 1 << 6;
pub(crate) const PWF_DIR: i32 = 1 << 7;
pub(crate) const PWF_SHELL: i32 = 1 << 8;
pub(crate) const PWF_EXPIRE: i32 = 1 << 9;

#[repr(C)]
pub struct Group {
    pub gr_name: *const u8,
    pub gr_passwd: *const u8,
    pub gr_gid: u32,
    pub gr_mem: *const *const u8,
}

#[repr(C)]
pub struct Spwd {
    pub sp_namp: *const u8,
    pub sp_pwdp: *const u8,
    pub sp_lstchg: i64,
    pub sp_min: i64,
    pub sp_max: i64,
    pub sp_warn: i64,
    pub sp_inact: i64,
    pub sp_expire: i64,
    pub sp_flag: u64,
}

// ---------------------------------------------------------------------------
// Static databases
// ---------------------------------------------------------------------------

const ZERO_PASSWD: PasswdEntry = PasswdEntry {
    active: false,
    pw_name: [0; 64],
    pw_passwd: [0; 256],
    pw_uid: 0,
    pw_gid: 0,
    pw_change: 0,
    pw_class: [0; 32],
    pw_gecos: [0; 128],
    pw_dir: [0; 64],
    pw_shell: [0; 64],
    pw_expire: 0,
    pw_fields: 0,
};

const ZERO_GROUP: GroupEntry = GroupEntry {
    active: false,
    gr_name: [0; 64],
    gr_passwd: [0; 4],
    gr_gid: 0,
    gr_members: [[0; MAX_MEMBER_NAME]; MAX_GROUP_MEMBERS],
    gr_member_count: 0,
};

const ZERO_SPWD: SpwdEntry = SpwdEntry {
    active: false,
    sp_namp: [0; 64],
    sp_pwdp: [0; 256],
    sp_lstchg: -1,
    sp_min: -1,
    sp_max: -1,
    sp_warn: -1,
    sp_inact: -1,
    sp_expire: -1,
};

static mut PASSWD_DB: [PasswdEntry; MAX_USERS] = [ZERO_PASSWD; MAX_USERS];
static mut PASSWD_COUNT: usize = 0;
static mut PASSWD_LOADED: bool = false;
static mut PASSWD_ITER: usize = 0;

static mut GROUP_DB: [GroupEntry; MAX_GROUPS] = [ZERO_GROUP; MAX_GROUPS];
static mut GROUP_COUNT: usize = 0;
static mut GROUP_LOADED: bool = false;
static mut GROUP_ITER: usize = 0;

static mut SHADOW_DB: [SpwdEntry; MAX_SHADOW] = [ZERO_SPWD; MAX_SHADOW];
static mut SHADOW_COUNT: usize = 0;
static mut SHADOW_LOADED: bool = false;
static mut SHADOW_ITER: usize = 0;

// Return structs for non-reentrant API
static mut RET_PASSWD: Passwd = Passwd {
    pw_name: ptr::null(),
    pw_passwd: ptr::null(),
    pw_uid: 0,
    pw_gid: 0,
    pw_change: 0,
    pw_class: ptr::null(),
    pw_gecos: ptr::null(),
    pw_dir: ptr::null(),
    pw_shell: ptr::null(),
    pw_expire: 0,
    pw_fields: 0,
};

static mut RET_GROUP: Group = Group {
    gr_name: ptr::null(),
    gr_passwd: ptr::null(),
    gr_gid: 0,
    gr_mem: ptr::null(),
};

// Member pointer array for non-reentrant group API (null-terminated)
static mut RET_GROUP_MEMBERS: [*const u8; MAX_GROUP_MEMBERS + 1] =
    [ptr::null(); MAX_GROUP_MEMBERS + 1];

static mut RET_SPWD: Spwd = Spwd {
    sp_namp: ptr::null(),
    sp_pwdp: ptr::null(),
    sp_lstchg: -1,
    sp_min: -1,
    sp_max: -1,
    sp_warn: -1,
    sp_inact: -1,
    sp_expire: -1,
    sp_flag: 0,
};

// Buffers for user_from_uid / group_from_gid
static mut UID_BUF: [u8; 32] = [0; 32];
static mut GID_BUF: [u8; 32] = [0; 32];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read an entire file into `buf`. Returns bytes read, or 0 on failure.
unsafe fn read_file(path: *const u8, buf: *mut u8, buf_size: usize) -> usize {
    unsafe {
        let fd = trona_posix::posix_open(path, trona_posix::O_RDONLY as i32, 0);
        if fd < 0 {
            return 0;
        }
        let mut total: usize = 0;
        while total < buf_size {
            let remain = buf_size - total;
            let n = trona_posix::posix_read(fd, buf.add(total), remain as u64);
            if n <= 0 {
                break;
            }
            total += n as usize;
        }
        trona_posix::posix_close(fd);
        total
    }
}

/// Copy bytes from `src` into `dst` up to `dst_len - 1`, null-terminate.
/// Returns the number of bytes copied (excluding null).
fn copy_field(dst: &mut [u8], src: &[u8]) -> usize {
    let max = if dst.len() > 0 {
        dst.len() - 1
    } else {
        return 0;
    };
    let len = if src.len() < max { src.len() } else { max };
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
    len
}

/// Parse a decimal integer from a byte slice. Returns (value, success).
fn parse_i64(s: &[u8]) -> (i64, bool) {
    if s.is_empty() {
        return (-1, true); // empty field → -1 (POSIX shadow convention)
    }
    let mut neg = false;
    let mut start = 0;
    if s.len() > 0 && s[0] == b'-' {
        neg = true;
        start = 1;
    }
    let mut val: i64 = 0;
    let mut any = false;
    for &ch in &s[start..] {
        if ch < b'0' || ch > b'9' {
            break;
        }
        any = true;
        val = val.wrapping_mul(10).wrapping_add((ch - b'0') as i64);
    }
    if !any {
        return (-1, true);
    }
    if neg {
        val = -val;
    }
    (val, true)
}

fn parse_u32(s: &[u8]) -> u32 {
    let (v, _) = parse_i64(s);
    v as u32
}

fn field_eq_literal(field: &[u8], literal: &[u8]) -> bool {
    let mut i = 0;
    loop {
        let field_byte = field.get(i).copied().unwrap_or(0);
        let literal_byte = literal.get(i).copied().unwrap_or(0);
        if field_byte != literal_byte {
            return false;
        }
        if field_byte == 0 {
            return true;
        }
        i += 1;
    }
}

/// Compare a null-terminated C string against a byte slice field.
/// The field is expected to be null-terminated in its buffer.
unsafe fn cstr_eq_field(cstr: *const u8, field: &[u8]) -> bool {
    unsafe {
        let mut i = 0;
        loop {
            let c = *cstr.add(i);
            if i >= field.len() {
                return c == 0;
            }
            let f = field[i];
            if f == 0 {
                return c == 0;
            }
            if c != f {
                return false;
            }
            i += 1;
        }
    }
}

unsafe fn shadow_password_for(name: *const u8) -> *const u8 {
    unsafe {
        if !*(&raw const SHADOW_LOADED) {
            load_shadow_db();
        }

        let db = &raw const SHADOW_DB;
        let count = *(&raw const SHADOW_COUNT);
        for i in 0..count {
            let entry = &(*db)[i];
            if entry.active && cstr_eq_field(name, &entry.sp_namp) {
                return entry.sp_pwdp.as_ptr();
            }
        }

        ptr::null()
    }
}

unsafe fn passwd_password_ptr(e: &PasswdEntry) -> *const u8 {
    unsafe {
        if field_eq_literal(&e.pw_passwd, b"x\0") {
            let shadow = shadow_password_for(e.pw_name.as_ptr());
            if !shadow.is_null() {
                return shadow;
            }
        }

        e.pw_passwd.as_ptr()
    }
}

/// Split a line on `delim`, returning slices. `out` is filled with up to
/// `out.len()` fields. Returns the number of fields found.
fn split_line<'a>(line: &'a [u8], delim: u8, out: &mut [&'a [u8]]) -> usize {
    let mut count = 0;
    let mut start = 0;
    for i in 0..line.len() {
        if line[i] == delim {
            if count < out.len() {
                out[count] = &line[start..i];
                count += 1;
            }
            start = i + 1;
        }
    }
    // Last field (after final delimiter or entire line if no delimiter)
    if count < out.len() {
        out[count] = &line[start..];
        count += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// Fallback: hardcoded root entries
// ---------------------------------------------------------------------------

unsafe fn install_root_passwd_fallback() {
    unsafe {
        let db = &raw mut PASSWD_DB;
        let e = &mut (*db)[0];
        e.active = true;
        copy_field(&mut e.pw_name, b"root");
        copy_field(&mut e.pw_passwd, b"x");
        e.pw_uid = 0;
        e.pw_gid = 0;
        e.pw_change = 0;
        copy_field(&mut e.pw_class, b"root");
        copy_field(&mut e.pw_gecos, b"root");
        copy_field(&mut e.pw_dir, b"/");
        copy_field(&mut e.pw_shell, b"/bin/sh");
        e.pw_expire = 0;
        e.pw_fields =
            PWF_NAME | PWF_PASSWD | PWF_UID | PWF_GID | PWF_CLASS | PWF_GECOS | PWF_DIR | PWF_SHELL;
        *(&raw mut PASSWD_COUNT) = 1;
    }
}

unsafe fn install_root_group_fallback() {
    unsafe {
        let db = &raw mut GROUP_DB;
        let e = &mut (*db)[0];
        e.active = true;
        copy_field(&mut e.gr_name, b"root");
        copy_field(&mut e.gr_passwd, b"");
        e.gr_gid = 0;
        e.gr_member_count = 0;
        *(&raw mut GROUP_COUNT) = 1;
    }
}

unsafe fn install_root_shadow_fallback() {
    unsafe {
        let db = &raw mut SHADOW_DB;
        let e = &mut (*db)[0];
        e.active = true;
        copy_field(&mut e.sp_namp, b"root");
        // Empty hash means no password set — login should check this
        copy_field(&mut e.sp_pwdp, b"");
        e.sp_lstchg = 0;
        e.sp_min = 0;
        e.sp_max = 99999;
        e.sp_warn = 7;
        e.sp_inact = -1;
        e.sp_expire = -1;
        *(&raw mut SHADOW_COUNT) = 1;
    }
}

// ---------------------------------------------------------------------------
// Database loaders
// ---------------------------------------------------------------------------

/// Load the password database.
///
/// Tries `/etc/master.passwd` first (FreeBSD 10-field format carrying real
/// hashes, mtime, class, etc.), then falls back to `/etc/passwd` (POSIX
/// 7-field format), then to a hardcoded root entry.
unsafe fn load_passwd_db() {
    unsafe {
        *(&raw mut PASSWD_LOADED) = true;
        *(&raw mut PASSWD_COUNT) = 0;

        let mut buf = [0u8; FILE_BUF_SIZE];

        // Try master.passwd first
        let n = read_file(
            b"/etc/master.passwd\0".as_ptr(),
            buf.as_mut_ptr(),
            FILE_BUF_SIZE,
        );
        if n > 0 {
            parse_master_passwd(&buf[..n]);
            if *(&raw const PASSWD_COUNT) > 0 {
                return;
            }
        }

        // Fallback: /etc/passwd (POSIX 7-field)
        let n = read_file(b"/etc/passwd\0".as_ptr(), buf.as_mut_ptr(), FILE_BUF_SIZE);
        if n > 0 {
            parse_posix_passwd(&buf[..n]);
            if *(&raw const PASSWD_COUNT) > 0 {
                return;
            }
        }

        install_root_passwd_fallback();
    }
}

/// Parse FreeBSD `/etc/master.passwd` — 10 colon-separated fields:
/// `name:passwd:uid:gid:class:change:expire:gecos:dir:shell`
unsafe fn parse_master_passwd(data: &[u8]) {
    unsafe {
        let db = &raw mut PASSWD_DB;
        let count_ptr = &raw mut PASSWD_COUNT;
        let mut line_start = 0;

        for i in 0..data.len() {
            if data[i] == b'\n' || i == data.len() - 1 {
                let end = if data[i] == b'\n' { i } else { i + 1 };
                let line = &data[line_start..end];
                line_start = i + 1;

                if line.is_empty() || line[0] == b'#' {
                    continue;
                }

                let idx = *count_ptr;
                if idx >= MAX_USERS {
                    break;
                }

                let mut fields: [&[u8]; 10] = [&[]; 10];
                let nf = split_line(line, b':', &mut fields);
                if nf < 10 {
                    continue;
                }

                let e = &mut (*db)[idx];
                e.active = true;
                copy_field(&mut e.pw_name, fields[0]);
                copy_field(&mut e.pw_passwd, fields[1]);
                e.pw_uid = parse_u32(fields[2]);
                e.pw_gid = parse_u32(fields[3]);
                copy_field(&mut e.pw_class, fields[4]);
                e.pw_change = parse_i64(fields[5]).0;
                if e.pw_change < 0 {
                    e.pw_change = 0;
                }
                e.pw_expire = parse_i64(fields[6]).0;
                if e.pw_expire < 0 {
                    e.pw_expire = 0;
                }
                copy_field(&mut e.pw_gecos, fields[7]);
                copy_field(&mut e.pw_dir, fields[8]);
                copy_field(&mut e.pw_shell, fields[9]);
                e.pw_fields = PWF_NAME
                    | PWF_PASSWD
                    | PWF_UID
                    | PWF_GID
                    | PWF_CLASS
                    | PWF_CHANGE
                    | PWF_EXPIRE
                    | PWF_GECOS
                    | PWF_DIR
                    | PWF_SHELL;
                *count_ptr = idx + 1;
            }
        }
    }
}

/// Parse POSIX `/etc/passwd` — 7 colon-separated fields:
/// `name:passwd:uid:gid:gecos:dir:shell`
unsafe fn parse_posix_passwd(data: &[u8]) {
    unsafe {
        let db = &raw mut PASSWD_DB;
        let count_ptr = &raw mut PASSWD_COUNT;
        let mut line_start = 0;

        for i in 0..data.len() {
            if data[i] == b'\n' || i == data.len() - 1 {
                let end = if data[i] == b'\n' { i } else { i + 1 };
                let line = &data[line_start..end];
                line_start = i + 1;

                if line.is_empty() || line[0] == b'#' {
                    continue;
                }

                let idx = *count_ptr;
                if idx >= MAX_USERS {
                    break;
                }

                let mut fields: [&[u8]; 7] = [&[]; 7];
                let nf = split_line(line, b':', &mut fields);
                if nf < 7 {
                    continue;
                }

                let e = &mut (*db)[idx];
                e.active = true;
                copy_field(&mut e.pw_name, fields[0]);
                copy_field(&mut e.pw_passwd, fields[1]);
                e.pw_uid = parse_u32(fields[2]);
                e.pw_gid = parse_u32(fields[3]);
                e.pw_change = 0;
                // POSIX /etc/passwd has no class field. Derive a default class
                // so login_getpwclass() can still dispatch correctly.
                if e.pw_uid == 0 {
                    copy_field(&mut e.pw_class, b"root");
                } else {
                    copy_field(&mut e.pw_class, b"");
                }
                copy_field(&mut e.pw_gecos, fields[4]);
                copy_field(&mut e.pw_dir, fields[5]);
                copy_field(&mut e.pw_shell, fields[6]);
                e.pw_expire = 0;
                e.pw_fields = PWF_NAME
                    | PWF_PASSWD
                    | PWF_UID
                    | PWF_GID
                    | PWF_CLASS
                    | PWF_GECOS
                    | PWF_DIR
                    | PWF_SHELL;
                *count_ptr = idx + 1;
            }
        }
    }
}

/// Load /etc/group into GROUP_DB. Falls back to root-only on failure.
unsafe fn load_group_db() {
    unsafe {
        *(&raw mut GROUP_LOADED) = true;
        *(&raw mut GROUP_COUNT) = 0;

        let mut buf = [0u8; FILE_BUF_SIZE];
        let n = read_file(b"/etc/group\0".as_ptr(), buf.as_mut_ptr(), FILE_BUF_SIZE);
        if n == 0 {
            install_root_group_fallback();
            return;
        }

        let data = &buf[..n];
        let mut line_start = 0;
        let db = &raw mut GROUP_DB;
        let count_ptr = &raw mut GROUP_COUNT;

        for i in 0..n {
            if data[i] == b'\n' || i == n - 1 {
                let end = if data[i] == b'\n' { i } else { i + 1 };
                let line = &data[line_start..end];
                line_start = i + 1;

                if line.is_empty() || line[0] == b'#' {
                    continue;
                }

                let idx = *count_ptr;
                if idx >= MAX_GROUPS {
                    break;
                }

                // name:passwd:gid:member1,member2,...
                let mut fields: [&[u8]; 4] = [&[]; 4];
                let nf = split_line(line, b':', &mut fields);
                if nf < 3 {
                    continue;
                }

                let e = &mut (*db)[idx];
                e.active = true;
                copy_field(&mut e.gr_name, fields[0]);
                copy_field(&mut e.gr_passwd, fields[1]);
                e.gr_gid = parse_u32(fields[2]);

                // Parse member list (comma-separated)
                e.gr_member_count = 0;
                if nf >= 4 && !fields[3].is_empty() {
                    let mut members: [&[u8]; MAX_GROUP_MEMBERS] = [&[]; MAX_GROUP_MEMBERS];
                    let nm = split_line(fields[3], b',', &mut members);
                    for j in 0..nm {
                        if j >= MAX_GROUP_MEMBERS {
                            break;
                        }
                        if !members[j].is_empty() {
                            copy_field(&mut e.gr_members[j], members[j]);
                            e.gr_member_count += 1;
                        }
                    }
                }

                *count_ptr = idx + 1;
            }
        }

        if *count_ptr == 0 {
            install_root_group_fallback();
        }
    }
}

/// Load /etc/shadow into SHADOW_DB. Falls back to root-only on failure.
unsafe fn load_shadow_db() {
    unsafe {
        *(&raw mut SHADOW_LOADED) = true;
        *(&raw mut SHADOW_COUNT) = 0;

        let mut buf = [0u8; FILE_BUF_SIZE];
        let n = read_file(b"/etc/shadow\0".as_ptr(), buf.as_mut_ptr(), FILE_BUF_SIZE);
        if n == 0 {
            install_root_shadow_fallback();
            return;
        }

        let data = &buf[..n];
        let mut line_start = 0;
        let db = &raw mut SHADOW_DB;
        let count_ptr = &raw mut SHADOW_COUNT;

        for i in 0..n {
            if data[i] == b'\n' || i == n - 1 {
                let end = if data[i] == b'\n' { i } else { i + 1 };
                let line = &data[line_start..end];
                line_start = i + 1;

                if line.is_empty() || line[0] == b'#' {
                    continue;
                }

                let idx = *count_ptr;
                if idx >= MAX_SHADOW {
                    break;
                }

                // name:hash:lstchg:min:max:warn:inactive:expire:reserved
                let mut fields: [&[u8]; 9] = [&[]; 9];
                let nf = split_line(line, b':', &mut fields);
                if nf < 2 {
                    continue;
                }

                let e = &mut (*db)[idx];
                e.active = true;
                copy_field(&mut e.sp_namp, fields[0]);
                copy_field(&mut e.sp_pwdp, fields[1]);
                if nf > 2 {
                    e.sp_lstchg = parse_i64(fields[2]).0;
                }
                if nf > 3 {
                    e.sp_min = parse_i64(fields[3]).0;
                }
                if nf > 4 {
                    e.sp_max = parse_i64(fields[4]).0;
                }
                if nf > 5 {
                    e.sp_warn = parse_i64(fields[5]).0;
                }
                if nf > 6 {
                    e.sp_inact = parse_i64(fields[6]).0;
                }
                if nf > 7 {
                    e.sp_expire = parse_i64(fields[7]).0;
                }

                *count_ptr = idx + 1;
            }
        }

        if *count_ptr == 0 {
            install_root_shadow_fallback();
        }
    }
}

// ---------------------------------------------------------------------------
// Internal lookup helpers
// ---------------------------------------------------------------------------

/// Ensure passwd DB is loaded, then return a pointer to the matching entry
/// (or null). Also populates RET_PASSWD.
unsafe fn passwd_lookup<F: Fn(&PasswdEntry) -> bool>(pred: F) -> *mut Passwd {
    unsafe {
        if !*(&raw const PASSWD_LOADED) {
            load_passwd_db();
        }
        let db = &raw const PASSWD_DB;
        let count = *(&raw const PASSWD_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && pred(e) {
                fill_ret_passwd(e);
                return &raw mut RET_PASSWD;
            }
        }
        ptr::null_mut()
    }
}

/// Populate the static RET_PASSWD from a PasswdEntry.
unsafe fn fill_ret_passwd(e: &PasswdEntry) {
    unsafe {
        let p = &raw mut RET_PASSWD;
        (*p).pw_name = e.pw_name.as_ptr();
        (*p).pw_passwd = passwd_password_ptr(e);
        (*p).pw_uid = e.pw_uid;
        (*p).pw_gid = e.pw_gid;
        (*p).pw_change = e.pw_change;
        (*p).pw_class = e.pw_class.as_ptr();
        (*p).pw_gecos = e.pw_gecos.as_ptr();
        (*p).pw_dir = e.pw_dir.as_ptr();
        (*p).pw_shell = e.pw_shell.as_ptr();
        (*p).pw_expire = e.pw_expire;
        (*p).pw_fields = e.pw_fields;
    }
}

/// Ensure group DB is loaded, then return a pointer to the matching entry.
unsafe fn group_lookup<F: Fn(&GroupEntry) -> bool>(pred: F) -> *mut Group {
    unsafe {
        if !*(&raw const GROUP_LOADED) {
            load_group_db();
        }
        let db = &raw const GROUP_DB;
        let count = *(&raw const GROUP_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && pred(e) {
                fill_ret_group(e);
                return &raw mut RET_GROUP;
            }
        }
        ptr::null_mut()
    }
}

/// Populate the static RET_GROUP from a GroupEntry.
unsafe fn fill_ret_group(e: &GroupEntry) {
    unsafe {
        let g = &raw mut RET_GROUP;
        let mem = &raw mut RET_GROUP_MEMBERS;

        // Build null-terminated member pointer array
        for j in 0..e.gr_member_count {
            (*mem)[j] = e.gr_members[j].as_ptr();
        }
        (*mem)[e.gr_member_count] = ptr::null();

        (*g).gr_name = e.gr_name.as_ptr();
        (*g).gr_passwd = e.gr_passwd.as_ptr();
        (*g).gr_gid = e.gr_gid;
        (*g).gr_mem = (*mem).as_ptr();
    }
}

/// Ensure shadow DB is loaded, then return a pointer to the matching entry.
unsafe fn shadow_lookup<F: Fn(&SpwdEntry) -> bool>(pred: F) -> *mut Spwd {
    unsafe {
        if !*(&raw const SHADOW_LOADED) {
            load_shadow_db();
        }
        let db = &raw const SHADOW_DB;
        let count = *(&raw const SHADOW_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && pred(e) {
                fill_ret_spwd(e);
                return &raw mut RET_SPWD;
            }
        }
        ptr::null_mut()
    }
}

/// Populate the static RET_SPWD from a SpwdEntry.
unsafe fn fill_ret_spwd(e: &SpwdEntry) {
    unsafe {
        let s = &raw mut RET_SPWD;
        (*s).sp_namp = e.sp_namp.as_ptr();
        (*s).sp_pwdp = e.sp_pwdp.as_ptr();
        (*s).sp_lstchg = e.sp_lstchg;
        (*s).sp_min = e.sp_min;
        (*s).sp_max = e.sp_max;
        (*s).sp_warn = e.sp_warn;
        (*s).sp_inact = e.sp_inact;
        (*s).sp_expire = e.sp_expire;
        (*s).sp_flag = 0;
    }
}

// ---------------------------------------------------------------------------
// Reentrant helpers
// ---------------------------------------------------------------------------

/// Copy passwd entry fields into a caller-provided buffer and populate `pwd`.
/// Returns 0 on success, ERANGE if buffer is too small.
unsafe fn fill_passwd_r(
    e: &PasswdEntry,
    pwd: *mut Passwd,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Passwd,
) -> i32 {
    const ERANGE: i32 = 34;
    unsafe {
        // Calculate needed space: each field null-terminated
        let name_len = cstr_len(&e.pw_name) + 1;
        let passwd_src = passwd_password_ptr(e);
        let passwd_len = cstr_len_ptr(passwd_src) + 1;
        let class_len = cstr_len(&e.pw_class) + 1;
        let gecos_len = cstr_len(&e.pw_gecos) + 1;
        let dir_len = cstr_len(&e.pw_dir) + 1;
        let shell_len = cstr_len(&e.pw_shell) + 1;
        let need = name_len + passwd_len + class_len + gecos_len + dir_len + shell_len;

        if buflen < need {
            *result = ptr::null_mut();
            return ERANGE;
        }

        let mut off = 0;

        let name_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.pw_name.as_ptr(), buf.add(off), name_len);
        off += name_len;

        let passwd_ptr = buf.add(off);
        ptr::copy_nonoverlapping(passwd_src, buf.add(off), passwd_len);
        off += passwd_len;

        let class_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.pw_class.as_ptr(), buf.add(off), class_len);
        off += class_len;

        let gecos_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.pw_gecos.as_ptr(), buf.add(off), gecos_len);
        off += gecos_len;

        let dir_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.pw_dir.as_ptr(), buf.add(off), dir_len);
        off += dir_len;

        let shell_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.pw_shell.as_ptr(), buf.add(off), shell_len);

        (*pwd).pw_name = name_ptr;
        (*pwd).pw_passwd = passwd_ptr;
        (*pwd).pw_uid = e.pw_uid;
        (*pwd).pw_gid = e.pw_gid;
        (*pwd).pw_change = e.pw_change;
        (*pwd).pw_class = class_ptr;
        (*pwd).pw_gecos = gecos_ptr;
        (*pwd).pw_dir = dir_ptr;
        (*pwd).pw_shell = shell_ptr;
        (*pwd).pw_expire = e.pw_expire;
        (*pwd).pw_fields = e.pw_fields;

        *result = pwd;
        0
    }
}

/// Copy group entry fields into a caller-provided buffer and populate `grp`.
/// Returns 0 on success, ERANGE if buffer is too small.
unsafe fn fill_group_r(
    e: &GroupEntry,
    grp: *mut Group,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Group,
) -> i32 {
    const ERANGE: i32 = 34;
    unsafe {
        let name_len = cstr_len(&e.gr_name) + 1;
        let passwd_len = cstr_len(&e.gr_passwd) + 1;
        // Member strings + pointer array (null-terminated)
        let mut member_total = 0usize;
        for j in 0..e.gr_member_count {
            member_total += cstr_len(&e.gr_members[j]) + 1;
        }
        let ptrs_size = (e.gr_member_count + 1) * core::mem::size_of::<*const u8>();
        let align = core::mem::align_of::<*const u8>();

        let need = name_len + passwd_len + member_total + ptrs_size + align.saturating_sub(1);
        if buflen < need {
            *result = ptr::null_mut();
            return ERANGE;
        }

        let mut off = 0;

        let name_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.gr_name.as_ptr(), buf.add(off), name_len);
        off += name_len;

        let passwd_ptr = buf.add(off);
        ptr::copy_nonoverlapping(e.gr_passwd.as_ptr(), buf.add(off), passwd_len);
        off += passwd_len;

        // Copy member strings
        let mut mem_str_ptrs = [ptr::null::<u8>(); MAX_GROUP_MEMBERS];
        for j in 0..e.gr_member_count {
            mem_str_ptrs[j] = buf.add(off);
            let mlen = cstr_len(&e.gr_members[j]) + 1;
            ptr::copy_nonoverlapping(e.gr_members[j].as_ptr(), buf.add(off), mlen);
            off += mlen;
        }

        // Align offset for pointer array
        off = (off + align - 1) & !(align - 1);
        if off.checked_add(ptrs_size).is_none_or(|end| end > buflen) {
            *result = ptr::null_mut();
            return ERANGE;
        }

        // Write pointer array into buffer
        let mem_arr = buf.add(off) as *mut *const u8;
        for j in 0..e.gr_member_count {
            *mem_arr.add(j) = mem_str_ptrs[j];
        }
        *mem_arr.add(e.gr_member_count) = ptr::null();

        (*grp).gr_name = name_ptr;
        (*grp).gr_passwd = passwd_ptr;
        (*grp).gr_gid = e.gr_gid;
        (*grp).gr_mem = mem_arr;

        *result = grp;
        0
    }
}

/// Length of a null-terminated string in a fixed-size buffer.
fn cstr_len(buf: &[u8]) -> usize {
    for i in 0..buf.len() {
        if buf[i] == 0 {
            return i;
        }
    }
    buf.len()
}

unsafe fn cstr_len_ptr(ptr: *const u8) -> usize {
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        len
    }
}

// ===========================================================================
// Public API — Passwd
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwnam(name: *const u8) -> *mut Passwd {
    if name.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: accessing static database; single-threaded POSIX semantics
    unsafe { passwd_lookup(|e| cstr_eq_field(name, &e.pw_name)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwuid(uid: u32) -> *mut Passwd {
    // SAFETY: accessing static database; single-threaded POSIX semantics
    unsafe { passwd_lookup(|e| e.pw_uid == uid) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwent() -> *mut Passwd {
    unsafe {
        if !*(&raw const PASSWD_LOADED) {
            load_passwd_db();
        }
        let iter = *(&raw const PASSWD_ITER);
        let count = *(&raw const PASSWD_COUNT);
        if iter < count {
            let db = &raw const PASSWD_DB;
            let e = &(*db)[iter];
            *(&raw mut PASSWD_ITER) = iter + 1;
            fill_ret_passwd(e);
            &raw mut RET_PASSWD
        } else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setpwent() {
    unsafe {
        // Reset iterator and force re-read on next access
        *(&raw mut PASSWD_ITER) = 0;
        *(&raw mut PASSWD_LOADED) = false;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn endpwent() {
    unsafe {
        *(&raw mut PASSWD_ITER) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwnam_r(
    name: *const u8,
    pwd: *mut Passwd,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Passwd,
) -> i32 {
    unsafe {
        if name.is_null() {
            *result = ptr::null_mut();
            return 0;
        }
        if !*(&raw const PASSWD_LOADED) {
            load_passwd_db();
        }
        let db = &raw const PASSWD_DB;
        let count = *(&raw const PASSWD_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && cstr_eq_field(name, &e.pw_name) {
                return fill_passwd_r(e, pwd, buf, buflen, result);
            }
        }
        *result = ptr::null_mut();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpwuid_r(
    uid: u32,
    pwd: *mut Passwd,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Passwd,
) -> i32 {
    unsafe {
        if !*(&raw const PASSWD_LOADED) {
            load_passwd_db();
        }
        let db = &raw const PASSWD_DB;
        let count = *(&raw const PASSWD_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && e.pw_uid == uid {
                return fill_passwd_r(e, pwd, buf, buflen, result);
            }
        }
        *result = ptr::null_mut();
        0
    }
}

// ===========================================================================
// Public API — Group
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrnam(name: *const u8) -> *mut Group {
    if name.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: accessing static database; single-threaded POSIX semantics
    unsafe { group_lookup(|e| cstr_eq_field(name, &e.gr_name)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrgid(gid: u32) -> *mut Group {
    // SAFETY: accessing static database; single-threaded POSIX semantics
    unsafe { group_lookup(|e| e.gr_gid == gid) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrent() -> *mut Group {
    unsafe {
        if !*(&raw const GROUP_LOADED) {
            load_group_db();
        }
        let iter = *(&raw const GROUP_ITER);
        let count = *(&raw const GROUP_COUNT);
        if iter < count {
            let db = &raw const GROUP_DB;
            let e = &(*db)[iter];
            *(&raw mut GROUP_ITER) = iter + 1;
            fill_ret_group(e);
            &raw mut RET_GROUP
        } else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgrent() {
    unsafe {
        *(&raw mut GROUP_ITER) = 0;
        *(&raw mut GROUP_LOADED) = false;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn endgrent() {
    unsafe {
        *(&raw mut GROUP_ITER) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrnam_r(
    name: *const u8,
    grp: *mut Group,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Group,
) -> i32 {
    unsafe {
        if name.is_null() {
            *result = ptr::null_mut();
            return 0;
        }
        if !*(&raw const GROUP_LOADED) {
            load_group_db();
        }
        let db = &raw const GROUP_DB;
        let count = *(&raw const GROUP_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && cstr_eq_field(name, &e.gr_name) {
                return fill_group_r(e, grp, buf, buflen, result);
            }
        }
        *result = ptr::null_mut();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrgid_r(
    gid: u32,
    grp: *mut Group,
    buf: *mut u8,
    buflen: usize,
    result: *mut *mut Group,
) -> i32 {
    unsafe {
        if !*(&raw const GROUP_LOADED) {
            load_group_db();
        }
        let db = &raw const GROUP_DB;
        let count = *(&raw const GROUP_COUNT);
        for i in 0..count {
            let e = &(*db)[i];
            if e.active && e.gr_gid == gid {
                return fill_group_r(e, grp, buf, buflen, result);
            }
        }
        *result = ptr::null_mut();
        0
    }
}

// ===========================================================================
// Public API — Shadow
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getspnam(name: *const u8) -> *mut Spwd {
    if name.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: accessing static database; single-threaded POSIX semantics
    unsafe { shadow_lookup(|e| cstr_eq_field(name, &e.sp_namp)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getspent() -> *mut Spwd {
    unsafe {
        if !*(&raw const SHADOW_LOADED) {
            load_shadow_db();
        }
        let iter = *(&raw const SHADOW_ITER);
        let count = *(&raw const SHADOW_COUNT);
        if iter < count {
            let db = &raw const SHADOW_DB;
            let e = &(*db)[iter];
            *(&raw mut SHADOW_ITER) = iter + 1;
            fill_ret_spwd(e);
            &raw mut RET_SPWD
        } else {
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setspent() {
    unsafe {
        *(&raw mut SHADOW_ITER) = 0;
        *(&raw mut SHADOW_LOADED) = false;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn endspent() {
    unsafe {
        *(&raw mut SHADOW_ITER) = 0;
    }
}

// ===========================================================================
// Public API — user_from_uid / group_from_gid
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn user_from_uid(uid: u32, noname: i32) -> *const u8 {
    unsafe {
        let pw = getpwuid(uid);
        if !pw.is_null() {
            return (*pw).pw_name;
        }
        if noname != 0 {
            return ptr::null();
        }
        // SAFETY: writing to static buffer for numeric fallback
        let buf = &raw mut UID_BUF;
        format_u32(uid, (*buf).as_mut_ptr(), 32);
        (*buf).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn group_from_gid(gid: u32, noname: i32) -> *const u8 {
    unsafe {
        let gr = getgrgid(gid);
        if !gr.is_null() {
            return (*gr).gr_name;
        }
        if noname != 0 {
            return ptr::null();
        }
        // SAFETY: writing to static buffer for numeric fallback
        let buf = &raw mut GID_BUF;
        format_u32(gid, (*buf).as_mut_ptr(), 32);
        (*buf).as_ptr()
    }
}

// ===========================================================================
// Internal helpers for initgroups()
// ===========================================================================

/// Collect the supplementary group ids for `user`.
///
/// `primary_gid` is stored at `out[0]` unconditionally (matching POSIX
/// `initgroups` semantics where the primary gid is always part of the list).
/// Additional gids are taken from GROUP_DB entries whose member list contains
/// `user`. Duplicates are eliminated. The function stops writing when `out`
/// is full.
///
/// Returns the number of gids written to `out`.
pub(crate) unsafe fn groups_for_user(user: *const u8, primary_gid: u32, out: &mut [u32]) -> usize {
    if out.is_empty() || user.is_null() {
        return 0;
    }
    unsafe {
        if !*(&raw const GROUP_LOADED) {
            load_group_db();
        }

        out[0] = primary_gid;
        let mut count = 1usize;

        let db = &raw const GROUP_DB;
        let total = *(&raw const GROUP_COUNT);
        for i in 0..total {
            if count >= out.len() {
                break;
            }
            let e = &(*db)[i];
            if !e.active {
                continue;
            }

            // Skip primary_gid — already in out[0].
            if e.gr_gid == primary_gid {
                continue;
            }

            // Check if user is listed as a member of this group.
            let mut is_member = false;
            for j in 0..e.gr_member_count {
                if cstr_eq_field(user, &e.gr_members[j]) {
                    is_member = true;
                    break;
                }
            }
            if !is_member {
                continue;
            }

            // Deduplicate against already-collected gids.
            let mut dup = false;
            for k in 0..count {
                if out[k] == e.gr_gid {
                    dup = true;
                    break;
                }
            }
            if dup {
                continue;
            }

            out[count] = e.gr_gid;
            count += 1;
        }

        count
    }
}

unsafe fn format_u32(mut val: u32, buf: *mut u8, buflen: usize) {
    unsafe {
        if buflen == 0 {
            return;
        }
        let mut tmp = [0u8; 12];
        let mut i = 0usize;
        if val == 0 {
            tmp[0] = b'0';
            i = 1;
        } else {
            while val > 0 && i < 11 {
                tmp[i] = b'0' + (val % 10) as u8;
                val /= 10;
                i += 1;
            }
        }
        let copy_len = if i < buflen - 1 { i } else { buflen - 1 };
        for j in 0..copy_len {
            *buf.add(j) = tmp[i - 1 - j];
        }
        *buf.add(copy_len) = 0;
    }
}
