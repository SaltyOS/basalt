//! FreeBSD stdio internals
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! __swbuf and __srget are called by FreeBSD's putc()/getc() macros
//! when the stdio buffer is full/empty.

/// FreeBSD putc() macro calls __swbuf when the buffer is full.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __swbuf(c: i32, f: *mut crate::stdio::FILE) -> i32 {
    unsafe { crate::stdio::fputc(c, f) }
}

/// FreeBSD getc() macro calls __srget when the read buffer is empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __srget(stream: *mut u8) -> i32 {
    unsafe { crate::stdio::fgetc(stream.cast::<crate::stdio::FILE>()) }
}

// FreeBSD libc internal aliases — used by contrib/ sources that call the
// underscore-prefixed POSIX wrappers instead of the public names.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _open(path: *const u8, flags: i32, mode: u32) -> i32 {
    unsafe { trona_posix::posix_open(path, flags, mode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _read(fd: i32, buf: *mut u8, count: u64) -> i64 {
    unsafe { trona_posix::posix_read(fd, buf, count) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _close(fd: i32) -> i32 {
    unsafe { trona_posix::posix_close(fd) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _write(fd: i32, buf: *const u8, count: u64) -> i64 {
    unsafe { trona_posix::posix_write(fd, buf, count) }
}

/// FreeBSD `__error()` — returns pointer to thread-local errno.
/// On FreeBSD, errno is `*__error()` not `*__errno_location()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __error() -> *mut i32 {
    unsafe { crate::errno::__errno_location() }
}
