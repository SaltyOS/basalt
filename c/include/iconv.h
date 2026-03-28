/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __ICONV_H__
#define __ICONV_H__

#include <stddef.h>
#include <sys/cdefs.h>

typedef void *iconv_t;

__BEGIN_DECLS

extern iconv_t iconv_open(const char *tocode, const char *fromcode);
extern size_t  iconv(iconv_t cd, char **inbuf, size_t *inbytesleft,
                     char **outbuf, size_t *outbytesleft);
extern int     iconv_close(iconv_t cd);

__END_DECLS

#endif /* __ICONV_H__ */
