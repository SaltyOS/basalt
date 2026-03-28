/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __STRINGS_H__
#define __STRINGS_H__

#include <stddef.h>
#include <sys/cdefs.h>

__BEGIN_DECLS

extern int  strcasecmp(const char *s1, const char *s2);
extern int  strncasecmp(const char *s1, const char *s2, size_t n);
extern void bcopy(const void *src, void *dest, size_t n);
extern void bzero(void *s, size_t n);
extern void explicit_bzero(void *s, size_t n);
extern int  ffs(int i);

__END_DECLS

#endif /* __STRINGS_H__ */
