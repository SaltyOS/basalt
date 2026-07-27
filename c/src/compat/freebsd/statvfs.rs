//! statvfs / fstatvfs — POSIX `statvfs(3)` C ABI shim.
//!
//! Layout matches `trona_posix::types::TronaStatvfs` so the trona
//! wrapper writes directly into the caller's buffer with no
//! intermediate copy. SPDX-License-Identifier: GPL-2.0-only

use trona_posix::types::TronaStatvfs;
use trona_posix::{posix_fstatvfs, posix_statvfs};

#[repr(C)]
pub struct Statvfs {
    pub f_bsize: u64,
    pub f_frsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_favail: u64,
    pub f_fsid: u64,
    pub f_flag: u64,
    pub f_namemax: u64,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn statvfs(path: *const u8, buf: *mut Statvfs) -> i32 {
    let trona_buf = buf as *mut TronaStatvfs;
    let rc = unsafe { posix_statvfs(path, trona_buf) };
    if rc < 0 {
        crate::errno::set_errno(-rc);
        -1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fstatvfs(fd: i32, buf: *mut Statvfs) -> i32 {
    let trona_buf = buf as *mut TronaStatvfs;
    let rc = unsafe { posix_fstatvfs(fd, trona_buf) };
    if rc < 0 {
        crate::errno::set_errno(-rc);
        -1
    } else {
        0
    }
}
