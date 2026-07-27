//! fts -- BSD file tree stream (fts(3))
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Minimal BSD fts(3) file tree stream for SaltyOS. Provides depth-first
//! traversal of file hierarchies with NOCHDIR behavior. Sufficient for
//! ls, cp -r, rm -r, chmod -R, find, etc.
//!
//! The five public functions (`fts_open`, `fts_read`, `fts_children`,
//! `fts_set`, `fts_close`) match the FreeBSD ABI defined in `<fts.h>`.

use crate::dirent::{closedir, opendir, readdir};
use crate::errno;
use crate::unistd::Stat;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// fts_open options
const FTS_COMFOLLOW: i32 = 0x001;
const FTS_LOGICAL: i32 = 0x002;
const FTS_NOCHDIR: i32 = 0x004;
const FTS_SEEDOT: i32 = 0x020;
const FTS_XDEV: i32 = 0x040;
const FTS_NAMEONLY: i32 = 0x100;

// fts_info values
const FTS_D: i32 = 1;
const FTS_DEFAULT: i32 = 3;
const FTS_DNR: i32 = 4;
const FTS_DP: i32 = 6;
const FTS_F: i32 = 8;
const FTS_INIT: i32 = 9;
const FTS_NS: i32 = 10;
const FTS_NSOK: i32 = 11;
const FTS_SL: i32 = 12;
const FTS_SLNONE: i32 = 13;

// fts_set instructions
const FTS_AGAIN: i32 = 1;
const FTS_FOLLOW: i32 = 2;
const FTS_NOINSTR: i32 = 3;
const FTS_SKIP: i32 = 4;

// fts_level
const FTS_ROOTLEVEL: i64 = 0;

// File type constants (from sys/stat.h)
const S_IFMT: u16 = 0o170000;
const S_IFLNK: u16 = 0o120000;
const S_IFREG: u16 = 0o100000;
const S_IFDIR: u16 = 0o040000;

fn s_isdir(m: u16) -> bool {
    (m & S_IFMT) == S_IFDIR
}

fn s_isreg(m: u16) -> bool {
    (m & S_IFMT) == S_IFREG
}

fn s_islnk(m: u16) -> bool {
    (m & S_IFMT) == S_IFLNK
}

// Internal traversal states
const STATE_ROOTS: i32 = 0;
const STATE_CHILDREN: i32 = 1;
const STATE_POSTORDER: i32 = 2;
const STATE_DONE: i32 = 3;

// Maximum path length
const FTS_MAXPATH: usize = 4096;

// ---------------------------------------------------------------------------
// FFI-compatible structures (must match include/fts.h exactly)
// ---------------------------------------------------------------------------

/// Comparator function type for fts_open.
type FtsCompar = unsafe extern "C" fn(*const *const Ftsent, *const *const Ftsent) -> i32;

/// FTS stream handle — matches `struct _fts` in fts.h.
#[repr(C)]
pub struct Fts {
    pub fts_cur: *mut Ftsent,
    pub fts_child: *mut Ftsent,
    pub fts_array: *mut *mut Ftsent,
    pub fts_dev: u64,
    pub fts_path: *mut u8,
    pub fts_rfd: i32,
    pub fts_pathlen: usize,
    pub fts_nitems: usize,
    pub fts_compar: Option<FtsCompar>,
    pub fts_options: i32,
    pub fts_clientptr: *mut u8,
}

/// FTS entry — matches `struct _ftsent` in fts.h.
#[repr(C)]
pub struct Ftsent {
    pub fts_cycle: *mut Ftsent,
    pub fts_parent: *mut Ftsent,
    pub fts_link: *mut Ftsent,
    pub fts_number: i64,
    pub fts_pointer: *mut u8,
    pub fts_accpath: *mut u8,
    pub fts_path: *mut u8,
    pub fts_errno: i32,
    pub fts_symfd: i32,
    pub fts_pathlen: usize,
    pub fts_namelen: usize,
    pub fts_ino: u64,
    pub fts_dev: u64,
    pub fts_nlink: u64,
    pub fts_level: i64,
    pub fts_info: i32,
    pub fts_flags: u32,
    pub fts_instr: i32,
    pub fts_statp: *mut Stat,
    pub fts_name: *mut u8,
    pub fts_fts: *mut Fts,
}

/// Private state stored in fts_clientptr.
#[repr(C)]
struct FtsPriv {
    argv: *mut *mut u8,
    argc: i32,
    arg_idx: i32,
    state: i32,
}

// ---------------------------------------------------------------------------
// Saved comparator for qsort wrapper (single-threaded, no reentrancy issue)
// ---------------------------------------------------------------------------

static mut FTS_SAVED_COMPAR: Option<FtsCompar> = None;

unsafe fn fts_priv(sp: *mut Fts) -> *mut FtsPriv {
    unsafe { (*sp).fts_clientptr as *mut FtsPriv }
}

unsafe extern "C" fn fts_qsort_wrapper(a: *const u8, b: *const u8) -> i32 {
    unsafe {
        if let Some(compar) = *(&raw const FTS_SAVED_COMPAR) {
            compar(a as *const *const Ftsent, b as *const *const Ftsent)
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Allocate an FTSENT with stat buffer, path string, and name string packed
/// into a single allocation:
///   [FTSENT][Stat][path bytes \0][name bytes \0]
unsafe fn fts_alloc(
    sp: *mut Fts,
    name: *const u8,
    namelen: usize,
    path: *const u8,
    pathlen: usize,
) -> *mut Ftsent {
    unsafe {
        let ftsent_size = core::mem::size_of::<Ftsent>();
        let stat_size = core::mem::size_of::<Stat>();
        let total = ftsent_size + stat_size + pathlen + 1 + namelen + 1;

        let ptr = crate::malloc::calloc(1, total);
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        let p = ptr as *mut Ftsent;

        // fts_statp points right after the Ftsent
        (*p).fts_statp = ptr.add(ftsent_size) as *mut Stat;

        // fts_path points after the Stat
        (*p).fts_path = ptr.add(ftsent_size + stat_size);
        core::ptr::copy_nonoverlapping(path, (*p).fts_path, pathlen);
        *(*p).fts_path.add(pathlen) = 0;
        (*p).fts_pathlen = pathlen;

        // fts_name points after the path
        (*p).fts_name = (*p).fts_path.add(pathlen + 1);
        core::ptr::copy_nonoverlapping(name, (*p).fts_name, namelen);
        *(*p).fts_name.add(namelen) = 0;
        (*p).fts_namelen = namelen;

        // NOCHDIR mode: access path is the full path
        (*p).fts_accpath = (*p).fts_path;
        (*p).fts_info = FTS_INIT;
        (*p).fts_flags = 0;
        (*p).fts_instr = FTS_NOINSTR;
        (*p).fts_link = core::ptr::null_mut();
        (*p).fts_parent = core::ptr::null_mut();
        (*p).fts_cycle = core::ptr::null_mut();
        (*p).fts_number = 0;
        (*p).fts_pointer = core::ptr::null_mut();
        (*p).fts_errno = 0;
        (*p).fts_symfd = -1;
        (*p).fts_ino = 0;
        (*p).fts_dev = 0;
        (*p).fts_nlink = 0;
        (*p).fts_fts = sp;

        p
    }
}

/// Free a linked list of FTSENT nodes (following fts_link).
unsafe fn fts_free_list(mut head: *mut Ftsent) {
    unsafe {
        while !head.is_null() {
            let next = (*head).fts_link;
            crate::malloc::free(head as *mut u8);
            head = next;
        }
    }
}

/// Extract the basename from a path string.
/// Returns pointer into the path buffer (does not allocate).
unsafe fn fts_basename(path: *const u8) -> *const u8 {
    unsafe {
        let slash = crate::string::strrchr(path, b'/' as i32);
        if !slash.is_null() {
            slash.add(1) as *const u8
        } else {
            path
        }
    }
}

/// Build a child path: parent/name. Writes into buf[bufsz].
/// Returns length on success, 0 on overflow.
unsafe fn fts_build_path(
    buf: *mut u8,
    bufsz: usize,
    parent: *const u8,
    plen: usize,
    name: *const u8,
    nlen: usize,
) -> usize {
    unsafe {
        let need_sep = plen > 0 && *parent.add(plen - 1) != b'/';
        let sep_len: usize = if need_sep { 1 } else { 0 };
        let total = plen + sep_len + nlen;

        if total >= bufsz {
            return 0;
        }

        core::ptr::copy_nonoverlapping(parent, buf, plen);
        let mut off = plen;
        if need_sep {
            *buf.add(off) = b'/';
            off += 1;
        }
        core::ptr::copy_nonoverlapping(name, buf.add(off), nlen);
        *buf.add(off + nlen) = 0;
        off + nlen
    }
}

/// Perform stat or lstat on an entry, set fts_info accordingly.
/// Returns 0 on success, -1 on stat failure (fts_info set to FTS_NS).
unsafe fn fts_stat_entry(sp: *mut Fts, ent: *mut Ftsent) -> i32 {
    unsafe {
        let opts = (*sp).fts_options;
        let rc;

        if (opts & FTS_LOGICAL) != 0
            || ((opts & FTS_COMFOLLOW) != 0 && (*ent).fts_level == FTS_ROOTLEVEL)
        {
            rc = crate::unistd::stat((*ent).fts_path, (*ent).fts_statp);
        } else {
            // FTS_PHYSICAL: use lstat to not follow symlinks
            rc = crate::unistd::lstat((*ent).fts_path, (*ent).fts_statp);
        }

        if rc != 0 {
            (*ent).fts_errno = errno::get_errno();
            (*ent).fts_info = FTS_NS;
            return -1;
        }

        (*ent).fts_ino = (*(*ent).fts_statp).st_ino;
        (*ent).fts_dev = (*(*ent).fts_statp).st_dev;
        (*ent).fts_nlink = (*(*ent).fts_statp).st_nlink;

        let mode = (*(*ent).fts_statp).st_mode;

        if s_isdir(mode) {
            (*ent).fts_info = FTS_D;
        } else if s_isreg(mode) {
            (*ent).fts_info = FTS_F;
        } else if s_islnk(mode) {
            // For FTS_PHYSICAL we got lstat, so S_ISLNK is possible.
            // Check if the target exists; if not, FTS_SLNONE.
            let mut target: Stat = core::mem::zeroed();
            if crate::unistd::stat((*ent).fts_path, &raw mut target) != 0 {
                (*ent).fts_info = FTS_SLNONE;
            } else {
                (*ent).fts_info = FTS_SL;
            }
        } else {
            (*ent).fts_info = FTS_DEFAULT;
        }

        0
    }
}

/// Sort a linked list of FTSENT using the stream's comparator.
/// Returns the new head of the sorted list.
unsafe fn fts_sort_list(sp: *mut Fts, head: *mut Ftsent) -> *mut Ftsent {
    unsafe {
        let compar = match (*sp).fts_compar {
            Some(c) => c,
            None => return head,
        };

        // Count entries
        let mut n: usize = 0;
        let mut p = head;
        while !p.is_null() {
            n += 1;
            p = (*p).fts_link;
        }

        if n <= 1 {
            return head;
        }

        // Allocate array of pointers
        let ptr_size = core::mem::size_of::<*mut Ftsent>();
        let arr = crate::malloc::malloc(n * ptr_size) as *mut *mut Ftsent;
        if arr.is_null() {
            return head;
        }

        // Fill array
        let mut i: usize = 0;
        p = head;
        while !p.is_null() {
            *arr.add(i) = p;
            i += 1;
            p = (*p).fts_link;
        }

        // Sort via qsort with saved comparator
        *(&raw mut FTS_SAVED_COMPAR) = Some(compar);
        crate::stdlib::qsort(arr as *mut u8, n, ptr_size, fts_qsort_wrapper);

        // Rebuild linked list in sorted order
        i = 0;
        while i + 1 < n {
            (**arr.add(i)).fts_link = *arr.add(i + 1);
            i += 1;
        }
        (**arr.add(n - 1)).fts_link = core::ptr::null_mut();
        let new_head = *arr;

        crate::malloc::free(arr as *mut u8);
        new_head
    }
}

/// Read directory contents and build a linked list of child FTSENT nodes.
/// Returns the head of the list, or NULL on error/empty.
unsafe fn fts_build_children(sp: *mut Fts, parent: *mut Ftsent, nameonly: i32) -> *mut Ftsent {
    unsafe {
        let dirp = opendir((*parent).fts_path);
        if dirp.is_null() {
            (*parent).fts_info = FTS_DNR;
            (*parent).fts_errno = errno::get_errno();
            return core::ptr::null_mut();
        }

        let mut pathbuf = [0u8; FTS_MAXPATH];
        let mut head: *mut Ftsent = core::ptr::null_mut();
        let mut tail: *mut Ftsent = core::ptr::null_mut();

        loop {
            let de = readdir(dirp);
            if de.is_null() {
                break;
            }

            let d_name = &(*de).d_name as *const u8;

            // Skip . and .. unless FTS_SEEDOT is set
            if *d_name == b'.' {
                let second = *d_name.add(1);
                if second == 0 {
                    // "."
                    if ((*sp).fts_options & FTS_SEEDOT) == 0 {
                        continue;
                    }
                } else if second == b'.' && *d_name.add(2) == 0 {
                    // ".."
                    if ((*sp).fts_options & FTS_SEEDOT) == 0 {
                        continue;
                    }
                }
            }

            let nlen = crate::string::strlen(d_name);
            let plen = fts_build_path(
                pathbuf.as_mut_ptr(),
                FTS_MAXPATH,
                (*parent).fts_path,
                (*parent).fts_pathlen,
                d_name,
                nlen,
            );
            if plen == 0 {
                continue; // path too long, skip
            }

            let child = fts_alloc(sp, d_name, nlen, pathbuf.as_ptr(), plen);
            if child.is_null() {
                continue; // allocation failure, skip entry
            }

            (*child).fts_level = (*parent).fts_level + 1;
            (*child).fts_parent = parent;

            if nameonly != 0 {
                (*child).fts_info = FTS_NSOK;
            } else {
                fts_stat_entry(sp, child);
            }

            // Check FTS_XDEV: skip entries on different device
            if nameonly == 0
                && ((*sp).fts_options & FTS_XDEV) != 0
                && (*child).fts_info != FTS_NS
                && (*child).fts_info != FTS_NSOK
            {
                if (*(*child).fts_statp).st_dev != (*sp).fts_dev {
                    crate::malloc::free(child as *mut u8);
                    continue;
                }
            }

            // Append to list
            (*child).fts_link = core::ptr::null_mut();
            if head.is_null() {
                head = child;
                tail = child;
            } else {
                (*tail).fts_link = child;
                tail = child;
            }
        }

        closedir(dirp);

        // Sort children if a comparator was provided
        if !head.is_null() {
            head = fts_sort_list(sp, head);
        }

        head
    }
}

/// Build an FTSENT list for the original argv root paths.
/// Used by callers that query fts_children() before the first fts_read().
unsafe fn fts_build_roots(sp: *mut Fts, nameonly: i32) -> *mut Ftsent {
    unsafe {
        let priv_ptr = fts_priv(sp);
        if priv_ptr.is_null() {
            return core::ptr::null_mut();
        }

        let mut head: *mut Ftsent = core::ptr::null_mut();
        let mut tail: *mut Ftsent = core::ptr::null_mut();

        let mut i = 0;
        while i < (*priv_ptr).argc {
            let root = *(*priv_ptr).argv.add(i as usize);
            let mut rlen = crate::string::strlen(root);
            while rlen > 1 && *root.add(rlen - 1) == b'/' {
                rlen -= 1;
            }

            let name = fts_basename(root);
            let nlen = crate::string::strlen(name);
            let ent = fts_alloc(sp, name, nlen, root, rlen);
            if ent.is_null() {
                i += 1;
                continue;
            }

            (*ent).fts_level = FTS_ROOTLEVEL;
            (*ent).fts_parent = core::ptr::null_mut();

            if nameonly != 0 {
                (*ent).fts_info = FTS_NSOK;
            } else {
                fts_stat_entry(sp, ent);
            }

            (*ent).fts_link = core::ptr::null_mut();
            if head.is_null() {
                head = ent;
                tail = ent;
            } else {
                (*tail).fts_link = ent;
                tail = ent;
            }

            i += 1;
        }

        if !head.is_null() {
            head = fts_sort_list(sp, head);
        }

        head
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fts_open(
    argv: *const *mut u8,
    options: i32,
    compar: Option<FtsCompar>,
) -> *mut Fts {
    unsafe {
        if argv.is_null() || (*argv).is_null() {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        let sp = crate::malloc::calloc(1, core::mem::size_of::<Fts>()) as *mut Fts;
        if sp.is_null() {
            return core::ptr::null_mut();
        }

        let priv_ptr = crate::malloc::calloc(1, core::mem::size_of::<FtsPriv>()) as *mut FtsPriv;
        if priv_ptr.is_null() {
            crate::malloc::free(sp as *mut u8);
            return core::ptr::null_mut();
        }

        (*sp).fts_options = options | FTS_NOCHDIR; // always nochdir
        (*sp).fts_compar = compar;
        (*sp).fts_clientptr = priv_ptr as *mut u8;
        (*sp).fts_rfd = -1;

        // Count argv entries
        let mut argc: i32 = 0;
        let mut p = argv;
        while !(*p).is_null() {
            argc += 1;
            p = p.add(1);
        }

        if argc == 0 {
            crate::malloc::free(priv_ptr as *mut u8);
            crate::malloc::free(sp as *mut u8);
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        // Copy argv
        let argv_size = (argc as usize + 1) * core::mem::size_of::<*mut u8>();
        (*priv_ptr).argv = crate::malloc::malloc(argv_size) as *mut *mut u8;
        if (*priv_ptr).argv.is_null() {
            crate::malloc::free(priv_ptr as *mut u8);
            crate::malloc::free(sp as *mut u8);
            return core::ptr::null_mut();
        }

        let mut i = 0;
        while i < argc {
            let dup = crate::malloc::strdup(*argv.add(i as usize));
            if dup.is_null() {
                // Free previously allocated strings
                let mut j = 0;
                while j < i {
                    crate::malloc::free(*(*priv_ptr).argv.add(j as usize));
                    j += 1;
                }
                crate::malloc::free((*priv_ptr).argv as *mut u8);
                crate::malloc::free(priv_ptr as *mut u8);
                crate::malloc::free(sp as *mut u8);
                return core::ptr::null_mut();
            }
            *(*priv_ptr).argv.add(i as usize) = dup;
            i += 1;
        }
        *(*priv_ptr).argv.add(argc as usize) = core::ptr::null_mut();
        (*priv_ptr).argc = argc;
        (*priv_ptr).arg_idx = 0;
        (*priv_ptr).state = STATE_ROOTS;

        (*sp).fts_cur = core::ptr::null_mut();
        (*sp).fts_child = core::ptr::null_mut();
        (*sp).fts_array = core::ptr::null_mut();
        (*sp).fts_nitems = 0;
        (*sp).fts_dev = 0;
        (*sp).fts_path = core::ptr::null_mut();
        (*sp).fts_pathlen = 0;

        sp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fts_read(sp: *mut Fts) -> *mut Ftsent {
    unsafe {
        if sp.is_null() {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        let priv_ptr = fts_priv(sp);
        if priv_ptr.is_null() {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        loop {
            let state = (*priv_ptr).state;

            if state == STATE_ROOTS {
                // Return root entries one at a time from argv
                if (*priv_ptr).arg_idx >= (*priv_ptr).argc {
                    (*priv_ptr).state = STATE_DONE;
                    return core::ptr::null_mut();
                }

                let root = *(*priv_ptr).argv.add((*priv_ptr).arg_idx as usize);
                (*priv_ptr).arg_idx += 1;

                let mut rlen = crate::string::strlen(root);

                // Strip trailing slashes for consistent paths
                while rlen > 1 && *root.add(rlen - 1) == b'/' {
                    rlen -= 1;
                }

                let name = fts_basename(root);
                let nlen = crate::string::strlen(name);

                let ent = fts_alloc(sp, name, nlen, root, rlen);
                if ent.is_null() {
                    errno::set_errno(errno::ENOMEM);
                    return core::ptr::null_mut();
                }

                (*ent).fts_level = 0;
                (*ent).fts_parent = core::ptr::null_mut();

                fts_stat_entry(sp, ent);

                // Record starting device for FTS_XDEV
                if (*priv_ptr).arg_idx == 1
                    && (*ent).fts_info != FTS_NS
                    && (*ent).fts_info != FTS_NSOK
                {
                    (*sp).fts_dev = (*(*ent).fts_statp).st_dev;
                }

                (*sp).fts_cur = ent;

                // If it is a directory, push to children state
                if (*ent).fts_info == FTS_D {
                    (*priv_ptr).state = STATE_CHILDREN;
                    // Build child list and chain onto entry
                    (*sp).fts_child = fts_build_children(sp, ent, 0);
                }
                // If more roots remain and this isn't a directory, stay in ROOTS

                return ent;
            } else if state == STATE_CHILDREN {
                // Pop next child from the child list
                if !(*sp).fts_child.is_null() {
                    let child = (*sp).fts_child;
                    (*sp).fts_child = (*child).fts_link;
                    (*child).fts_link = core::ptr::null_mut();

                    (*sp).fts_cur = child;

                    if (*child).fts_info == FTS_D {
                        // Push: save current child list on the directory's
                        // fts_link chain, then descend into this directory.
                        (*child).fts_link = (*sp).fts_child;
                        (*sp).fts_child = fts_build_children(sp, child, 0);
                    }

                    return child;
                }

                // No more children -- return postorder (FTS_DP) for the
                // current directory, then resume its parent's sibling list.
                (*priv_ptr).state = STATE_POSTORDER;
                continue;
            } else if state == STATE_POSTORDER {
                // Walk up the tree returning FTS_DP entries for each
                // completed directory.
                let cur = (*sp).fts_cur;
                if cur.is_null() {
                    (*priv_ptr).state = STATE_ROOTS;
                    continue;
                }

                // Find the directory that just finished. Walk up through
                // parents to find the innermost directory that hasn't
                // been postorder-returned yet.
                let dir: *mut Ftsent;

                // If cur itself is a preorder directory, return its DP
                if (*cur).fts_info == FTS_D {
                    dir = cur;
                } else {
                    // Walk up to find enclosing directory
                    dir = (*cur).fts_parent;
                }

                if dir.is_null() {
                    // Back at root level, continue to next root
                    (*priv_ptr).state = STATE_ROOTS;
                    continue;
                }

                // Create the postorder entry for this directory
                let dp = fts_alloc(
                    sp,
                    (*dir).fts_name,
                    (*dir).fts_namelen,
                    (*dir).fts_path,
                    (*dir).fts_pathlen,
                );
                if dp.is_null() {
                    (*priv_ptr).state = STATE_ROOTS;
                    continue;
                }

                (*dp).fts_info = FTS_DP;
                (*dp).fts_level = (*dir).fts_level;
                (*dp).fts_parent = (*dir).fts_parent;
                // Copy stat data
                core::ptr::copy_nonoverlapping(
                    (*dir).fts_statp as *const u8,
                    (*dp).fts_statp as *mut u8,
                    core::mem::size_of::<Stat>(),
                );

                (*sp).fts_cur = dp;

                // Resume siblings that were saved on the directory's fts_link
                (*sp).fts_child = (*dir).fts_link;
                (*dir).fts_link = core::ptr::null_mut();

                if !(*sp).fts_child.is_null() {
                    (*priv_ptr).state = STATE_CHILDREN;
                } else if !(*dir).fts_parent.is_null() {
                    // The parent directory may also be done.
                    // Set cur to parent so next postorder iteration handles it.
                    (*sp).fts_cur = (*dir).fts_parent;
                    (*priv_ptr).state = STATE_POSTORDER;

                    // Only emit DP if the parent is itself a directory
                    if (*(*dir).fts_parent).fts_info != FTS_D {
                        (*priv_ptr).state = STATE_ROOTS;
                    }
                } else {
                    // Root directory finished, return to roots
                    (*priv_ptr).state = STATE_ROOTS;
                }

                return dp;
            } else {
                // STATE_DONE or unknown
                return core::ptr::null_mut();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fts_children(sp: *mut Fts, instr: i32) -> *mut Ftsent {
    unsafe {
        if sp.is_null() {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        let priv_ptr = fts_priv(sp);
        if priv_ptr.is_null() {
            errno::set_errno(errno::EINVAL);
            return core::ptr::null_mut();
        }

        if (*sp).fts_cur.is_null() {
            // BSD fts allows querying children right after fts_open():
            // this returns the root argument list.
            if (*priv_ptr).arg_idx == 0 && (*priv_ptr).state == STATE_ROOTS {
                let nameonly = if (instr & FTS_NAMEONLY) != 0 { 1 } else { 0 };
                return fts_build_roots(sp, nameonly);
            }
            return core::ptr::null_mut();
        }

        let cur = (*sp).fts_cur;

        // Only meaningful for directories in preorder
        if (*cur).fts_info != FTS_D {
            return core::ptr::null_mut();
        }

        // Build a fresh child list. This is separate from the traversal
        // child list -- callers use fts_children() to peek at children
        // without advancing the traversal.
        let nameonly = if (instr & FTS_NAMEONLY) != 0 { 1 } else { 0 };
        fts_build_children(sp, cur, nameonly)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fts_set(sp: *mut Fts, ent: *mut Ftsent, instr: i32) -> i32 {
    unsafe {
        if sp.is_null() || ent.is_null() {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        if instr == FTS_SKIP {
            // Mark this entry to be skipped. If it's a preorder directory,
            // discard its children so fts_read will move to postorder.
            if ent == (*sp).fts_cur && (*ent).fts_info == FTS_D {
                fts_free_list((*sp).fts_child);
                (*sp).fts_child = core::ptr::null_mut();
            }
        } else if instr == FTS_FOLLOW {
            // Re-stat this entry following symlinks.
            // Only meaningful for FTS_SL/FTS_SLNONE entries.
            if (*ent).fts_info == FTS_SL || (*ent).fts_info == FTS_SLNONE {
                if crate::unistd::stat((*ent).fts_path, (*ent).fts_statp) == 0 {
                    let mode = (*(*ent).fts_statp).st_mode;
                    if s_isdir(mode) {
                        (*ent).fts_info = FTS_D;
                    } else if s_isreg(mode) {
                        (*ent).fts_info = FTS_F;
                    } else {
                        (*ent).fts_info = FTS_DEFAULT;
                    }
                }
            }
        } else if instr == FTS_AGAIN || instr == FTS_NOINSTR {
            // No-op in this simplified implementation
        } else {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fts_close(sp: *mut Fts) -> i32 {
    unsafe {
        if sp.is_null() {
            return 0;
        }

        let priv_ptr = fts_priv(sp);

        // Free remaining child list
        fts_free_list((*sp).fts_child);
        (*sp).fts_child = core::ptr::null_mut();

        // Free private argv copy and state
        if !priv_ptr.is_null() {
            if !(*priv_ptr).argv.is_null() {
                let mut i = 0;
                while i < (*priv_ptr).argc {
                    crate::malloc::free(*(*priv_ptr).argv.add(i as usize));
                    i += 1;
                }
                crate::malloc::free((*priv_ptr).argv as *mut u8);
            }
            crate::malloc::free(priv_ptr as *mut u8);
        }

        // Free sort array if allocated
        if !(*sp).fts_array.is_null() {
            crate::malloc::free((*sp).fts_array as *mut u8);
        }

        // Free path buffer if allocated
        if !(*sp).fts_path.is_null() {
            crate::malloc::free((*sp).fts_path);
        }

        crate::malloc::free(sp as *mut u8);
        0
    }
}
