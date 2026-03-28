/* SPDX-License-Identifier: GPL-2.0-only */
/* SaltyOS syscall numbers exposed to userspace.
 *
 * SaltyOS is a microkernel — file I/O goes through VFS IPC, not raw syscalls.
 * Only kernel-level primitives are exposed here.  Code that checks for
 * SYS_open / SYS_read / SYS_close will fall back to libc wrappers.
 */
#ifndef __SYS_SYSCALL_H__
#define __SYS_SYSCALL_H__

#define SYS_futex       18
#define SYS_getrandom   19
#define SYS_clock_gettime 12
#define SYS_nanosleep   13
#define SYS_yield       8
#define SYS_shutdown    20

#include <sys/cdefs.h>

__BEGIN_DECLS
extern long syscall(long number, ...);
__END_DECLS

/* POSIX file I/O syscalls are NOT available as raw syscalls on SaltyOS.
 * SYS_open, SYS_read, SYS_write, SYS_close, SYS_access etc. are
 * intentionally absent — use the libc wrappers which route through VFS IPC.
 */

#endif /* __SYS_SYSCALL_H__ */
