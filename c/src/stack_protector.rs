//! Stack smashing protector support
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Provides `__stack_chk_guard` and `__stack_chk_fail` for
//! `-fstack-protector-strong` compiled C code.

/// Stack canary value. A fixed non-zero value with a null byte to
/// terminate string copies that might overflow.
#[unsafe(no_mangle)]
pub static __stack_chk_guard: u64 = 0x00000aff0a0d0000;

/// Called by compiler-inserted stack protector epilogue when the
/// canary has been overwritten (buffer overflow detected).
#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail() -> ! {
    salty::serial::serial_puts(b"*** stack smashing detected ***\n");
    unsafe { salty::posix::posix_exit(139) };
}
