/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_SYSMACROS_H__
#define __SYS_SYSMACROS_H__

/* Linux glibc's device-id encoding macros. Matches glibc's split:
   major = bits 31:8, minor = bits 7:0 + bits 19:12 (extended). */

#define major(dev)            ((unsigned int)(((dev) >> 8) & 0xfffff000) | (((dev) >> 8) & 0xfff))
#define minor(dev)            ((unsigned int)(((dev) >> 12) & 0xffffff00) | ((dev) & 0xff))
#define makedev(maj, min) \
    ((unsigned long long) (((unsigned long long)((maj) & 0xfffff000ULL) << 32) | \
                           ((unsigned long long)((maj) & 0x00000fffULL) << 8) | \
                           ((unsigned long long)((min) & 0xffffff00ULL) << 12) | \
                           ((unsigned long long) (min) & 0xff)))

#endif /* __SYS_SYSMACROS_H__ */
