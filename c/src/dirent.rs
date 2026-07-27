//! Directory operations
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Implements `opendir`/`readdir`/`closedir` using a static pool of 16 `DIR`
//! entries. Each `DIR` wraps a POSIX file descriptor obtained from
//! `posix_open` with `O_DIRECTORY`. Directory entries are read one at a time
//! via `posix_getdents`.

use crate::errno;

/// Directory entry returned by `readdir()`.
#[repr(C)]
pub struct Dirent {
    /// Inode number.
    pub d_ino: u64,
    /// Offset to the next directory entry (opaque).
    pub d_off: i64,
    /// Length of this record in bytes.
    pub d_reclen: u16,
    /// File type: `DT_REG` (8), `DT_DIR` (4), `DT_LNK` (10), `DT_UNKNOWN` (0).
    pub d_type: u8,
    /// ABI padding/alignment field (matches FreeBSD dirent layout).
    pub d_pad0: u8,
    /// Length of string in d_name.
    pub d_namlen: u16,
    /// ABI padding/alignment field (matches FreeBSD dirent layout).
    pub d_pad1: u16,
    /// Null-terminated filename (max 255 characters + NUL).
    pub d_name: [u8; 256],
}

impl Dirent {
    const fn zeroed() -> Self {
        Dirent {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type: 0,
            d_pad0: 0,
            d_namlen: 0,
            d_pad1: 0,
            d_name: [0; 256],
        }
    }
}

pub struct DIR {
    fd: i32,
    entry: Dirent,
    has_entry: bool,
    pos: i64,
}

impl DIR {
    const fn zeroed() -> Self {
        DIR {
            fd: -1,
            entry: Dirent::zeroed(),
            has_entry: false,
            pos: 0,
        }
    }
}

const DIR_POOL_SIZE: usize = 16;

static mut DIR_POOL: [DIR; DIR_POOL_SIZE] = {
    const ZERO: DIR = DIR::zeroed();
    [ZERO; DIR_POOL_SIZE]
};
static mut DIR_USED: [bool; DIR_POOL_SIZE] = [false; DIR_POOL_SIZE];

unsafe fn alloc_dir() -> *mut DIR {
    unsafe {
        for i in 0..DIR_POOL_SIZE {
            if !(*(&raw const DIR_USED))[i] {
                (*(&raw mut DIR_USED))[i] = true;
                let dir = &raw mut (*(&raw mut DIR_POOL))[i];
                (*dir).fd = -1;
                (*dir).has_entry = false;
                return dir;
            }
        }
        core::ptr::null_mut()
    }
}

unsafe fn free_dir(dir: *mut DIR) {
    unsafe {
        let pool_base = &raw mut DIR_POOL as *mut DIR;
        let offset = (dir as usize).wrapping_sub(pool_base as usize);
        let elem_size = core::mem::size_of::<DIR>();
        if elem_size > 0 {
            let idx = offset / elem_size;
            if idx < DIR_POOL_SIZE {
                (*(&raw mut DIR_USED))[idx] = false;
                (*dir).fd = -1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opendir(path: *const u8) -> *mut DIR {
    if path.is_null() {
        errno::set_errno(errno::EINVAL);
        return core::ptr::null_mut();
    }

    unsafe {
        let fd = trona_posix::posix_opendir(path);
        if fd < 0 {
            errno::set_errno(-fd);
            return core::ptr::null_mut();
        }

        let dir = alloc_dir();
        if dir.is_null() {
            trona_posix::posix_closedir(fd);
            errno::set_errno(errno::ENOMEM);
            return core::ptr::null_mut();
        }

        (*dir).fd = fd;
        dir
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fdopendir(fd: i32) -> *mut DIR {
    if fd < 0 {
        errno::set_errno(errno::EBADF);
        return core::ptr::null_mut();
    }

    unsafe {
        let dir = alloc_dir();
        if dir.is_null() {
            errno::set_errno(errno::ENOMEM);
            return core::ptr::null_mut();
        }

        (*dir).fd = fd;
        (*dir).has_entry = false;
        (*dir).pos = 0;
        dir
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn readdir(dir: *mut DIR) -> *mut Dirent {
    if dir.is_null() {
        errno::set_errno(errno::EBADF);
        return core::ptr::null_mut();
    }

    unsafe {
        let mut trona_entry = trona_posix::TronaDirent::zeroed();
        let ret = trona_posix::posix_readdir((*dir).fd, &raw mut trona_entry);
        if ret == 0 {
            // No more entries
            return core::ptr::null_mut();
        }

        // Copy fields from TronaDirent into our Dirent
        (*dir).entry.d_ino = trona_entry.d_ino;
        (*dir).entry.d_off = 0;
        (*dir).entry.d_reclen = core::mem::size_of::<Dirent>() as u16;
        (*dir).entry.d_type = trona_entry.d_type;
        (*dir).entry.d_pad0 = 0;
        (*dir).entry.d_pad1 = 0;

        // Copy name, capping at the smaller of TronaDirent.d_name (62 bytes)
        // and our d_name (256 bytes)
        let name_len = trona_entry.d_namlen as usize;
        let copy_len = if name_len < 62 { name_len } else { 61 };
        (*dir).entry.d_namlen = copy_len as u16;
        for i in 0..copy_len {
            (*dir).entry.d_name[i] = trona_entry.d_name[i];
        }
        (*dir).entry.d_name[copy_len] = 0;
        (*dir).has_entry = true;
        (*dir).pos += 1;

        &raw mut (*dir).entry
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn closedir(dir: *mut DIR) -> i32 {
    if dir.is_null() {
        errno::set_errno(errno::EBADF);
        return -1;
    }

    unsafe {
        let fd = (*dir).fd;
        trona_posix::posix_closedir(fd);
        free_dir(dir);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dirfd(dir: *mut DIR) -> i32 {
    if dir.is_null() {
        errno::set_errno(errno::EBADF);
        return -1;
    }
    unsafe { (*dir).fd }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rewinddir(dir: *mut DIR) {
    if !dir.is_null() {
        unsafe {
            (*dir).pos = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn telldir(dir: *mut DIR) -> i64 {
    if dir.is_null() {
        return -1;
    }
    unsafe { (*dir).pos }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seekdir(dir: *mut DIR, loc: i64) {
    if !dir.is_null() {
        unsafe {
            (*dir).pos = loc;
        }
    }
}

// ---------------------------------------------------------------------------
// alphasort / scandir — directory scanning
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alphasort(a: *const *const Dirent, b: *const *const Dirent) -> i32 {
    unsafe {
        let na = (**a).d_name.as_ptr();
        let nb = (**b).d_name.as_ptr();
        crate::string::strcmp(na, nb)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn scandir(
    dirname: *const u8,
    namelist: *mut *mut *mut Dirent,
    filter: Option<unsafe extern "C" fn(*const Dirent) -> i32>,
    compar: Option<unsafe extern "C" fn(*const *const Dirent, *const *const Dirent) -> i32>,
) -> i32 {
    unsafe {
        let dir = opendir(dirname);
        if dir.is_null() {
            return -1;
        }

        let mut entries: *mut *mut Dirent = core::ptr::null_mut();
        let mut count: usize = 0;
        let mut cap: usize = 0;

        loop {
            let ent = readdir(dir);
            if ent.is_null() {
                break;
            }

            if let Some(f) = filter {
                if f(ent as *const Dirent) == 0 {
                    continue;
                }
            }

            if count >= cap {
                cap = if cap == 0 { 16 } else { cap * 2 };
                let ptr_size = core::mem::size_of::<*mut Dirent>();
                let new_arr = crate::malloc::realloc(entries as *mut u8, cap * ptr_size);
                if new_arr.is_null() {
                    let mut i = 0;
                    while i < count {
                        crate::malloc::free(*entries.add(i) as *mut u8);
                        i += 1;
                    }
                    crate::malloc::free(entries as *mut u8);
                    closedir(dir);
                    return -1;
                }
                entries = new_arr as *mut *mut Dirent;
            }

            let ent_size = core::mem::size_of::<Dirent>();
            let copy = crate::malloc::malloc(ent_size) as *mut Dirent;
            if copy.is_null() {
                let mut i = 0;
                while i < count {
                    crate::malloc::free(*entries.add(i) as *mut u8);
                    i += 1;
                }
                crate::malloc::free(entries as *mut u8);
                closedir(dir);
                return -1;
            }
            core::ptr::copy_nonoverlapping(ent as *const Dirent, copy, 1);
            *entries.add(count) = copy;
            count += 1;
        }

        closedir(dir);

        if let Some(cmp) = compar {
            crate::stdlib::qsort(
                entries as *mut u8,
                count,
                core::mem::size_of::<*mut Dirent>(),
                core::mem::transmute(cmp),
            );
        }

        *namelist = entries;
        count as i32
    }
}
