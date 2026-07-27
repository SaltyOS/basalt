/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_STATFS_H__
#define __SYS_STATFS_H__

/* Linux <sys/statfs.h> is the same surface as BSD-style statfs in
   <sys/mount.h>, which basaltc already provides. Forward the declaration
   so Linux-targeted ports compile without duplicating the type. */
#include <sys/mount.h>

#endif /* __SYS_STATFS_H__ */
