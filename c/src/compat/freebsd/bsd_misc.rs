//! Miscellaneous FreeBSD/BSD compatibility functions
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Functions here have no POSIX equivalent or are FreeBSD-specific
//! interfaces used by ported utilities.

use crate::errno;

// ---------------------------------------------------------------------------
// getprogname / setprogname — BSD program name accessors
// ---------------------------------------------------------------------------

static mut PROGNAME: *const u8 = b"\0".as_ptr();

/// BSD `__progname` global — alias for getprogname() result.
#[unsafe(no_mangle)]
pub static mut __progname: *const u8 = core::ptr::null();

#[unsafe(no_mangle)]
pub extern "C" fn getprogname() -> *const u8 {
    // SAFETY: PROGNAME is only written via setprogname and during startup.
    unsafe { *(&raw const PROGNAME) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setprogname(name: *const u8) {
    if name.is_null() {
        return;
    }
    unsafe {
        // Store the basename portion (after last '/')
        let mut last_slash: *const u8 = core::ptr::null();
        let mut p = name;
        while *p != 0 {
            if *p == b'/' {
                last_slash = p;
            }
            p = p.add(1);
        }
        if !last_slash.is_null() {
            *(&raw mut PROGNAME) = last_slash.add(1);
        } else {
            *(&raw mut PROGNAME) = name;
        }
        __progname = *(&raw const PROGNAME);
    }
}

// ---------------------------------------------------------------------------
// getprogpath — resolve executable path from argv[0] via PATH search
// ---------------------------------------------------------------------------

unsafe extern "C" {
    safe fn stat(path: *const u8, buf: *mut u8) -> i32;
    safe fn realpath(name: *const u8, resolved: *mut u8) -> *mut u8;
    safe fn getcwd(buf: *mut u8, size: usize) -> *mut u8;
    safe fn getenv(name: *const u8) -> *mut u8;
    safe fn strdup(s: *const u8) -> *mut u8;
    safe fn strtok_r(s: *mut u8, delim: *const u8, saveptr: *mut *mut u8) -> *mut u8;
    safe fn free(ptr: *mut u8);
}

const PATH_MAX: usize = 4096;

/// Resolve the full path of `argv0` by searching PATH.
///
/// On success, writes the resolved path into `buf` (which must be at least
/// PATH_MAX bytes) and returns `buf`. Returns null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getprogpath(buf: *mut u8, argv0: *const u8) -> *mut u8 {
    unsafe {
        if argv0.is_null() || *argv0 == 0 {
            return core::ptr::null_mut();
        }

        // Absolute path
        if *argv0 == b'/' {
            if realpath(argv0, buf).is_null() {
                return core::ptr::null_mut();
            }
            let mut st = [0u8; 256];
            if stat(buf, st.as_mut_ptr()) != 0 {
                return core::ptr::null_mut();
            }
            return buf;
        }

        // Relative path (contains '/')
        let mut p = argv0;
        let mut has_slash = false;
        while *p != 0 {
            if *p == b'/' {
                has_slash = true;
                break;
            }
            p = p.add(1);
        }
        if has_slash {
            let mut cwd = [0u8; PATH_MAX];
            if getcwd(cwd.as_mut_ptr(), PATH_MAX).is_null() {
                return core::ptr::null_mut();
            }
            // Build cwd/argv0
            let mut full = [0u8; PATH_MAX];
            let mut i = 0usize;
            let mut cp = cwd.as_ptr();
            while *cp != 0 && i < PATH_MAX - 2 {
                full[i] = *cp;
                i += 1;
                cp = cp.add(1);
            }
            full[i] = b'/';
            i += 1;
            cp = argv0;
            while *cp != 0 && i < PATH_MAX - 1 {
                full[i] = *cp;
                i += 1;
                cp = cp.add(1);
            }
            full[i] = 0;
            if realpath(full.as_ptr(), buf).is_null() {
                return core::ptr::null_mut();
            }
            return buf;
        }

        // PATH search
        let path_env = getenv(b"PATH\0".as_ptr());
        if path_env.is_null() {
            return core::ptr::null_mut();
        }
        let path_copy = strdup(path_env);
        if path_copy.is_null() {
            return core::ptr::null_mut();
        }

        let mut state: *mut u8 = core::ptr::null_mut();
        let mut tok = strtok_r(path_copy, b":\0".as_ptr(), &raw mut state);
        while !tok.is_null() {
            let mut full = [0u8; PATH_MAX];
            let mut i = 0usize;
            let mut cp = tok as *const u8;
            while *cp != 0 && i < PATH_MAX - 2 {
                full[i] = *cp;
                i += 1;
                cp = cp.add(1);
            }
            full[i] = b'/';
            i += 1;
            cp = argv0;
            while *cp != 0 && i < PATH_MAX - 1 {
                full[i] = *cp;
                i += 1;
                cp = cp.add(1);
            }
            full[i] = 0;

            let mut st = [0u8; 256];
            if stat(full.as_ptr(), st.as_mut_ptr()) == 0 {
                if !realpath(full.as_ptr(), buf).is_null() {
                    free(path_copy);
                    return buf;
                }
            }
            tok = strtok_r(core::ptr::null_mut(), b":\0".as_ptr(), &raw mut state);
        }
        free(path_copy);
        core::ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// __xuname — FreeBSD uname wrapper
// ---------------------------------------------------------------------------

/// __xuname — FreeBSD's uname() is a macro that calls __xuname(SYS_NMLN, buf).
/// We ignore the nmln parameter and fill our standard Utsname.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __xuname(_nmln: i32, buf: *mut crate::sysinfo::Utsname) -> i32 {
    unsafe { crate::sysinfo::uname(buf) }
}

// ---------------------------------------------------------------------------
// FreeBSD-specific miscellaneous functions
// ---------------------------------------------------------------------------

/// getosreldate — FreeBSD OS release date.
#[unsafe(no_mangle)]
pub extern "C" fn getosreldate() -> i32 {
    1402000
}

/// getloginclass — FreeBSD login class. Not supported.
#[unsafe(no_mangle)]
pub extern "C" fn getloginclass(_buf: *mut u8, _len: usize) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

/// getlogin — return login name. Returns "root".
#[unsafe(no_mangle)]
pub extern "C" fn getlogin() -> *const u8 {
    b"root\0".as_ptr()
}

/// getgrouplist — get groups for user. Returns just the primary group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgrouplist(
    _user: *const u8,
    group: u32,
    groups: *mut u32,
    ngroups: *mut i32,
) -> i32 {
    unsafe {
        if !groups.is_null() && !ngroups.is_null() && *ngroups >= 1 {
            *groups = group;
            *ngroups = 1;
            return 0;
        }
        if !ngroups.is_null() {
            *ngroups = 1;
        }
        -1
    }
}

/// pledge — OpenBSD security model. No-op on SaltyOS (uses capabilities).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pledge(_promises: *const u8, _execpromises: *const u8) -> i32 {
    0
}

/// unveil — OpenBSD security model. No-op on SaltyOS (uses capabilities).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unveil(_path: *const u8, _permissions: *const u8) -> i32 {
    0
}

/// lchmod — chmod on symlink. Not supported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lchmod(_path: *const u8, _mode: u32) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

/// mknod — create device special file. Not supported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mknod(_path: *const u8, _mode: u32, _dev: u64) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

unsafe extern "C" {
    safe fn abort() -> !;
}

/// FreeBSD __assert(func, file, line, expr) — called by FreeBSD's assert() macro.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __assert(
    _func: *const u8,
    _file: *const u8,
    _line: i32,
    _expr: *const u8,
) -> ! {
    abort()
}

static mut GETBSIZE_BUF: [u8; 32] = [0; 32];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getbsize(headerlenp: *mut i32, blocksizep: *mut i64) -> *mut u8 {
    unsafe {
        // Default: 512-byte blocks, "512" header
        if !blocksizep.is_null() {
            *blocksizep = 512;
        }
        if !headerlenp.is_null() {
            *headerlenp = 3; // length of "512"
        }
        let buf = core::ptr::addr_of_mut!(GETBSIZE_BUF) as *mut u8;
        *buf.add(0) = b'5';
        *buf.add(1) = b'1';
        *buf.add(2) = b'2';
        *buf.add(3) = 0;
        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lpathconf(_path: *const u8, name: i32) -> i64 {
    unsafe { crate::unistd::pathconf(_path, name) }
}

unsafe extern "C" {
    safe fn access(path: *const u8, mode: i32) -> i32;
}

/// eaccess — like access() but uses effective uid/gid. We only have one uid so just alias.
#[unsafe(no_mangle)]
pub extern "C" fn eaccess(path: *const u8, mode: i32) -> i32 {
    access(path, mode)
}

unsafe extern "C" {
    safe fn fseek(stream: *mut u8, offset: i64, whence: i32) -> i32;
}

/// fseeko — same as fseek with off_t (both 64-bit on our platform).
#[unsafe(no_mangle)]
pub extern "C" fn fseeko(stream: *mut u8, offset: i64, whence: i32) -> i32 {
    fseek(stream, offset, whence)
}

/// ftello — same as ftell with off_t.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ftello(stream: *mut u8) -> i64 {
    unsafe extern "C" {
        safe fn ftell(stream: *mut u8) -> i64;
    }
    ftell(stream)
}

unsafe extern "C" {
    safe fn fork() -> i32;
}

/// vfork — alias for fork (no MMU optimization needed with COW).
#[unsafe(no_mangle)]
pub extern "C" fn vfork() -> i32 {
    fork()
}

/// kqueue — BSD event notification. Not supported.
#[unsafe(no_mangle)]
pub extern "C" fn kqueue() -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

/// kevent — BSD event notification. Not supported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kevent(
    _kq: i32,
    _changelist: *const u8,
    _nchanges: i32,
    _eventlist: *mut u8,
    _nevents: i32,
    _timeout: *const u8,
) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

/// fstatfs — not supported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fstatfs(_fd: i32, _buf: *mut u8) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

/// statfs — not supported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn statfs(_path: *const u8, _buf: *mut u8) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

// ---------------------------------------------------------------------------
// BSD string/number conversions
// ---------------------------------------------------------------------------

unsafe extern "C" {
    safe fn strtol(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64;
    safe fn strtoul(s: *const u8, endptr: *mut *mut u8, base: i32) -> u64;
    safe fn strcspn(s: *const u8, reject: *const u8) -> usize;
    safe fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
}

/// strtoq — BSD legacy alias for strtoll.
#[unsafe(no_mangle)]
pub extern "C" fn strtoq(s: *const u8, endptr: *mut *mut u8, base: i32) -> i64 {
    strtol(s, endptr, base)
}

/// strtouq — BSD legacy alias for strtoull.
#[unsafe(no_mangle)]
pub extern "C" fn strtouq(s: *const u8, endptr: *mut *mut u8, base: i32) -> u64 {
    strtoul(s, endptr, base)
}

/// strsep — 4.4BSD string tokenizer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strsep(stringp: *mut *mut u8, delim: *const u8) -> *mut u8 {
    unsafe {
        let s = *stringp;
        if s.is_null() {
            return core::ptr::null_mut();
        }
        let span = strcspn(s, delim);
        if *s.add(span) != 0 {
            *s.add(span) = 0;
            *stringp = s.add(span + 1);
        } else {
            *stringp = core::ptr::null_mut();
        }
        s
    }
}

/// sys_nsig — BSD signal count.
#[unsafe(no_mangle)]
pub static sys_nsig: i32 = 32;

/// reallocf — BSD realloc that frees ptr on failure.
#[unsafe(no_mangle)]
pub extern "C" fn reallocf(ptr: *mut u8, size: usize) -> *mut u8 {
    let result = realloc(ptr, size);
    if result.is_null() && size != 0 {
        free(ptr);
    }
    result
}

// ---------------------------------------------------------------------------
// elf_aux_info — FreeBSD auxiliary vector lookup
// ---------------------------------------------------------------------------

/// Walk the saved auxv (tag, value) pairs and copy the value for `aux` into
/// `buf`. For AT_EXECPATH (15), the value is treated as a C string pointer
/// and the string is copied. For all other tags the raw u64 value is copied.
/// Returns 0 on success, ENOENT if the tag is not present, EINVAL on bad args.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn elf_aux_info(aux: i32, buf: *mut core::ffi::c_void, buflen: i32) -> i32 {
    if buf.is_null() || buflen <= 0 {
        return errno::EINVAL;
    }
    unsafe {
        let auxv = core::ptr::addr_of!(crate::crt::SAVED_AUXV).read();
        if auxv.is_null() {
            return errno::ENOENT;
        }
        let mut p = auxv;
        loop {
            let tag = *p;
            let val = *p.add(1);
            if tag == 0 {
                break; // AT_NULL
            }
            if tag == aux as u64 {
                // AT_EXECPATH (15): value is a pointer to a C string
                if aux == 15 {
                    let src = val as *const u8;
                    if src.is_null() {
                        return errno::ENOENT;
                    }
                    let mut len = 0usize;
                    while *src.add(len) != 0 {
                        len += 1;
                    }
                    if len + 1 > buflen as usize {
                        return errno::EINVAL;
                    }
                    core::ptr::copy_nonoverlapping(src, buf as *mut u8, len + 1);
                    return 0;
                }
                // Integer-type: copy raw u64 value
                if (buflen as usize) < core::mem::size_of::<u64>() {
                    return errno::EINVAL;
                }
                core::ptr::copy_nonoverlapping(
                    &val as *const u64 as *const u8,
                    buf as *mut u8,
                    core::mem::size_of::<u64>(),
                );
                return 0;
            }
            p = p.add(2);
        }
    }
    errno::ENOENT
}

/// rpmatch — BSD yes/no response matching.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rpmatch(response: *const u8) -> i32 {
    unsafe {
        if response.is_null() || *response == 0 {
            return -1;
        }
        match *response {
            b'y' | b'Y' => 1,
            b'n' | b'N' => 0,
            _ => -1,
        }
    }
}
