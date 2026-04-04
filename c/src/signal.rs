//! POSIX signal handling
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Single-threaded signal implementation backed by libtrona's notification
//! mechanism. Signal handlers are registered via `trona_posix::signals::posix_signal`,
//! which sets up a kernel notification object to deliver signals asynchronously.
//!
//! The `sigaction` interface stores `sa_mask` and `sa_flags` in shared libtrona
//! globals (`__sig_sa_mask`, `__sig_sa_flags`) so the signal delivery trampoline
//! can apply the correct mask before invoking the handler. Up to 32 signals
//! are supported (`NSIG = 32`).

use crate::errno;

pub type SighandlerT = usize;
pub const SIG_DFL: SighandlerT = 0;
pub const SIG_IGN: SighandlerT = 1;
pub const SIG_ERR: SighandlerT = usize::MAX;

// SA_RESTART: In SaltyOS, POSIX signals are delivered cooperatively via
// notification polling (posix_sigcheck). System calls (IPC to VFS/procmgr)
// complete atomically from userland's perspective and are never interrupted
// by signals. Therefore SA_RESTART has no behavioral effect — it is stored
// in __sig_sa_flags for sigaction() compatibility but intentionally unused.
pub const SA_RESTART: i32 = 0x10000000;
pub const SA_NOCLDSTOP: i32 = 0x00000001;
pub const SA_NOCLDWAIT: i32 = 0x00000002;
pub const SA_SIGINFO: i32 = 0x00000004;
pub const SA_RESETHAND: i32 = 0x80000000u32 as i32;

pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

const NSIG: usize = 32;

#[repr(C)]
pub struct Sigaction {
    pub sa_handler: SighandlerT,
    pub sa_mask: Sigset,
    pub sa_flags: i32,
    pub sa_restorer: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Sigset {
    pub bits: u32,
}

static mut HANDLERS: [SighandlerT; NSIG] = [SIG_DFL; NSIG];
static mut BLOCKED_MASK: Sigset = Sigset { bits: 0 };

/// Mutex protecting HANDLERS and libtrona signal globals (__sig_sa_mask,
/// __sig_sa_flags) for thread-safe signal()/sigaction().
static SIGNAL_LOCK: trona_posix::sync::Mutex = trona_posix::sync::Mutex::new();

/// Install a signal handler for signal `sig`.
///
/// Registers the handler with libtrona's notification-based signal delivery
/// via `trona_posix::signals::posix_signal`. Both `SIG_DFL` and `SIG_IGN` are
/// forwarded to libtrona so it can update the kernel notification mask.
///
/// Returns the previous handler on success, or `SIG_ERR` on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn signal(sig: i32, handler: SighandlerT) -> SighandlerT {
    if sig <= 0 || sig >= NSIG as i32 {
        errno::set_errno(errno::EINVAL);
        return SIG_ERR;
    }

    SIGNAL_LOCK.lock();
    let result = unsafe {
        let old = *(&raw const HANDLERS)
            .cast::<[SighandlerT; NSIG]>()
            .as_ref()
            .unwrap_unchecked()
            .get_unchecked(sig as usize);
        (*(&raw mut HANDLERS))[sig as usize] = handler;

        let reg_result = trona_posix::signals::posix_signal(sig, handler);
        if reg_result == usize::MAX {
            // Registration failed, revert
            (*(&raw mut HANDLERS))[sig as usize] = old;
            errno::set_errno(errno::EINVAL);
            SIG_ERR
        } else {
            old
        }
    };
    SIGNAL_LOCK.unlock();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sigaction(sig: i32, act: *const Sigaction, oact: *mut Sigaction) -> i32 {
    if sig <= 0 || sig >= NSIG as i32 {
        errno::set_errno(errno::EINVAL);
        return -1;
    }

    SIGNAL_LOCK.lock();
    let result = unsafe {
        // Fill old action if requested
        if !oact.is_null() {
            (*oact).sa_handler = (*(&raw const HANDLERS))[sig as usize];
            (*oact).sa_mask = Sigset {
                bits: (*(&raw const trona_posix::__sig_sa_mask))[sig as usize],
            };
            (*oact).sa_flags = (*(&raw const trona_posix::__sig_sa_flags))[sig as usize];
            (*oact).sa_restorer = 0;
        }

        // Install new action if provided
        if !act.is_null() {
            let new_handler = (*act).sa_handler;
            let old = (*(&raw const HANDLERS))[sig as usize];

            (*(&raw mut HANDLERS))[sig as usize] = new_handler;

            let reg_result = trona_posix::signals::posix_signal(sig, new_handler);
            if reg_result == usize::MAX {
                // Revert on failure
                (*(&raw mut HANDLERS))[sig as usize] = old;
                errno::set_errno(errno::EINVAL);
                SIGNAL_LOCK.unlock();
                return -1;
            }

            // Store sa_mask and sa_flags in shared libtrona globals
            (*(&raw mut trona_posix::__sig_sa_mask))[sig as usize] = (*act).sa_mask.bits;
            (*(&raw mut trona_posix::__sig_sa_flags))[sig as usize] = (*act).sa_flags;
        }

        0
    };
    SIGNAL_LOCK.unlock();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sigprocmask(how: i32, set: *const Sigset, oldset: *mut Sigset) -> i32 {
    unsafe {
        let current = *(&raw const trona_posix::__sig_blocked_mask);
        if !oldset.is_null() {
            (*oldset).bits = current;
        }

        if !set.is_null() {
            let new_bits = (*set).bits;
            let updated = match how {
                SIG_BLOCK => current | new_bits,
                SIG_UNBLOCK => current & !new_bits,
                SIG_SETMASK => new_bits,
                _ => {
                    errno::set_errno(errno::EINVAL);
                    return -1;
                }
            };
            (*(&raw mut trona_posix::__sig_blocked_mask)) = updated;
            (*(&raw mut BLOCKED_MASK)).bits = updated;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sigsuspend(mask: *const Sigset) -> i32 {
    if mask.is_null() {
        errno::set_errno(errno::EFAULT);
        return -1;
    }
    unsafe {
        // Save current blocked mask
        let saved = *(&raw const trona_posix::__sig_blocked_mask);
        // Apply temporary mask
        (*(&raw mut trona_posix::__sig_blocked_mask)) = (*mask).bits;
        (*(&raw mut BLOCKED_MASK)).bits = (*mask).bits;

        // Wait on signal notification (blocking).
        // salty_wait atomically swaps notification bits to 0 (consuming them).
        // We must repost the consumed bits so posix_sigcheck can find them.
        let cap_signal_ntfn: u64 = 6;
        let pending_bits = trona::trona_wait(cap_signal_ntfn);
        if pending_bits != 0 {
            trona::trona_signal(cap_signal_ntfn, pending_bits);
        }

        // Dispatch pending signals
        trona_posix::signals::posix_sigcheck();

        // Restore original mask
        (*(&raw mut trona_posix::__sig_blocked_mask)) = saved;
        (*(&raw mut BLOCKED_MASK)).bits = saved;

        errno::set_errno(errno::EINTR);
        -1 // POSIX: always returns -1 with EINTR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sigpending(set: *mut Sigset) -> i32 {
    if set.is_null() {
        errno::set_errno(errno::EFAULT);
        return -1;
    }
    unsafe {
        // Non-blocking poll for notification bits
        let cap_signal_ntfn: u64 = 6;
        let mut bits: u64 = 0;
        let err = trona::trona_poll(cap_signal_ntfn, &raw mut bits);

        if err == 0 && bits != 0 {
            // Re-signal ALL consumed bits back (poll is destructive)
            trona::trona_signal(cap_signal_ntfn, bits);

            // Pending = signaled AND blocked
            let blocked = *(&raw const trona_posix::__sig_blocked_mask);
            (*set).bits = (bits as u32) & blocked;
        } else {
            (*set).bits = 0;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sigemptyset(set: *mut Sigset) -> i32 {
    if set.is_null() {
        return -1;
    }
    unsafe {
        (*set).bits = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sigfillset(set: *mut Sigset) -> i32 {
    if set.is_null() {
        return -1;
    }
    unsafe {
        (*set).bits = 0xFFFFFFFF;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sigaddset(set: *mut Sigset, sig: i32) -> i32 {
    if set.is_null() || sig <= 0 || sig >= NSIG as i32 {
        return -1;
    }
    unsafe {
        (*set).bits |= 1u32 << sig;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sigdelset(set: *mut Sigset, sig: i32) -> i32 {
    if set.is_null() || sig <= 0 || sig >= NSIG as i32 {
        return -1;
    }
    unsafe {
        (*set).bits &= !(1u32 << sig);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sigismember(set: *const Sigset, sig: i32) -> i32 {
    if set.is_null() || sig <= 0 || sig >= NSIG as i32 {
        return -1;
    }
    unsafe { (((*set).bits >> sig) & 1) as i32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn siginterrupt(_sig: i32, _flag: i32) -> i32 {
    0
}

/// pthread_sigmask is identical to sigprocmask for single-process signal masks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_sigmask(how: i32, set: *const Sigset, oldset: *mut Sigset) -> i32 {
    unsafe { sigprocmask(how, set, oldset) }
}

// ---------------------------------------------------------------------------
// Signal name table
// ---------------------------------------------------------------------------

/// Wrapper to make `*const u8` usable in statics (raw pointers lack `Sync`).
#[repr(transparent)]
struct SyncPtr(*const u8);
// SAFETY: All pointers in sys_signame point to static byte string literals
// which are immutable and have 'static lifetime.
unsafe impl Sync for SyncPtr {}

/// Signal name table indexed by signal number.
/// `sys_signame[1]` = "HUP", `sys_signame[2]` = "INT", etc.
#[unsafe(no_mangle)]
pub static sys_signame: [SyncPtr; NSIG] = [
    SyncPtr(b"EXIT\0".as_ptr()),   // 0
    SyncPtr(b"HUP\0".as_ptr()),    // 1 SIGHUP
    SyncPtr(b"INT\0".as_ptr()),    // 2 SIGINT
    SyncPtr(b"QUIT\0".as_ptr()),   // 3 SIGQUIT
    SyncPtr(b"ILL\0".as_ptr()),    // 4 SIGILL
    SyncPtr(b"TRAP\0".as_ptr()),   // 5 SIGTRAP
    SyncPtr(b"ABRT\0".as_ptr()),   // 6 SIGABRT
    SyncPtr(b"BUS\0".as_ptr()),    // 7 SIGBUS
    SyncPtr(b"FPE\0".as_ptr()),    // 8 SIGFPE
    SyncPtr(b"KILL\0".as_ptr()),   // 9 SIGKILL
    SyncPtr(b"USR1\0".as_ptr()),   // 10 SIGUSR1
    SyncPtr(b"SEGV\0".as_ptr()),   // 11 SIGSEGV
    SyncPtr(b"USR2\0".as_ptr()),   // 12 SIGUSR2
    SyncPtr(b"PIPE\0".as_ptr()),   // 13 SIGPIPE
    SyncPtr(b"ALRM\0".as_ptr()),   // 14 SIGALRM
    SyncPtr(b"TERM\0".as_ptr()),   // 15 SIGTERM
    SyncPtr(b"STKFLT\0".as_ptr()), // 16 SIGSTKFLT
    SyncPtr(b"CHLD\0".as_ptr()),   // 17 SIGCHLD
    SyncPtr(b"CONT\0".as_ptr()),   // 18 SIGCONT
    SyncPtr(b"STOP\0".as_ptr()),   // 19 SIGSTOP
    SyncPtr(b"TSTP\0".as_ptr()),   // 20 SIGTSTP
    SyncPtr(b"TTIN\0".as_ptr()),   // 21 SIGTTIN
    SyncPtr(b"TTOU\0".as_ptr()),   // 22 SIGTTOU
    SyncPtr(core::ptr::null()),    // 23 (undefined)
    SyncPtr(core::ptr::null()),    // 24 (undefined)
    SyncPtr(core::ptr::null()),    // 25 (undefined)
    SyncPtr(core::ptr::null()),    // 26 (undefined)
    SyncPtr(core::ptr::null()),    // 27 (undefined)
    SyncPtr(b"WINCH\0".as_ptr()),  // 28 SIGWINCH
    SyncPtr(b"INFO\0".as_ptr()),   // 29 SIGINFO
    SyncPtr(core::ptr::null()),    // 30 (undefined)
    SyncPtr(core::ptr::null()),    // 31 (undefined)
];
