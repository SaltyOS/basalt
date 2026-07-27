/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_STATVFS_H__
#define __SYS_STATVFS_H__

#include <sys/types.h>
#include <sys/cdefs.h>
#include <sys/mount.h>

struct statvfs {
    unsigned long f_bsize;
    unsigned long f_frsize;
    unsigned long f_blocks;
    unsigned long f_bfree;
    unsigned long f_bavail;
    unsigned long f_files;
    unsigned long f_ffree;
    unsigned long f_favail;
    unsigned long f_fsid;
    unsigned long f_flag;
    unsigned long f_namemax;
};

/* BSD/Linux compat: some software uses f_flags instead of POSIX f_flag */
#define f_flags f_flag

/* POSIX ST_* mount flags (returned in f_flag). */
#define ST_RDONLY      0x0001
#define ST_NOSUID      0x0002
#define ST_NODEV       0x0004
#define ST_NOEXEC      0x0008
#define ST_SYNCHRONOUS 0x0010
#define ST_MANDLOCK    0x0040
#define ST_WRITE       0x0080
#define ST_APPEND      0x0100
#define ST_IMMUTABLE   0x0200
#define ST_NOATIME     0x0400
#define ST_NODIRATIME  0x0800
#define ST_RELATIME    0x1000

__BEGIN_DECLS

extern int statvfs(const char *path, struct statvfs *buf);
extern int fstatvfs(int fd, struct statvfs *buf);

__END_DECLS

#endif /* __SYS_STATVFS_H__ */
