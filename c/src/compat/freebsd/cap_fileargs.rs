//! Capsicum fileargs wrapper for SaltyOS
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Real implementation that wraps POSIX open/fopen. The `fileargs_t` struct
//! stores the open() flags and mode passed to `fileargs_init()`, then
//! `fileargs_open()` calls open() and `fileargs_fopen()` calls fopen().
//!
//! SaltyOS does not implement Capsicum sandboxing — these wrappers exist to
//! satisfy FreeBSD port code that uses cap_fileargs.

use crate::stdio::FILE;
use crate::unistd::Stat;

unsafe extern "C" {
    fn open(path: *const u8, flags: i32, ...) -> i32;
    fn fopen(path: *const u8, mode: *const u8) -> *mut FILE;
    fn realpath(path: *const u8, resolved: *mut u8) -> *mut u8;
    fn lstat(path: *const u8, buf: *mut Stat) -> i32;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

/// Capsicum fileargs handle. Stores flags and mode for deferred open().
#[repr(C)]
pub struct FileArgs {
    flags: i32,
    mode: u32,
}

/// Type alias matching the C `fileargs_t` name.
pub type fileargs_t = FileArgs;

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Create a fileargs handle storing the given open() flags and mode.
///
/// `argc` and `argv` are accepted for API compatibility but ignored.
/// Returns a heap-allocated handle, or null on allocation failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_init(
    _argc: i32,
    _argv: *mut *mut u8,
    flags: i32,
    mode: u32,
    _args: ...,
) -> *mut fileargs_t {
    unsafe {
        let fa = malloc(core::mem::size_of::<fileargs_t>()) as *mut fileargs_t;
        if fa.is_null() {
            return core::ptr::null_mut();
        }
        (*fa).flags = flags;
        (*fa).mode = mode;
        fa
    }
}

/// Create a fileargs handle (Casper variant).
///
/// Identical to `fileargs_init` — the `casper_cap` parameter is accepted for
/// API compatibility but ignored since SaltyOS does not use Casper/Capsicum.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_cinit(
    _casper_cap: *mut u8,
    _argc: i32,
    _argv: *mut *mut u8,
    flags: i32,
    mode: u32,
    _args: ...,
) -> *mut fileargs_t {
    unsafe {
        let fa = malloc(core::mem::size_of::<fileargs_t>()) as *mut fileargs_t;
        if fa.is_null() {
            return core::ptr::null_mut();
        }
        (*fa).flags = flags;
        (*fa).mode = mode;
        fa
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Open a file using the flags and mode stored in the fileargs handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_open(fa: *mut fileargs_t, name: *const u8) -> i32 {
    if fa.is_null() || name.is_null() {
        return -1;
    }
    unsafe { open(name, (*fa).flags, (*fa).mode) }
}

/// Open a file using fopen(). The fileargs handle is accepted for API
/// compatibility but the open mode comes from the `mode` string parameter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_fopen(
    _fa: *mut fileargs_t,
    name: *const u8,
    mode: *const u8,
) -> *mut FILE {
    if name.is_null() || mode.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { fopen(name, mode) }
}

/// Resolve a pathname. The fileargs handle is accepted for API compatibility
/// but ignored — delegates directly to realpath().
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_realpath(
    _fa: *mut fileargs_t,
    path: *const u8,
    resolved: *mut u8,
) -> *mut u8 {
    unsafe { realpath(path, resolved) }
}

/// Stat a file (no symlink follow). The fileargs handle is accepted for API
/// compatibility but ignored — delegates directly to lstat().
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_lstat(
    _fa: *mut fileargs_t,
    name: *const u8,
    sb: *mut Stat,
) -> i32 {
    unsafe { lstat(name, sb) }
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

/// Free a fileargs handle. Accepts null (no-op).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fileargs_free(fa: *mut fileargs_t) {
    unsafe { free(fa as *mut u8) }
}
