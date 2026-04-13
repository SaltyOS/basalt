/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef _SHADOW_H
#define _SHADOW_H

#include <sys/types.h>
#include <sys/cdefs.h>

struct spwd {
    char           *sp_namp;
    char           *sp_pwdp;
    long            sp_lstchg;
    long            sp_min;
    long            sp_max;
    long            sp_warn;
    long            sp_inact;
    long            sp_expire;
    unsigned long   sp_flag;
};

__BEGIN_DECLS

extern struct spwd *getspnam(const char *name);
extern struct spwd *getspent(void);
extern void         setspent(void);
extern void         endspent(void);

__END_DECLS

#endif /* _SHADOW_H */
