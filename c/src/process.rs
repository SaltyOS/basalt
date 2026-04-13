//! Process management
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! POSIX process management wrappers: `fork`, `execve`, `execv`, `execvp`,
//! `execl`, `execlp`, `waitpid`, `wait`, `kill`, `raise`, `abort`, and
//! UID/GID accessors. All exec variants ultimately call `posix_execve`.
//! The `execl`/`execlp` functions collect variadic arguments into a stack
//! buffer (max 64 args).

use crate::errno;

const SIGABRT: i32 = 6;

// Maximum number of varargs we support for execl/execlp argv construction
const MAX_EXEC_ARGS: usize = 64;

// ---------------------------------------------------------------------------
// Fork / exec
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fork() -> i32 {
    let ret = trona_posix::posix_fork();
    if ret < 0 {
        errno::set_errno(-ret);
        return -1;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execve(
    path: *const u8,
    argv: *const *const u8,
    envp: *const *const u8,
) -> i32 {
    unsafe {
        let mut path_len = 0usize;
        while !path.is_null() && *path.add(path_len) != 0 && path_len < 255 {
            path_len += 1;
        }
        let mut argv0_len = 0usize;
        let argv0 = if !argv.is_null() && !(*argv).is_null() {
            *argv
        } else {
            core::ptr::null()
        };
        while !argv0.is_null() && *argv0.add(argv0_len) != 0 && argv0_len < 255 {
            argv0_len += 1;
        }
        trona::udebug!(|_lb| {
            _lb.str(b"[libc] execve path='");
            if !path.is_null() {
                _lb.bytes(core::slice::from_raw_parts(path, path_len));
            }
            _lb.str(b"' argv0='");
            if !argv0.is_null() {
                _lb.bytes(core::slice::from_raw_parts(argv0, argv0_len));
            }
            _lb.str(b"'\n");
        });
        let ret = trona_posix::posix_execve(path, argv, envp);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execvp(file: *const u8, argv: *const *const u8) -> i32 {
    if file.is_null() {
        errno::set_errno(errno::ENOENT);
        return -1;
    }
    unsafe {
        // If file contains '/', treat as absolute/relative path
        let mut i = 0;
        let mut has_slash = false;
        while *file.add(i) != 0 {
            if *file.add(i) == b'/' {
                has_slash = true;
                break;
            }
            i += 1;
        }

        if has_slash {
            return execve(file, argv, core::ptr::null());
        }

        let file_len = crate::string::strlen(file);

        // Get PATH from environment
        let path_env = crate::env::getenv(b"PATH\0".as_ptr());
        let path = if path_env.is_null() || *path_env == 0 {
            trona_posix::DEFAULT_PATH_NUL.as_ptr()
        } else {
            path_env
        };

        // Walk PATH components separated by ':'
        let mut start = 0;
        loop {
            let mut end = start;
            while *path.add(end) != 0 && *path.add(end) != b':' {
                end += 1;
            }

            let comp_len = end - start;
            if comp_len > 0 {
                let mut path_buf = [0u8; 512];
                let mut pos = 0;

                for k in 0..comp_len {
                    if pos < 510 {
                        path_buf[pos] = *path.add(start + k);
                        pos += 1;
                    }
                }

                if pos > 0 && path_buf[pos - 1] != b'/' && pos < 510 {
                    path_buf[pos] = b'/';
                    pos += 1;
                }

                for k in 0..file_len {
                    if pos < 511 {
                        path_buf[pos] = *file.add(k);
                        pos += 1;
                    }
                }
                path_buf[pos] = 0;

                let mut argv0_len = 0usize;
                let argv0 = if !argv.is_null() && !(*argv).is_null() {
                    *argv
                } else {
                    core::ptr::null()
                };
                while !argv0.is_null() && *argv0.add(argv0_len) != 0 && argv0_len < 255 {
                    argv0_len += 1;
                }
                trona::udebug!(|_lb| {
                    _lb.str(b"[libc] execvp try path='");
                    _lb.bytes(&path_buf[..pos]);
                    _lb.str(b"' argv0='");
                    if !argv0.is_null() {
                        _lb.bytes(core::slice::from_raw_parts(argv0, argv0_len));
                    }
                    _lb.str(b"'\n");
                });

                let ret = execve(path_buf.as_ptr(), argv, core::ptr::null());
                // execve only returns on error
                let _ = ret;
            }

            if *path.add(end) == 0 {
                break;
            }
            start = end + 1;
        }

        errno::set_errno(errno::ENOENT);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execvpe(
    file: *const u8,
    argv: *const *const u8,
    _envp: *const *const u8,
) -> i32 {
    // Ignore custom envp for now; delegate to execvp
    unsafe { execvp(file, argv) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execv(path: *const u8, argv: *const *const u8) -> i32 {
    unsafe { execve(path, argv, core::ptr::null()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execl(path: *const u8, arg0: *const u8, mut args: ...) -> i32 {
    unsafe {
        let mut argv_buf: [*const u8; MAX_EXEC_ARGS + 1] = [core::ptr::null(); MAX_EXEC_ARGS + 1];
        argv_buf[0] = arg0;
        let mut argc = 1;

        // Collect varargs until NULL
        loop {
            let arg: *const u8 = args.arg();
            if arg.is_null() || argc >= MAX_EXEC_ARGS {
                break;
            }
            argv_buf[argc] = arg;
            argc += 1;
        }
        argv_buf[argc] = core::ptr::null();

        execve(path, argv_buf.as_ptr(), core::ptr::null())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execlp(file: *const u8, arg0: *const u8, mut args: ...) -> i32 {
    unsafe {
        let mut argv_buf: [*const u8; MAX_EXEC_ARGS + 1] = [core::ptr::null(); MAX_EXEC_ARGS + 1];
        argv_buf[0] = arg0;
        let mut argc = 1;

        loop {
            let arg: *const u8 = args.arg();
            if arg.is_null() || argc >= MAX_EXEC_ARGS {
                break;
            }
            argv_buf[argc] = arg;
            argc += 1;
        }
        argv_buf[argc] = core::ptr::null();

        execvp(file, argv_buf.as_ptr())
    }
}

// ---------------------------------------------------------------------------
// Process termination / PID
// ---------------------------------------------------------------------------

/// Immediate process exit (no cleanup).
/// Note: crt.rs also defines _exit; this wrapper simply delegates to it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _exit_process(status: i32) -> ! {
    unsafe {
        trona_posix::posix_exit(status);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpid() -> i32 {
    unsafe { trona_posix::posix_getpid() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getppid() -> i32 {
    unsafe { trona_posix::posix_getppid() }
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32 {
    unsafe {
        let ret = trona_posix::posix_waitpid3(pid, status, options);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait(status: *mut i32) -> i32 {
    unsafe { waitpid(-1, status, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait3(status: *mut i32, options: i32, _rusage: *mut u8) -> i32 {
    unsafe { waitpid(-1, status, options) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait4(pid: i32, status: *mut i32, options: i32, _rusage: *mut u8) -> i32 {
    unsafe { waitpid(pid, status, options) }
}

// ---------------------------------------------------------------------------
// Wait status inspection (C-callable function versions of the macros)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn WIFEXITED(status: i32) -> i32 {
    ((status & 0x7f) == 0) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn WEXITSTATUS(status: i32) -> i32 {
    (status >> 8) & 0xff
}

#[unsafe(no_mangle)]
pub extern "C" fn WIFSIGNALED(status: i32) -> i32 {
    (((status & 0x7f) + 1) >> 1 > 0) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn WTERMSIG(status: i32) -> i32 {
    status & 0x7f
}

#[unsafe(no_mangle)]
pub extern "C" fn WIFSTOPPED(status: i32) -> i32 {
    ((status & 0xff) == 0x7f) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn WSTOPSIG(status: i32) -> i32 {
    (status >> 8) & 0xff
}

// ---------------------------------------------------------------------------
// Signals
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn kill(pid: i32, sig: i32) -> i32 {
    unsafe {
        let ret = trona_posix::posix_kill(pid, sig);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn killpg(pgrp: i32, sig: i32) -> i32 {
    unsafe { kill(-pgrp, sig) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn raise(sig: i32) -> i32 {
    unsafe {
        let pid = getpid();
        kill(pid, sig)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn abort() -> ! {
    unsafe {
        raise(SIGABRT);
        // If raise returns (handler caught it or ignored), force exit
        trona_posix::posix_exit(134);
    }
}

// ---------------------------------------------------------------------------
// UID / GID
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getuid() -> u32 {
    unsafe { trona_posix::posix_getuid() as u32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn geteuid() -> u32 {
    unsafe { trona_posix::posix_geteuid() as u32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgid() -> u32 {
    unsafe { trona_posix::posix_getgid() as u32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getegid() -> u32 {
    unsafe { trona_posix::posix_getegid() as u32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getgroups(size: i32, list: *mut u32) -> i32 {
    unsafe { trona_posix::posix_getgroups(size, list as *mut i32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setuid(uid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setuid(uid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgid(gid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setgid(gid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seteuid(uid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_seteuid(uid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setegid(gid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setegid(gid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setreuid(ruid: u32, euid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setreuid(ruid, euid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setregid(rgid: u32, egid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setregid(rgid, egid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setresuid(ruid: u32, euid: u32, suid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setresuid(ruid, euid, suid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setresgid(rgid: u32, egid: u32, sgid: u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setresgid(rgid, egid, sgid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getresuid(ruid: *mut u32, euid: *mut u32, suid: *mut u32) -> i32 {
    let ret = unsafe { trona_posix::posix_getresuid(ruid, euid, suid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getresgid(rgid: *mut u32, egid: *mut u32, sgid: *mut u32) -> i32 {
    let ret = unsafe { trona_posix::posix_getresgid(rgid, egid, sgid) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setgroups(size: i32, list: *const u32) -> i32 {
    let ret = unsafe { trona_posix::posix_setgroups(size as usize, list) };
    if ret < 0 { crate::errno::set_errno(-ret); return -1; }
    0
}

/// POSIX `initgroups(3)` — set supplementary group list from /etc/group.
///
/// Walks the group database and collects every gid where `user` appears as
/// a member, always prepending `group` (the primary gid) at position 0, then
/// invokes `setgroups()` with the deduplicated list. Caps at NGROUPS_MAX = 16
/// per Linux/POSIX.1-2008.
///
/// errno mapping:
/// - `EINVAL (22)` — `user` is null
/// - kernel errno passthrough — from `setgroups`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn initgroups(user: *const u8, group: u32) -> i32 {
    const NGROUPS_MAX: usize = 16;
    const EINVAL: i32 = 22;

    if user.is_null() {
        crate::errno::set_errno(EINVAL);
        return -1;
    }

    let mut gids = [0u32; NGROUPS_MAX];
    // groups_for_user guarantees: gids[0] = group, no duplicates, count >= 1.
    let count = unsafe { crate::pwd::groups_for_user(user, group, &mut gids) };
    if count == 0 {
        // Empty list is invalid for setgroups — at minimum the primary gid
        // must be present. This can only happen if out.is_empty() which we
        // prevent above, but guard defensively.
        crate::errno::set_errno(EINVAL);
        return -1;
    }

    let ret = unsafe { trona_posix::posix_setgroups(count, gids.as_ptr()) };
    if ret < 0 {
        crate::errno::set_errno(-ret);
        return -1;
    }
    0
}

// ---------------------------------------------------------------------------
// Priority (stubs — SaltyOS uses EDF scheduling, not nice values)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpriority(_which: i32, _who: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setpriority(_which: i32, _who: i32, _prio: i32) -> i32 {
    0
}

// ---------------------------------------------------------------------------
// User shell enumeration (stub — returns /bin/sh then NULL)
// ---------------------------------------------------------------------------

static USERSHELL_SHELLS: [&[u8]; 2] = [b"/bin/sh\0", b"/bin/bash\0"];
static mut USERSHELL_INDEX: usize = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getusershell() -> *mut u8 {
    unsafe {
        let idx = core::ptr::addr_of!(USERSHELL_INDEX).read_volatile();
        if idx < USERSHELL_SHELLS.len() {
            core::ptr::addr_of_mut!(USERSHELL_INDEX).write_volatile(idx + 1);
            USERSHELL_SHELLS[idx].as_ptr() as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setusershell() {
    unsafe {
        core::ptr::addr_of_mut!(USERSHELL_INDEX).write_volatile(0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn endusershell() {
    unsafe {
        core::ptr::addr_of_mut!(USERSHELL_INDEX).write_volatile(0);
    }
}

// ---------------------------------------------------------------------------
// posix_spawn — fork+exec with file actions and attributes
// ---------------------------------------------------------------------------

// File action types (must match spawn.h)
const SPAWN_ACTION_CLOSE: i32 = 0;
const SPAWN_ACTION_DUP2: i32 = 1;
const SPAWN_ACTION_OPEN: i32 = 2;

// Spawn attribute flags
const POSIX_SPAWN_SETSIGDEF: i16 = 0x04;
const POSIX_SPAWN_SETSIGMASK: i16 = 0x08;

const SPAWN_MAX_FILE_ACTIONS: usize = 16;

#[repr(C)]
struct SpawnFileAction {
    action_type: i32,
    fd: i32,
    newfd: i32,
    path: *const u8,
    oflag: i32,
    mode: u32,
}

#[repr(C)]
struct SpawnFileActions {
    count: i32,
    actions: [SpawnFileAction; SPAWN_MAX_FILE_ACTIONS],
}

#[repr(C)]
struct SpawnAttr {
    flags: i16,
    pgroup: i32,
    sigdefault: u32,
    sigmask: u32,
    schedpolicy: i32,
    schedparam_priority: i32,
}

/// Core posix_spawn implementation (fork + file_actions + attrp + exec).
///
/// Returns 0 on success (child PID written to `*pid`), or an error code on
/// failure. Unlike most POSIX functions, posix_spawn returns the error code
/// directly (not via errno).
unsafe fn do_posix_spawn(
    pid: *mut i32,
    path: *const u8,
    file_actions: *const SpawnFileActions,
    attrp: *const SpawnAttr,
    argv: *const *const u8,
    envp: *const *const u8,
    use_path: bool,
) -> i32 {
    unsafe {
        if path.is_null() {
            return errno::EINVAL;
        }

        let child = trona_posix::posix_fork();
        if child < 0 {
            return -child; // return error code directly
        }

        if child == 0 {
            // === Child process ===

            // Apply spawn attributes
            if !attrp.is_null() {
                let attr = &*attrp;

                // Set signal mask
                if (attr.flags & POSIX_SPAWN_SETSIGMASK) != 0 {
                    let mask = attr.sigmask;
                    crate::signal::sigprocmask(
                        crate::signal::SIG_SETMASK,
                        &mask as *const u32 as *const crate::signal::Sigset,
                        core::ptr::null_mut(),
                    );
                }

                // Reset signal defaults
                if (attr.flags & POSIX_SPAWN_SETSIGDEF) != 0 {
                    for sig in 1..32i32 {
                        if (attr.sigdefault & (1u32 << sig)) != 0 {
                            crate::signal::signal(sig, crate::signal::SIG_DFL);
                        }
                    }
                }
            }

            // Apply file actions
            if !file_actions.is_null() {
                let fa = &*file_actions;
                for i in 0..fa.count as usize {
                    if i >= SPAWN_MAX_FILE_ACTIONS {
                        break;
                    }
                    let action = &fa.actions[i];
                    match action.action_type {
                        SPAWN_ACTION_CLOSE => {
                            trona_posix::posix_close(action.fd);
                        }
                        SPAWN_ACTION_DUP2 => {
                            crate::unistd::dup2(action.fd, action.newfd);
                        }
                        SPAWN_ACTION_OPEN => {
                            let fd = trona_posix::posix_open(
                                action.path,
                                action.oflag,
                                action.mode,
                            );
                            if fd >= 0 && fd != action.fd {
                                crate::unistd::dup2(fd, action.fd);
                                trona_posix::posix_close(fd);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Exec
            if use_path {
                execvp(path, argv);
            } else {
                execve(path, argv, envp);
            }

            // exec failed — exit with 127
            trona_posix::posix_exit(127);
        }

        // === Parent process ===
        if !pid.is_null() {
            *pid = child;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut i32,
    path: *const u8,
    file_actions: *const SpawnFileActions,
    attrp: *const SpawnAttr,
    argv: *const *const u8,
    envp: *const *const u8,
) -> i32 {
    unsafe { do_posix_spawn(pid, path, file_actions, attrp, argv, envp, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut i32,
    file: *const u8,
    file_actions: *const SpawnFileActions,
    attrp: *const SpawnAttr,
    argv: *const *const u8,
    envp: *const *const u8,
) -> i32 {
    unsafe { do_posix_spawn(pid, file, file_actions, attrp, argv, envp, true) }
}

// ---------------------------------------------------------------------------
// posix_spawnattr_* — attribute manipulation
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_init(attrp: *mut SpawnAttr) -> i32 {
    if attrp.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*attrp).flags = 0;
        (*attrp).pgroup = 0;
        (*attrp).sigdefault = 0;
        (*attrp).sigmask = 0;
        (*attrp).schedpolicy = 0;
        (*attrp).schedparam_priority = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_destroy(_attrp: *mut SpawnAttr) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setflags(attrp: *mut SpawnAttr, flags: i16) -> i32 {
    if attrp.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*attrp).flags = flags;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getflags(attrp: *const SpawnAttr, flags: *mut i16) -> i32 {
    if attrp.is_null() || flags.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        *flags = (*attrp).flags;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setsigmask(
    attrp: *mut SpawnAttr,
    sigmask: *const u32,
) -> i32 {
    if attrp.is_null() || sigmask.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*attrp).sigmask = *sigmask;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getsigmask(
    attrp: *const SpawnAttr,
    sigmask: *mut u32,
) -> i32 {
    if attrp.is_null() || sigmask.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        *sigmask = (*attrp).sigmask;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setsigdefault(
    attrp: *mut SpawnAttr,
    sigdefault: *const u32,
) -> i32 {
    if attrp.is_null() || sigdefault.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*attrp).sigdefault = *sigdefault;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getsigdefault(
    attrp: *const SpawnAttr,
    sigdefault: *mut u32,
) -> i32 {
    if attrp.is_null() || sigdefault.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        *sigdefault = (*attrp).sigdefault;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setpgroup(attrp: *mut SpawnAttr, pgroup: i32) -> i32 {
    if attrp.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*attrp).pgroup = pgroup;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getpgroup(
    attrp: *const SpawnAttr,
    pgroup: *mut i32,
) -> i32 {
    if attrp.is_null() || pgroup.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        *pgroup = (*attrp).pgroup;
    }
    0
}

// ---------------------------------------------------------------------------
// posix_spawnattr sched — scheduler policy/param (stubs, SaltyOS uses EDF)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setschedpolicy(
    attrp: *mut SpawnAttr,
    policy: i32,
) -> i32 {
    if attrp.is_null() {
        return errno::EINVAL;
    }
    unsafe { (*attrp).schedpolicy = policy; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getschedpolicy(
    attrp: *const SpawnAttr,
    policy: *mut i32,
) -> i32 {
    if attrp.is_null() || policy.is_null() {
        return errno::EINVAL;
    }
    unsafe { *policy = (*attrp).schedpolicy; }
    0
}

#[repr(C)]
pub struct SchedParam {
    pub sched_priority: i32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_setschedparam(
    attrp: *mut SpawnAttr,
    param: *const SchedParam,
) -> i32 {
    if attrp.is_null() || param.is_null() {
        return errno::EINVAL;
    }
    unsafe { (*attrp).schedparam_priority = (*param).sched_priority; }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawnattr_getschedparam(
    attrp: *const SpawnAttr,
    param: *mut SchedParam,
) -> i32 {
    if attrp.is_null() || param.is_null() {
        return errno::EINVAL;
    }
    unsafe { (*param).sched_priority = (*attrp).schedparam_priority; }
    0
}

// ---------------------------------------------------------------------------
// posix_spawn_file_actions_* — file action list manipulation
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn_file_actions_init(fact: *mut SpawnFileActions) -> i32 {
    if fact.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        (*fact).count = 0;
        // Zero-init actions array
        let ptr = (*fact).actions.as_mut_ptr() as *mut u8;
        let size = core::mem::size_of::<[SpawnFileAction; SPAWN_MAX_FILE_ACTIONS]>();
        for i in 0..size {
            *ptr.add(i) = 0;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn_file_actions_destroy(_fact: *mut SpawnFileActions) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn_file_actions_addclose(
    fact: *mut SpawnFileActions,
    fd: i32,
) -> i32 {
    if fact.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        let idx = (*fact).count as usize;
        if idx >= SPAWN_MAX_FILE_ACTIONS {
            return errno::ENOMEM;
        }
        (*fact).actions[idx].action_type = SPAWN_ACTION_CLOSE;
        (*fact).actions[idx].fd = fd;
        (*fact).actions[idx].newfd = -1;
        (*fact).actions[idx].path = core::ptr::null();
        (*fact).actions[idx].oflag = 0;
        (*fact).actions[idx].mode = 0;
        (*fact).count += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn_file_actions_adddup2(
    fact: *mut SpawnFileActions,
    fd: i32,
    newfd: i32,
) -> i32 {
    if fact.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        let idx = (*fact).count as usize;
        if idx >= SPAWN_MAX_FILE_ACTIONS {
            return errno::ENOMEM;
        }
        (*fact).actions[idx].action_type = SPAWN_ACTION_DUP2;
        (*fact).actions[idx].fd = fd;
        (*fact).actions[idx].newfd = newfd;
        (*fact).actions[idx].path = core::ptr::null();
        (*fact).actions[idx].oflag = 0;
        (*fact).actions[idx].mode = 0;
        (*fact).count += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_spawn_file_actions_addopen(
    fact: *mut SpawnFileActions,
    fd: i32,
    path: *const u8,
    oflag: i32,
    mode: u32,
) -> i32 {
    if fact.is_null() || path.is_null() {
        return errno::EINVAL;
    }
    unsafe {
        let idx = (*fact).count as usize;
        if idx >= SPAWN_MAX_FILE_ACTIONS {
            return errno::ENOMEM;
        }
        (*fact).actions[idx].action_type = SPAWN_ACTION_OPEN;
        (*fact).actions[idx].fd = fd;
        (*fact).actions[idx].newfd = -1;
        (*fact).actions[idx].path = path;
        (*fact).actions[idx].oflag = oflag;
        (*fact).actions[idx].mode = mode;
        (*fact).count += 1;
    }
    0
}
