//! SPDX-License-Identifier: GPL-2.0-only
//! basaltc dlfcn shim — every public C entry forwards into rtld through the
//! `RtldDlfcnV1` table installed by `trona_loader_runtime_install`.
//!
//! No ELF parsing, mapping, relocation, or link-map walking happens here.
//! libc owns the per-thread `dlerror` buffer pointer (lazy-allocated via
//! malloc) and stores it in `ThreadLocalBlock::dlerror_msg`. Every dlfcn
//! call passes that buffer to the rtld so error strings are written
//! per-thread. `ThreadLocalBlock::dlerror_pending` tracks whether the buffer
//! contains an unread error; the buffer itself stays intact until the next
//! dlfcn call overwrites it.
//!
//! Caller PC capture for RTLD_NEXT lives in arch-specific assembly trampolines
//! in `arch/<arch>/dlfcn_shim.S`. Those trampolines call into
//! [`dlsym_with_pc`] / [`dlfunc_with_pc`] below.

#![allow(non_camel_case_types)]

use crate::malloc;

const DLERROR_BUF_SIZE: usize = 256;

const RTLD_NEXT_HANDLE: *mut u8 = usize::MAX as *mut u8;
const DEFAULT_HANDLE_MARKER: *mut u8 = 1 as *mut u8;

/// Public Dl_info type — matches the C struct in `<dlfcn.h>` byte-for-byte
/// because it is just a re-export of the uapi type.
pub type DlInfo = trona_kernel::core_types::DlInfo;

/// Public dl_phdr_info — re-export of the uapi type.
pub type DlPhdrInfo = trona_kernel::core_types::DlPhdrInfo;

// ---------------------------------------------------------------------------
// Per-thread dlerror buffer
// ---------------------------------------------------------------------------

/// Return the current thread's dlerror buffer pointer, lazily allocating one
/// from libc malloc on first use. Returns null if no TLS is set up — in that
/// rare case errors are simply not reported.
fn dlerror_buf() -> *mut u8 {
    let tls = trona_runtime::thread::tls::current_tls();
    let tcb = match tls {
        Some(p) => p,
        None => return core::ptr::null_mut(),
    };
    let cur = unsafe { (*tcb).dlerror_msg };
    if !cur.is_null() {
        return cur;
    }
    let alloc = unsafe { malloc::malloc(DLERROR_BUF_SIZE) } as *mut u8;
    if alloc.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        *alloc = 0;
        (*tcb).dlerror_msg = alloc;
        (*tcb).dlerror_pending = 0;
    }
    alloc
}

fn dlerror_buf_len() -> usize {
    DLERROR_BUF_SIZE
}

fn mark_dlerror_result(buf: *mut u8) {
    let Some(tcb) = trona_runtime::thread::tls::current_tls() else {
        return;
    };
    unsafe {
        (*tcb).dlerror_pending = if !buf.is_null() && *buf != 0 { 1 } else { 0 };
    }
}

// ---------------------------------------------------------------------------
// Public C entries
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlopen(filename: *const u8, flags: i32) -> *mut u8 {
    let table = match trona_runtime::loader_dlfcn() {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };
    let buf = dlerror_buf();
    unsafe {
        if !buf.is_null() {
            *buf = 0;
        }
        let ret = (table.dlopen)(filename, flags, buf, dlerror_buf_len());
        mark_dlerror_result(buf);
        ret
    }
}

/// Inner `dlsym` entry that takes the caller PC explicitly. The arch-specific
/// asm trampoline (`__dlsym_capture_pc`) captures `[%rsp]` (x86_64) or `LR`
/// (aarch64) before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __basaltc_dlsym_with_pc(
    handle: *mut u8,
    symbol: *const u8,
    caller_pc: usize,
) -> *mut u8 {
    let table = match trona_runtime::loader_dlfcn() {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };
    let buf = dlerror_buf();
    unsafe {
        if !buf.is_null() {
            *buf = 0;
        }
        let ret = (table.dlsym_from)(handle, symbol, caller_pc, buf, dlerror_buf_len());
        mark_dlerror_result(buf);
        ret
    }
}

/// `dlfunc` differs from dlsym only in the return type (function pointer).
/// The asm trampoline captures caller PC and then tail-calls into here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __basaltc_dlfunc_with_pc(
    handle: *mut u8,
    symbol: *const u8,
    caller_pc: usize,
) -> *mut u8 {
    unsafe { __basaltc_dlsym_with_pc(handle, symbol, caller_pc) }
}

// arch-specific entry points exported by `arch/<arch>/dlfcn_shim.S` —
// declared here so callers in libc see the right signature.
unsafe extern "C" {
    fn __dlsym_capture_pc(handle: *mut u8, symbol: *const u8) -> *mut u8;
    fn __dlfunc_capture_pc(handle: *mut u8, symbol: *const u8) -> Option<unsafe extern "C" fn()>;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(handle: *mut u8, symbol: *const u8) -> *mut u8 {
    unsafe { __dlsym_capture_pc(handle, symbol) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlfunc(
    handle: *mut u8,
    symbol: *const u8,
) -> Option<unsafe extern "C" fn()> {
    unsafe { __dlfunc_capture_pc(handle, symbol) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlclose(handle: *mut u8) -> i32 {
    let table = match trona_runtime::loader_dlfcn() {
        Some(t) => t,
        None => return -1,
    };
    let buf = dlerror_buf();
    unsafe {
        if !buf.is_null() {
            *buf = 0;
        }
        let ret = (table.dlclose)(handle, buf, dlerror_buf_len());
        mark_dlerror_result(buf);
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dladdr(addr: *const u8, info: *mut DlInfo) -> i32 {
    let table = match trona_runtime::loader_dlfcn() {
        Some(t) => t,
        None => return 0,
    };
    let buf = dlerror_buf();
    unsafe {
        if !buf.is_null() {
            *buf = 0;
        }
        let ret = (table.dladdr)(addr, info, buf, dlerror_buf_len());
        mark_dlerror_result(buf);
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dl_iterate_phdr(
    callback: trona_kernel::core_types::DlIterateCallback,
    data: *mut u8,
) -> i32 {
    let table = match trona_runtime::loader_dlfcn() {
        Some(t) => t,
        None => return 0,
    };
    let buf = dlerror_buf();
    unsafe {
        if !buf.is_null() {
            *buf = 0;
        }
        let ret = (table.dl_iterate_phdr)(callback, data, buf, dlerror_buf_len());
        mark_dlerror_result(buf);
        ret
    }
}

/// POSIX `dlerror`: returns the current per-thread error string and clears
/// the pending flag so subsequent calls return null until another dlfcn call
/// sets it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlerror() -> *mut u8 {
    let tls = trona_runtime::thread::tls::current_tls();
    let tcb = match tls {
        Some(p) => p,
        None => return core::ptr::null_mut(),
    };
    let buf = unsafe { (*tcb).dlerror_msg };
    if buf.is_null() || unsafe { (*tcb).dlerror_pending } == 0 {
        return core::ptr::null_mut();
    }
    unsafe {
        (*tcb).dlerror_pending = 0;
        buf
    }
}

/// Suppress the unused-function warning for the marker constants. They are
/// referenced by debugging only.
#[doc(hidden)]
pub const fn _dlfcn_handle_markers() -> (*mut u8, *mut u8) {
    (RTLD_NEXT_HANDLE, DEFAULT_HANDLE_MARKER)
}
