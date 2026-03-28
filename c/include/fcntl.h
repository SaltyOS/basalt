/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __FCNTL_H__
#define __FCNTL_H__
/* FreeBSD-compatible guard name.
 * We set _SYS_FCNTL_H_ so FreeBSD's sys/fcntl.h is suppressed when we are
 * included first. We also set __BESALTC_FCNTL_PROVIDES_FLOCK__ so that
 * struct flock is only emitted when we are first (not when FreeBSD's header
 * already provided it). */
#ifndef _SYS_FCNTL_H_
#define _SYS_FCNTL_H_
#define __BESALTC_FCNTL_PROVIDES_FLOCK__
#endif

#include <sys/types.h>
#include <sys/cdefs.h>

#define O_RDONLY    0x0000
#define O_WRONLY    0x0001
#define O_RDWR      0x0002
#define O_CREAT     0x0040
#define O_TRUNC     0x0200
#define O_APPEND    0x0400
#define O_NONBLOCK  0x0800
#define O_EXCL      0x0080
#define O_NOCTTY    0x0100
#define O_CLOEXEC   0x80000
#define O_DIRECTORY 0x10000
#define O_NOFOLLOW  0x20000

#define O_ACCMODE   (O_RDONLY | O_WRONLY | O_RDWR)

#define F_DUPFD     0
#define F_GETFD     1
#define F_SETFD     2
#define F_GETFL     3
#define F_SETFL     4
#define F_GETLK     5
#define F_SETLK     6
#define F_SETLKW    7

#define F_RDLCK     0
#define F_WRLCK     1
#define F_UNLCK     2

#define FD_CLOEXEC  1

/* POSIX advisory file lock — only provided when we were included before
 * FreeBSD's sys/fcntl.h (which defines struct flock unconditionally). */
#ifdef __BESALTC_FCNTL_PROVIDES_FLOCK__
struct flock {
    short  l_type;    /* F_RDLCK, F_WRLCK, or F_UNLCK */
    short  l_whence;  /* SEEK_SET, SEEK_CUR, or SEEK_END */
    off_t  l_start;   /* byte offset to start of lock region */
    off_t  l_len;     /* length of lock region (0 = to EOF) */
    pid_t  l_pid;     /* PID of process holding lock (F_GETLK only) */
};
#endif /* __BESALTC_FCNTL_PROVIDES_FLOCK__ */

#define AT_FDCWD            (-100)
#define AT_SYMLINK_NOFOLLOW 0x100
#define AT_REMOVEDIR        0x200
#define AT_SYMLINK_FOLLOW   0x400
#define AT_EACCESS          0x200
#define AT_EMPTY_PATH       0x1000

__BEGIN_DECLS

extern int open(const char *pathname, int flags, ...);
extern int openat(int dirfd, const char *pathname, int flags, ...);
extern int creat(const char *pathname, mode_t mode);
extern int fcntl(int fd, int cmd, ...);

__END_DECLS

#endif /* __FCNTL_H__ */
