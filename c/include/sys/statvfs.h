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

__BEGIN_DECLS

extern int statvfs(const char *path, struct statvfs *buf);
extern int fstatvfs(int fd, struct statvfs *buf);

__END_DECLS

#endif /* __SYS_STATVFS_H__ */
