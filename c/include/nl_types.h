/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NL_TYPES_H__
#define __NL_TYPES_H__

#include <sys/cdefs.h>

typedef long nl_catd;
typedef int  nl_item;

#define NL_SETD   1
#define NL_CAT_LOCALE 1

__BEGIN_DECLS

extern nl_catd catopen(const char *name, int oflag);
extern char   *catgets(nl_catd catd, int set_id, int msg_id, const char *s);
extern int     catclose(nl_catd catd);

__END_DECLS

#endif /* __NL_TYPES_H__ */
