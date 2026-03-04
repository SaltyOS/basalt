/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __UTIME_H__
#define __UTIME_H__

#include <sys/types.h>
#include <sys/cdefs.h>

struct utimbuf {
    long actime;
    long modtime;
};

__BEGIN_DECLS

extern int utime(const char *filename, const struct utimbuf *times);

__END_DECLS

#endif /* __UTIME_H__ */
