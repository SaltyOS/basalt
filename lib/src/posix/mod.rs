//! POSIX file I/O and process management wrappers
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Every POSIX operation is implemented as an IPC `Call` to either the VFS
//! server (`CAP_VFS_EP`) or the process manager (`CAP_PROCMGR_EP`). The
//! client packs arguments into a `BesaltMsg`, sends it, and unpacks the
//! reply. No kernel objects are created -- all state lives in the servers.
//!
//! # Data transfer chunking
//!
//! `posix_read` and `posix_write` transfer data in chunks of up to 152/144
//! bytes per IPC round-trip (limited by the 20-register message buffer).
//! Large reads/writes loop until the full count is transferred or EOF.
//!
//! # Path encoding
//!
//! Filesystem paths are packed into message registers by `pack_path()`:
//! `regs[offset]` = path length (max 64), followed by the path bytes
//! packed into subsequent u64 registers.

mod file;
mod socket;
mod poll;
mod pipe;
mod proc;
mod misc;
mod at;
pub(crate) mod bulk;

pub use file::*;
pub use socket::*;
pub use poll::*;
pub use pipe::*;
pub use proc::*;
pub use misc::*;
pub use at::*;

use crate::consts::*;
use crate::types::*;

// Standard child CSpace layout (set by procmgr at spawn time)
const CAP_PROCMGR_EP: u64 = 3;
const CAP_VFS_EP: u64 = 4;

/// Convert a server error label to a negative POSIX errno code.
///
/// Servers return specific error codes in `reply.label`. This translates them
/// to negative errno values so besaltc can extract the correct errno.
pub(crate) fn besalt_err_to_posix(label: u64) -> i32 {
    match label {
        BESALT_OK => 0,
        BESALT_NOT_FOUND => -2,                // ENOENT
        BESALT_ALREADY_EXISTS => -17,           // EEXIST
        BESALT_INVALID_ARGUMENT => -22,         // EINVAL
        BESALT_OUT_OF_MEMORY => -12,            // ENOMEM
        BESALT_BUSY => -16,                     // EBUSY
        BESALT_WOULD_BLOCK => -11,              // EAGAIN
        BESALT_IN_PROGRESS => -115,            // EINPROGRESS
        BESALT_BAD_ADDRESS => -14,              // EFAULT
        BESALT_INSUFFICIENT_RIGHTS => -13,      // EACCES
        BESALT_INVALID_CAPABILITY => -9,        // EBADF
        BESALT_DEADLOCK => -35,                 // EDEADLK
        BESALT_INVALID_OPERATION => -1,         // EPERM
        BESALT_OUT_OF_RANGE => -34,             // ERANGE
        BESALT_CANCELLED => -125,               // ECANCELED
        BESALT_CONN_REFUSED => -111,            // ECONNREFUSED
        BESALT_TIMED_OUT => -110,               // ETIMEDOUT
        BESALT_PROTO_NOT_SUPPORTED => -93,      // EPROTONOSUPPORT
        BESALT_HOST_UNREACHABLE => -113,        // EHOSTUNREACH
        BESALT_NET_UNREACHABLE => -101,         // ENETUNREACH
        BESALT_NO_BUFS => -105,                 // ENOBUFS
        BESALT_CONN_RESET => -104,              // ECONNRESET
        BESALT_NOT_CONNECTED => -107,           // ENOTCONN
        BESALT_IS_CONNECTED => -106,            // EISCONN
        BESALT_ADDR_IN_USE => -98,              // EADDRINUSE
        BESALT_DNS_NXDOMAIN => -2,              // ENOENT
        BESALT_DNS_SERVER_FAIL => -5,           // EIO
        _ => -5,                               // EIO (generic)
    }
}

/// Pack a null-terminated path into message registers starting at `offset`.
///
/// Writes the path length into `regs[offset]` and the path bytes (up to 64)
/// into `regs[offset+1..]`. Returns the path length.
pub(crate) unsafe fn pack_path(msg: *mut BesaltMsg, offset: usize, path: *const u8, max_len: usize) -> u8 {
    unsafe {
        let avail = (20usize.saturating_sub(offset + 1)) * 8;
        let cap = if max_len < 128 { max_len } else { 128 };
        let limit = if cap < avail { cap } else { avail };
        let mut path_len: u8 = 0;
        while (path_len as usize) < limit && *path.add(path_len as usize) != 0 {
            path_len += 1;
        }
        (*msg).regs[offset] = path_len as u64;
        for i in (offset + 1)..20 {
            (*msg).regs[i] = 0;
        }
        let dst = &mut (*msg).regs[offset + 1] as *mut u64 as *mut u8;
        for i in 0..path_len as usize {
            *dst.add(i) = *path.add(i);
        }
        path_len
    }
}
