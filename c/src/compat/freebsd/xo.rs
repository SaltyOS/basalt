//! libxo text-mode stub for SaltyOS
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Implements a subset of FreeBSD's libxo sufficient for utilities like `wc`
//! that use `xo_emit()` for output. Only text display mode is supported
//! (no JSON/XML).
//!
//! Handle support: `xo_create_to_file()` stores a `FILE*` as the handle.
//! `xo_emit_h()` outputs to that `FILE*` instead of stdout when non-NULL.

use core::ffi::VaList;

use crate::stdio::FILE;

// ---------------------------------------------------------------------------
// Internal: format string extraction
// ---------------------------------------------------------------------------

/// Parse an XO format string, extract printf format specifiers, and print.
///
/// XO format: literal text mixed with `{[role]:name[/printf-format]}` fields.
/// For text display we output literal text as-is and for fields that have a
/// `/format` portion we output using that printf format with the corresponding
/// vararg. Fields without `/format` are skipped (structural-only).
unsafe fn xo_emit_core(fp: *mut FILE, fmt: *const u8, ap: VaList<'_>) -> i32 {
    const BUF_SIZE: usize = 2048;
    let mut pfmt = [0u8; BUF_SIZE];
    let mut pi: usize = 0;

    unsafe {
        let mut p = fmt;

        while *p != 0 && pi < BUF_SIZE - 2 {
            if *p == b'{' {
                p = p.add(1); // skip '{'

                // Find '/' and '}' within this field
                let mut slash: *const u8 = core::ptr::null();
                let mut end = p;
                while *end != 0 && *end != b'}' {
                    if *end == b'/' && slash.is_null() {
                        slash = end;
                    }
                    end = end.add(1);
                }

                if !slash.is_null() {
                    // Copy printf format between '/' and '}'
                    let mut f = slash.add(1);
                    while f < end && pi < BUF_SIZE - 2 {
                        pfmt[pi] = *f;
                        pi += 1;
                        f = f.add(1);
                    }
                }
                // else: field without format, skip (no vararg consumed)

                p = if *end == b'}' { end.add(1) } else { end };
            } else {
                pfmt[pi] = *p;
                pi += 1;
                p = p.add(1);
            }
        }
        pfmt[pi] = 0;

        crate::stdio::vfprintf(fp, pfmt.as_ptr(), ap)
    }
}

// ---------------------------------------------------------------------------
// Structural functions — no-ops for text mode
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_list(_name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_list(_name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_instance(_name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_instance(_name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_container(_name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_container(_name: *const u8) {}

// ---------------------------------------------------------------------------
// Handle-based structural functions — delegate to non-handle versions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_list_h(_xop: *mut u8, _name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_list_h(_xop: *mut u8, _name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_instance_h(_xop: *mut u8, _name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_instance_h(_xop: *mut u8, _name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_open_container_h(_xop: *mut u8, _name: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_close_container_h(_xop: *mut u8, _name: *const u8) {}

// ---------------------------------------------------------------------------
// Configuration — no-ops for text mode
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_set_flags(_xop: *mut u8, _flags: i32) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_no_setlocale() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_set_version(_ver: *const u8) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_set_program(_prog: *const u8) {}

/// Parse XO-specific arguments from argv. Returns argc unchanged since
/// text-only mode has no special arguments to consume.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_parse_args(argc: i32, _argv: *mut *mut u8) -> i32 {
    argc
}

// ---------------------------------------------------------------------------
// Handle management
// ---------------------------------------------------------------------------

/// Create an XO handle for the given FILE*. In text mode the handle is simply
/// the FILE* cast to `*mut u8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_create_to_file(fp: *mut FILE, _style: i32, _flags: i32) -> *mut u8 {
    fp as *mut u8
}

/// Destroy an XO handle. No-op in text mode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_destroy(_xop: *mut u8) {}

// ---------------------------------------------------------------------------
// Finish / flush
// ---------------------------------------------------------------------------

/// Flush stdout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_finish() -> i32 {
    unsafe { crate::stdio::fflush(crate::stdio::stdout) }
}

/// Flush the handle's FILE*, or stdout if handle is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_finish_h(xop: *mut u8) -> i32 {
    unsafe {
        if !xop.is_null() {
            crate::stdio::fflush(xop as *mut FILE)
        } else {
            crate::stdio::fflush(crate::stdio::stdout)
        }
    }
}

// ---------------------------------------------------------------------------
// Emit — output formatted text
// ---------------------------------------------------------------------------

/// Format and print to stdout, parsing XO field specifiers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_emit(fmt: *const u8, args: ...) -> i32 {
    unsafe { xo_emit_core(crate::stdio::stdout, fmt, args) }
}

/// Format and print to handle's FILE* (or stdout if null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_emit_h(xop: *mut u8, fmt: *const u8, args: ...) -> i32 {
    unsafe {
        let fp = if !xop.is_null() {
            xop as *mut FILE
        } else {
            crate::stdio::stdout
        };
        xo_emit_core(fp, fmt, args)
    }
}

// ---------------------------------------------------------------------------
// Error / warning wrappers — forward to BSD err(3) functions
// ---------------------------------------------------------------------------

/// Print error message and exit. Forwards to `verr()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_err(eval: i32, fmt: *const u8, args: ...) -> ! {
    unsafe { crate::compat::freebsd::bsd_err::verr(eval, fmt, args) }
}

/// Print error message (no errno) and exit. Forwards to `verrx()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_errx(eval: i32, fmt: *const u8, args: ...) -> ! {
    unsafe { crate::compat::freebsd::bsd_err::verrx(eval, fmt, args) }
}

/// Print warning message. Forwards to `vwarn()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_warn(fmt: *const u8, args: ...) {
    unsafe { crate::compat::freebsd::bsd_err::vwarn(fmt, args) }
}

/// Print warning message (no errno). Forwards to `vwarnx()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_warnx(fmt: *const u8, args: ...) {
    unsafe { crate::compat::freebsd::bsd_err::vwarnx(fmt, args) }
}

/// Print formatted message to stderr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xo_error(fmt: *const u8, args: ...) {
    unsafe { crate::stdio::vfprintf(crate::stdio::stderr, fmt, args) };
}
