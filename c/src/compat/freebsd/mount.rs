//! BSD `statfs` / `fstatfs` / `getmntinfo` — backed by VFS_MOUNT_LIST.
//! SPDX-License-Identifier: GPL-2.0-only

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use trona_posix::types::{
    TRONA_MOUNT_INFO_FS_TYPE_LEN, TRONA_MOUNT_INFO_PATH_LEN, TronaMountInfo, TronaStatvfs,
};
use trona_posix::{posix_fstatvfs, posix_mount, posix_mount_list, posix_remount, posix_statvfs};
use trona_protocol::posix::{
    MNT_BIND, MNT_NOATIME, MNT_NODEV, MNT_NOEXEC, MNT_NOSUID, MNT_RBIND, MNT_RDONLY,
};

use crate::errno;

/// Linux `MS_*` flag word mirrors. Match `lib/basalt/c/include/sys/mount.h`
/// and the canonical Linux values so glibc-shaped callers see no
/// surprises. The internal `MNT_*` wire flags (defined in
/// `trona_protocol::posix`) use a different bit layout —
/// `linux_ms_to_mnt` is the conversion seam.
const MS_RDONLY: u64 = 0x1;
const MS_NOSUID: u64 = 0x2;
const MS_NODEV: u64 = 0x4;
const MS_NOEXEC: u64 = 0x8;
const MS_REMOUNT: u64 = 0x20;
const MS_NOATIME: u64 = 0x400;
const MS_BIND: u64 = 0x1000;
const MS_REC: u64 = 0x4000;

/// BSD `MNT_*` bits exposed via `struct statfs.f_flags`. Match the
/// definitions in `lib/basalt/c/include/sys/mount.h`. These differ
/// from the SaltyOS internal flag layout above (notably NOSUID is
/// 0x08 in BSD vs 0x02 internally).
const BSD_MNT_RDONLY: u64 = 0x01;
const BSD_MNT_NOEXEC: u64 = 0x04;
const BSD_MNT_NOSUID: u64 = 0x08;
const BSD_MNT_LOCAL: u64 = 0x1000;

/// Render the SaltyOS internal `MNT_*` snapshot we get back from VFS
/// into the BSD-shaped `MNT_*` set the C ABI advertises through
/// `struct statfs.f_flags`. Bits without a BSD analogue (CASEFOLD,
/// NOATIME, BIND, …) are dropped — they are SaltyOS-specific and not
/// part of the BSD ABI surface.
fn mnt_to_bsd(internal: u32) -> u64 {
    let mut out: u64 = 0;
    if internal & MNT_RDONLY != 0 {
        out |= BSD_MNT_RDONLY;
    }
    if internal & MNT_NOSUID != 0 {
        out |= BSD_MNT_NOSUID;
    }
    if internal & MNT_NOEXEC != 0 {
        out |= BSD_MNT_NOEXEC;
    }
    // Every SaltyOS mount today is local-storage backed; surface the
    // BSD MNT_LOCAL bit so callers (autofs / statfs filters) treat
    // them consistently.
    out |= BSD_MNT_LOCAL;
    out
}

/// Translate Linux `MS_*` flags into SaltyOS internal `MNT_*` flags.
/// `MS_REMOUNT` is intentionally NOT translated — the call site
/// strips it before invoking us so the remaining bits describe the
/// new state, not the operation.
fn linux_ms_to_mnt(ms_flags: u64) -> u32 {
    let mut out: u32 = 0;
    if ms_flags & MS_RDONLY != 0 {
        out |= MNT_RDONLY;
    }
    if ms_flags & MS_NOSUID != 0 {
        out |= MNT_NOSUID;
    }
    if ms_flags & MS_NODEV != 0 {
        out |= MNT_NODEV;
    }
    if ms_flags & MS_NOEXEC != 0 {
        out |= MNT_NOEXEC;
    }
    if ms_flags & MS_NOATIME != 0 {
        out |= MNT_NOATIME;
    }
    if ms_flags & MS_BIND != 0 {
        out |= if ms_flags & MS_REC != 0 {
            MNT_RBIND
        } else {
            MNT_BIND
        };
    }
    out
}

/// Render the mount option string (`getmntent`'s `mnt_opts`) from the
/// internal `MNT_*` flag set, writing into `out` and returning the
/// byte length written (no NUL terminator). The flag constants are
/// private to this module, so this is the single rendering seam — the
/// VFS server leaves the wire `opts` empty and ships only the flags.
pub(crate) fn mnt_opts_string(flags: u32, out: &mut [u8]) -> usize {
    fn append(out: &mut [u8], n: &mut usize, s: &[u8]) {
        let take = s.len().min(out.len().saturating_sub(*n));
        out[*n..*n + take].copy_from_slice(&s[..take]);
        *n += take;
    }
    let mut n = 0usize;
    if flags & MNT_RDONLY != 0 {
        append(out, &mut n, b"ro");
    } else {
        append(out, &mut n, b"rw");
    }
    if flags & MNT_NOSUID != 0 {
        append(out, &mut n, b",nosuid");
    }
    if flags & MNT_NODEV != 0 {
        append(out, &mut n, b",nodev");
    }
    if flags & MNT_NOEXEC != 0 {
        append(out, &mut n, b",noexec");
    }
    if flags & MNT_NOATIME != 0 {
        append(out, &mut n, b",noatime");
    }
    n
}

/// Maximum NUL-terminated string length we pull from C buffers when
/// packing the mount IPC. The VFS server enforces tighter per-field
/// caps (target ≤ 128, fstype ≤ 16, opts ≤ 128); these limits exist so
/// a missing terminator cannot drag us into unreadable memory.
const MOUNT_TARGET_MAX: usize = 128;
const MOUNT_FSTYPE_MAX: usize = 16;
const MOUNT_OPTS_MAX: usize = 128;

const MFSNAMELEN: usize = 16;
const MNAMELEN: usize = 88;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Statfs {
    pub f_version: u32,
    pub f_type: u32,
    pub f_flags: u64,
    pub f_bsize: u64,
    pub f_iosize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: i64,
    pub f_files: u64,
    pub f_ffree: i64,
    pub f_syncwrites: u64,
    pub f_asyncwrites: u64,
    pub f_syncreads: u64,
    pub f_asyncreads: u64,
    pub f_namemax: u64,
    pub f_owner: u32,
    pub f_fsid: i32,
    pub f_fstypename: [u8; MFSNAMELEN],
    pub f_mntfromname: [u8; MNAMELEN],
    pub f_mntonname: [u8; MNAMELEN],
}

impl Statfs {
    pub const fn zeroed() -> Self {
        Self {
            f_version: 0,
            f_type: 0,
            f_flags: 0,
            f_bsize: 0,
            f_iosize: 0,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_syncwrites: 0,
            f_asyncwrites: 0,
            f_syncreads: 0,
            f_asyncreads: 0,
            f_namemax: 0,
            f_owner: 0,
            f_fsid: 0,
            f_fstypename: [0; MFSNAMELEN],
            f_mntfromname: [0; MNAMELEN],
            f_mntonname: [0; MNAMELEN],
        }
    }
}

/// Persistent `struct statfs` buffer handed back from `getmntinfo`.
/// Like FreeBSD's internal static, it is owned by libc, reused
/// across calls, and grown (never shrunk) to fit the active mount
/// count. Not thread-safe — `getmntinfo(3)` never was.
static GETMNTINFO_BUF_PTR: AtomicUsize = AtomicUsize::new(0);
static GETMNTINFO_BUF_CAP: AtomicU32 = AtomicU32::new(0);

/// Grow [`GETMNTINFO_BUF_PTR`] to hold at least `n` records, keeping
/// any existing buffer. Returns null on allocation failure.
unsafe fn ensure_getmntinfo_buf(n: usize) -> *mut Statfs {
    let cur_ptr = GETMNTINFO_BUF_PTR.load(Ordering::Acquire) as *mut Statfs;
    let cur_cap = GETMNTINFO_BUF_CAP.load(Ordering::Acquire) as usize;
    if !cur_ptr.is_null() && cur_cap >= n {
        return cur_ptr;
    }
    let bytes = n.saturating_mul(core::mem::size_of::<Statfs>());
    let new_ptr = if cur_ptr.is_null() {
        unsafe { crate::malloc::malloc(bytes) }
    } else {
        unsafe { crate::malloc::realloc(cur_ptr as *mut u8, bytes) }
    } as *mut Statfs;
    if new_ptr.is_null() {
        return core::ptr::null_mut();
    }
    GETMNTINFO_BUF_PTR.store(new_ptr as usize, Ordering::Release);
    GETMNTINFO_BUF_CAP.store(n as u32, Ordering::Release);
    new_ptr
}

fn convert(info: &TronaMountInfo, out: &mut Statfs) {
    *out = Statfs::zeroed();
    out.f_flags = mnt_to_bsd(info.flags);
    out.f_fsid = info.statvfs.f_fsid as i32;
    out.f_bsize = info.statvfs.f_bsize;
    out.f_iosize = info.statvfs.f_frsize;
    out.f_blocks = info.statvfs.f_blocks;
    out.f_bfree = info.statvfs.f_bfree;
    out.f_bavail = info.statvfs.f_bavail as i64;
    out.f_files = info.statvfs.f_files;
    out.f_ffree = info.statvfs.f_ffree as i64;
    out.f_namemax = info.statvfs.f_namemax;

    let fs_type_len = (info.fs_type_len as usize).min(TRONA_MOUNT_INFO_FS_TYPE_LEN);
    let path_len = (info.mount_path_len as usize).min(TRONA_MOUNT_INFO_PATH_LEN);
    let copy_fs = fs_type_len.min(MFSNAMELEN - 1);
    let copy_path = path_len.min(MNAMELEN - 1);
    out.f_fstypename[..copy_fs].copy_from_slice(&info.fs_type[..copy_fs]);
    out.f_mntfromname[..copy_fs].copy_from_slice(&info.fs_type[..copy_fs]);
    out.f_mntonname[..copy_path].copy_from_slice(&info.mount_path[..copy_path]);
}

fn fill_from_statvfs(buf: &mut Statfs, st: &TronaStatvfs) {
    buf.f_bsize = st.f_bsize;
    buf.f_iosize = st.f_frsize;
    buf.f_blocks = st.f_blocks;
    buf.f_bfree = st.f_bfree;
    buf.f_bavail = st.f_bavail as i64;
    buf.f_files = st.f_files;
    buf.f_ffree = st.f_ffree as i64;
    buf.f_namemax = st.f_namemax;
    // `TronaStatvfs.f_flag` carries internal MNT_* bits in its low
    // 32 bits (see `tmpfs::tmpfs_statfs`). Translate to the BSD
    // ABI shape before exposing on `struct statfs.f_flags`.
    buf.f_flags = mnt_to_bsd(st.f_flag as u32);
    buf.f_fsid = st.f_fsid as i32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn statfs(path: *const u8, buf: *mut Statfs) -> i32 {
    if buf.is_null() || path.is_null() {
        errno::set_errno(errno::EINVAL);
        return -1;
    }
    let mut sv = TronaStatvfs::zeroed();
    let rc = unsafe { posix_statvfs(path, &raw mut sv) };
    if rc < 0 {
        errno::set_errno(-rc);
        return -1;
    }
    unsafe {
        *buf = Statfs::zeroed();
        fill_from_statvfs(&mut *buf, &sv);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fstatfs(fd: i32, buf: *mut Statfs) -> i32 {
    if buf.is_null() {
        errno::set_errno(errno::EINVAL);
        return -1;
    }
    let mut sv = TronaStatvfs::zeroed();
    let rc = unsafe { posix_fstatvfs(fd, &raw mut sv) };
    if rc < 0 {
        errno::set_errno(-rc);
        return -1;
    }
    unsafe {
        *buf = Statfs::zeroed();
        fill_from_statvfs(&mut *buf, &sv);
    }
    0
}

/// Walk a NUL-terminated C string up to `max` bytes and return the
/// resulting byte slice. Returns `None` on a NULL pointer.
unsafe fn cstr_to_slice<'a>(ptr: *const u8, max: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let mut len = 0usize;
        while len < max && *ptr.add(len) != 0 {
            len += 1;
        }
        Some(core::slice::from_raw_parts(ptr, len))
    }
}

/// Linux `mount(2)` / SaltyOS unified mount ABI. `MS_REMOUNT` routes
/// to the VFS remount path; everything else creates a fresh mount.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mount(
    _source: *const u8,
    target: *const u8,
    fstype: *const u8,
    mountflags: u64,
    data: *const u8,
) -> i32 {
    let target_slice = match unsafe { cstr_to_slice(target, MOUNT_TARGET_MAX) } {
        Some(s) if !s.is_empty() => s,
        _ => {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
    };
    let opts_slice = match unsafe { cstr_to_slice(data, MOUNT_OPTS_MAX) } {
        Some(s) => s,
        None => &[],
    };
    if (mountflags & MS_REMOUNT) != 0 {
        // MS_REMOUNT is the path selector; strip it before translating
        // the rest of the bits into internal MNT_* form so the
        // backend remount callback sees the correct new-state flags.
        let remount_flags = linux_ms_to_mnt(mountflags & !MS_REMOUNT);
        let rc = unsafe { posix_remount(target_slice, remount_flags, opts_slice) };
        if rc < 0 {
            errno::set_errno(-rc);
            return -1;
        }
        return 0;
    }
    let fstype_slice = match unsafe { cstr_to_slice(fstype, MOUNT_FSTYPE_MAX) } {
        Some(s) if !s.is_empty() => s,
        _ => {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
    };
    let mnt_flags = linux_ms_to_mnt(mountflags);
    let rc = unsafe { posix_mount(target_slice, fstype_slice, mnt_flags, opts_slice) };
    if rc < 0 {
        errno::set_errno(-rc);
        return -1;
    }
    0
}

/// Linux `umount(2)`. We don't yet honor unmount flags beyond the
/// "lazy" / "force" semantics the VFS server inherently rejects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn umount(target: *const u8) -> i32 {
    unsafe { umount2(target, 0) }
}

/// Linux `umount2(target, flags)`. Currently delegates to `posix_umount`
/// with the flags forwarded as-is so the server can later honor MNT_FORCE
/// and friends. The basalt entrypoint exists so glibc-shaped callers
/// (e.g. mount.cifs ports) link cleanly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn umount2(target: *const u8, flags: i32) -> i32 {
    let target_slice = match unsafe { cstr_to_slice(target, MOUNT_TARGET_MAX) } {
        Some(s) if !s.is_empty() => s,
        _ => {
            errno::set_errno(errno::EINVAL);
            return -1;
        }
    };
    let rc = unsafe { trona_posix::posix_umount(target_slice, flags as u32) };
    if rc < 0 {
        errno::set_errno(-rc);
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getmntinfo(mntbufp: *mut *mut Statfs, _mode: i32) -> i32 {
    // FreeBSD 2-pass: count the active mounts, size a buffer to the
    // total, then fill it — no fixed entry cap.
    let available = unsafe { posix_mount_list(&mut []) };
    if available < 0 {
        errno::set_errno(-available);
        return 0;
    }
    let n = available as usize;
    if n == 0 {
        if !mntbufp.is_null() {
            unsafe { *mntbufp = core::ptr::null_mut() };
        }
        return 0;
    }

    let dst = unsafe { ensure_getmntinfo_buf(n) };
    if dst.is_null() {
        errno::set_errno(errno::ENOMEM);
        return 0;
    }

    // Stage the raw records in a temporary heap buffer, convert into
    // the persistent `struct statfs` array, then release the staging.
    let bytes = n * core::mem::size_of::<TronaMountInfo>();
    let tmp = unsafe { crate::malloc::malloc(bytes) } as *mut TronaMountInfo;
    if tmp.is_null() {
        errno::set_errno(errno::ENOMEM);
        return 0;
    }
    unsafe { core::ptr::write_bytes(tmp as *mut u8, 0, bytes) };
    let infos = unsafe { core::slice::from_raw_parts_mut(tmp, n) };
    let written = unsafe { posix_mount_list(infos) };
    if written < 0 {
        unsafe { crate::malloc::free(tmp as *mut u8) };
        errno::set_errno(-written);
        return 0;
    }
    let count = (written as usize).min(n);
    for idx in 0..count {
        // Build on the stack and `write` into the slot: the persistent
        // buffer is malloc/realloc'd and uninitialised, so forming a
        // `&mut Statfs` over it before initialisation would be UB.
        let mut sf = Statfs::zeroed();
        convert(&infos[idx], &mut sf);
        unsafe { dst.add(idx).write(sf) };
    }
    unsafe { crate::malloc::free(tmp as *mut u8) };

    if !mntbufp.is_null() {
        unsafe { *mntbufp = dst };
    }
    count as i32
}
