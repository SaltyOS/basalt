//! saltyc — SaltyOS C Standard Library
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Rust implementation of a C standard library for SaltyOS. Provides POSIX
//! and BSD-compatible C functions for ported userland programs (FreeBSD ls,
//! cat, etc.). All public functions use `#[unsafe(no_mangle)] pub extern "C"`
//! for C ABI compatibility and are linked into executables via `libsalty.so`.
//!
//! System operations (file I/O, memory management, process control) are
//! delegated to `libsalty`, which communicates with kernel services via IPC.
//! Core subsystems (malloc, errno) are thread-safe via spinlocks and TLS.
//! stdio FILE operations are not yet fully locked.

#![no_std]
#![no_main]
#![no_builtins]
#![allow(internal_features)]
#![feature(c_variadic)]
#![feature(linkage)]

extern crate salty;

pub mod arch;
pub mod crt;
pub mod ctype;
pub mod env;
pub mod errno;
pub mod malloc;
pub mod mem;
pub mod string;
pub mod stdio;
pub mod unistd;
pub mod process;
pub mod signal;
pub mod dirent;
pub mod jobctl;
pub mod ioctl;
pub mod termios;
pub mod termcap;
pub mod regex;
pub mod time;
pub mod stdlib;
pub mod wchar;
pub mod sysinfo;
pub mod pwd;
pub mod locale;
pub mod glob;
pub mod select;
pub mod math;
pub mod misc;
pub mod pthread;
pub mod search;
pub mod dlfcn;
pub mod fts;
pub mod compat;
pub mod socket;
pub mod netif;
pub mod inet;
pub mod getopt;
pub mod getrandom;
pub mod iconv;
pub mod stack_protector;

// Panic handler is provided by libsalty (our dependency)
