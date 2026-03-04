/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_UTSNAME_H__
#define __SYS_UTSNAME_H__

#include <sys/cdefs.h>

struct utsname {
    char sysname[65];
    char nodename[65];
    char release[65];
    char version[65];
    char machine[65];
};

__BEGIN_DECLS

extern int uname(struct utsname *buf);

__END_DECLS

#endif /* __SYS_UTSNAME_H__ */
