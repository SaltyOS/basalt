//! getmntent / setmntent / endmntent — backed by VFS_MOUNT_LIST.
//!
//! `setmntent` snapshots the mount table into a heap buffer grown to
//! the active mount count (no fixed cap); `getmntent` walks the
//! cursor; `endmntent` resets it. The returned `struct mntent`
//! strings live in a per-entry string arena so successive
//! `getmntent` calls return stable pointers until the next
//! `setmntent`. Not thread-safe — neither is glibc's `getmntent(3)`.
//!
//! SPDX-License-Identifier: GPL-2.0-only

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use trona_posix::posix_mount_list;
use trona_posix::types::{
    TRONA_MOUNT_INFO_FS_TYPE_LEN, TRONA_MOUNT_INFO_OPTS_LEN, TRONA_MOUNT_INFO_PATH_LEN,
    TronaMountInfo,
};

const MNT_OPTS_BUF: usize = TRONA_MOUNT_INFO_OPTS_LEN + 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CMntEnt {
    mnt_fsname: *mut u8,
    mnt_dir: *mut u8,
    mnt_type: *mut u8,
    mnt_opts: *mut u8,
    mnt_freq: i32,
    mnt_passno: i32,
}

struct EntryStorage {
    fs_type: [u8; TRONA_MOUNT_INFO_FS_TYPE_LEN + 1],
    mount_path: [u8; TRONA_MOUNT_INFO_PATH_LEN + 1],
    opts: [u8; MNT_OPTS_BUF],
}

impl EntryStorage {
    const fn zeroed() -> Self {
        Self {
            fs_type: [0; TRONA_MOUNT_INFO_FS_TYPE_LEN + 1],
            mount_path: [0; TRONA_MOUNT_INFO_PATH_LEN + 1],
            opts: [0; MNT_OPTS_BUF],
        }
    }
}

const ZERO_ENTRY: CMntEnt = CMntEnt {
    mnt_fsname: core::ptr::null_mut(),
    mnt_dir: core::ptr::null_mut(),
    mnt_type: core::ptr::null_mut(),
    mnt_opts: core::ptr::null_mut(),
    mnt_freq: 0,
    mnt_passno: 0,
};

/// Heap snapshot of the mount table, grown (never shrunk) to the
/// active mount count. The parallel arrays are owned by libc and
/// reused across `setmntent` calls.
static MNTENT_ENTRIES_PTR: AtomicUsize = AtomicUsize::new(0);
static MNTENT_STORAGE_PTR: AtomicUsize = AtomicUsize::new(0);
static MNTENT_CAP: AtomicU32 = AtomicU32::new(0);
static MNTENT_COUNT: AtomicU32 = AtomicU32::new(0);
static MNTENT_CURSOR: AtomicU32 = AtomicU32::new(0);

unsafe fn grow(cur: *mut u8, bytes: usize) -> *mut u8 {
    if cur.is_null() {
        unsafe { crate::malloc::malloc(bytes) }
    } else {
        unsafe { crate::malloc::realloc(cur, bytes) }
    }
}

/// Grow the parallel `CMntEnt` / `EntryStorage` arrays to hold at
/// least `n` entries. Returns null pointers on allocation failure.
unsafe fn ensure_mntent_buffers(n: usize) -> (*mut CMntEnt, *mut EntryStorage) {
    let cur_e = MNTENT_ENTRIES_PTR.load(Ordering::Acquire) as *mut CMntEnt;
    let cur_s = MNTENT_STORAGE_PTR.load(Ordering::Acquire) as *mut EntryStorage;
    let cur_cap = MNTENT_CAP.load(Ordering::Acquire) as usize;
    if !cur_e.is_null() && !cur_s.is_null() && cur_cap >= n {
        return (cur_e, cur_s);
    }
    let e_bytes = n.saturating_mul(core::mem::size_of::<CMntEnt>());
    let s_bytes = n.saturating_mul(core::mem::size_of::<EntryStorage>());
    let new_e = unsafe { grow(cur_e as *mut u8, e_bytes) } as *mut CMntEnt;
    if new_e.is_null() {
        return (core::ptr::null_mut(), core::ptr::null_mut());
    }
    MNTENT_ENTRIES_PTR.store(new_e as usize, Ordering::Release);
    let new_s = unsafe { grow(cur_s as *mut u8, s_bytes) } as *mut EntryStorage;
    if new_s.is_null() {
        return (core::ptr::null_mut(), core::ptr::null_mut());
    }
    MNTENT_STORAGE_PTR.store(new_s as usize, Ordering::Release);
    MNTENT_CAP.store(n as u32, Ordering::Release);
    (new_e, new_s)
}

fn copy_field(dst: &mut [u8], src: &[u8]) -> *mut u8 {
    let len = src.len().min(dst.len() - 1);
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
    dst.as_mut_ptr()
}

/// Fill `entries` / `storage` (sized `>= infos.len()`) from the
/// snapshot. The malloc'd slots are written before any reference is
/// taken, since the region is uninitialised.
unsafe fn populate(infos: &[TronaMountInfo], entries: *mut CMntEnt, storage: *mut EntryStorage) {
    let count = infos.len();
    unsafe {
        for idx in 0..count {
            let info = &infos[idx];
            storage.add(idx).write(EntryStorage::zeroed());
            entries.add(idx).write(ZERO_ENTRY);
            let store = &mut *storage.add(idx);
            let entry = &mut *entries.add(idx);
            let fs_ptr = copy_field(&mut store.fs_type, info.fs_type_slice());
            entry.mnt_fsname = fs_ptr;
            entry.mnt_dir = copy_field(&mut store.mount_path, info.mount_path_slice());
            entry.mnt_type = fs_ptr;
            // The server ships only flags; render the option text from
            // the internal `MNT_*` set (single seam in `mount.rs`).
            let opts_len =
                super::mount::mnt_opts_string(info.flags, &mut store.opts[..MNT_OPTS_BUF - 1]);
            store.opts[opts_len] = 0;
            entry.mnt_opts = store.opts.as_mut_ptr();
            entry.mnt_freq = 0;
            entry.mnt_passno = 0;
        }
    }
    MNTENT_COUNT.store(count as u32, Ordering::Release);
    MNTENT_CURSOR.store(0, Ordering::Release);
}

fn reset_table() {
    MNTENT_COUNT.store(0, Ordering::Release);
    MNTENT_CURSOR.store(0, Ordering::Release);
}

fn refresh() {
    // FreeBSD/getfsstat 2-pass: count, grow, then fill.
    let available = unsafe { posix_mount_list(&mut []) };
    if available <= 0 {
        reset_table();
        return;
    }
    let n = available as usize;
    let (entries, storage) = unsafe { ensure_mntent_buffers(n) };
    if entries.is_null() || storage.is_null() {
        reset_table();
        return;
    }
    let bytes = n * core::mem::size_of::<TronaMountInfo>();
    let tmp = unsafe { crate::malloc::malloc(bytes) } as *mut TronaMountInfo;
    if tmp.is_null() {
        reset_table();
        return;
    }
    unsafe { core::ptr::write_bytes(tmp as *mut u8, 0, bytes) };
    let infos = unsafe { core::slice::from_raw_parts_mut(tmp, n) };
    let written = unsafe { posix_mount_list(infos) };
    if written < 0 {
        unsafe { crate::malloc::free(tmp as *mut u8) };
        reset_table();
        return;
    }
    let count = (written as usize).min(n);
    unsafe { populate(&infos[..count], entries, storage) };
    unsafe { crate::malloc::free(tmp as *mut u8) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setmntent(_filename: *const u8, _typ: *const u8) -> *mut u8 {
    refresh();
    1usize as *mut u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getmntent(_stream: *mut u8) -> *mut CMntEnt {
    let count = MNTENT_COUNT.load(Ordering::Acquire);
    let idx = MNTENT_CURSOR.fetch_add(1, Ordering::AcqRel);
    if idx >= count {
        return core::ptr::null_mut();
    }
    let entries = MNTENT_ENTRIES_PTR.load(Ordering::Acquire) as *mut CMntEnt;
    if entries.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { entries.add(idx as usize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn endmntent(_stream: *mut u8) -> i32 {
    MNTENT_CURSOR.store(0, Ordering::Release);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hasmntopt(_mnt: *const u8, _opt: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}
