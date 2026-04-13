/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __TTYENT_H__
#define __TTYENT_H__

#include <sys/cdefs.h>

#define TTY_ON 0x01
#define TTY_SECURE 0x02

struct ttyent {
    char *ty_name;
    char *ty_getty;
    char *ty_type;
    int ty_status;
    char *ty_window;
    char *ty_comment;
    char *ty_group;
};

__BEGIN_DECLS

extern struct ttyent *getttynam(const char *tty);

__END_DECLS

#endif
