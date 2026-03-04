/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * sys/user.h — minimal kinfo_proc stub for SaltyOS (FreeBSD compat)
 *
 * LLVM Threading.inc uses kinfo_proc under __FreeBSD__ to retrieve the thread
 * name via sysctl KERN_PROC_PID | KERN_PROC_INC_THREAD. SaltyOS's sysctl()
 * returns ENOSYS, so this code path is never actually executed; we only need
 * the struct to compile.
 */
#ifndef __SYS_USER_H__
#define __SYS_USER_H__

#include <sys/types.h>
#include <sys/cdefs.h>

/* Thread ID type (BSD lwpid_t) */
typedef int lwpid_t;

/* Minimum kinfo_proc required by LLVM Threading.inc */
#define TDNAMLEN 16

struct kinfo_proc {
    lwpid_t ki_tid;              /* thread ID */
    char    ki_tdname[TDNAMLEN + 1]; /* thread name */
    /* other fields omitted — not accessed by LLVM */
};

#endif /* __SYS_USER_H__ */
