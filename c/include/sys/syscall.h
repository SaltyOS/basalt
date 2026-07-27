/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * SaltyOS does not expose a Linux-style `syscall(2)` C ABI. The
 * kernite microkernel routes every kernel operation through a single
 * `KERNITE_SYS_INVOKE` trap against a capability + invoke label pair;
 * there is no stable mapping from a Linux/glibc syscall number to a
 * kernite operation, so the historical `extern long syscall(long, ...)`
 * symbol is intentionally absent.
 *
 * Programs that historically reached for `syscall(SYS_futex, ...)` or
 * similar must call the substrate / posix wrappers directly — e.g.
 * `pthread_*` for thread synchronisation, `clock_gettime` /
 * `nanosleep` for time, `getrandom` for entropy, or the appropriate
 * capability invocation surface for kernel-privileged operations.
 */
#ifndef __SYS_SYSCALL_H__
#define __SYS_SYSCALL_H__

#endif /* __SYS_SYSCALL_H__ */
