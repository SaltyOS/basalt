/* SPDX-License-Identifier: GPL-2.0-only */
/* mount/statfs — backed by VFS_STATFS / VFS_MOUNT_LIST IPC */
#ifndef __SYS_MOUNT_H__
#define __SYS_MOUNT_H__

#include <sys/types.h>
#include <sys/cdefs.h>

#define MFSNAMELEN  16
#define MNAMELEN    88
#define MAXPATHLEN  4096

#define MNT_RDONLY      0x00000001
#define MNT_NOSUID      0x00000008
#define MNT_NOEXEC      0x00000004
#define MNT_LOCAL       0x00001000

/* Linux-style mount(2) flags. Only the subset SaltyOS honors today. */
#define MS_RDONLY       0x00000001
#define MS_NOSUID       0x00000002
#define MS_NODEV        0x00000004
#define MS_NOEXEC       0x00000008
#define MS_REMOUNT      0x00000020
#define MS_NOATIME      0x00000400

struct statfs {
    unsigned int  f_version;
    unsigned int  f_type;
    unsigned long f_flags;
    unsigned long f_bsize;
    unsigned long f_iosize;
    unsigned long f_blocks;
    unsigned long f_bfree;
    long          f_bavail;
    unsigned long f_files;
    long          f_ffree;
    unsigned long f_syncwrites;
    unsigned long f_asyncwrites;
    unsigned long f_syncreads;
    unsigned long f_asyncreads;
    unsigned long f_namemax;
    uid_t         f_owner;
    int           f_fsid;
    char          f_fstypename[MFSNAMELEN];
    char          f_mntfromname[MNAMELEN];
    char          f_mntonname[MNAMELEN];
};

__BEGIN_DECLS

extern int statfs(const char *path, struct statfs *buf);
extern int fstatfs(int fd, struct statfs *buf);
extern int getmntinfo(struct statfs **mntbufp, int mode);

/* Linux-style mount(2) / umount(2). `data` carries comma-separated
 * options (the SaltyOS VFS server treats it as a byte slice; size= and
 * nr_inodes= are the tmpfs-specific tokens). When `mountflags`
 * contains `MS_REMOUNT`, the call lands on the VFS remount path
 * instead. */
extern int mount(const char *source, const char *target,
                 const char *filesystemtype, unsigned long mountflags,
                 const void *data);
extern int umount(const char *target);
extern int umount2(const char *target, int flags);

__END_DECLS

#endif /* __SYS_MOUNT_H__ */
