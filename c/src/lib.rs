//! basaltc — SaltyOS C Standard Library
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Rust implementation of a C standard library for SaltyOS. Provides POSIX
//! and BSD-compatible C functions for ported userland programs (FreeBSD ls,
//! cat, etc.). All public functions use `#[unsafe(no_mangle)] pub extern "C"`
//! for C ABI compatibility and are linked into executables via `libtrona.so`.
//!
//! System operations (file I/O, memory management, process control) are
//! delegated to `libtrona`, which communicates with kernel services via IPC.
//! Core subsystems (malloc, errno) are thread-safe via spinlocks and TLS.
//! stdio FILE operations are not yet fully locked.

#![no_std]
#![no_main]
#![no_builtins]
#![allow(internal_features)]
#![feature(c_variadic)]

extern crate trona_kernel;
extern crate trona_posix;
extern crate trona_protocol;
extern crate trona_runtime;
extern crate trona_server;

pub mod arch;
pub mod compat;
pub mod crt;
pub mod crypt;
pub mod ctype;
pub mod dirent;
pub mod dlfcn;
pub mod env;
pub mod errno;
pub mod fts;
pub mod getopt;
pub mod getrandom;
pub mod glob;
pub mod iconv;
pub mod inet;
pub mod ioctl;
pub mod jobctl;
pub mod locale;
pub mod malloc;
pub mod math;
pub mod mem;
pub mod misc;
pub mod netif;
pub mod process;
pub mod pthread;
pub mod pty;
pub mod pwd;
pub mod regex;
pub mod search;
pub mod select;
pub mod sha512;
pub mod signal;
pub mod socket;
pub mod stack_protector;
pub mod stdio;
pub mod stdlib;
pub mod string;
pub mod sysinfo;
pub mod termcap;
pub mod termios;
pub mod time;
pub mod ttyent;
pub mod unistd;
pub mod wchar;

// Panic handler is provided by libtrona (our dependency)
