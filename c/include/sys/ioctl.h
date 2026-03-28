/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_IOCTL_H__
#define __SYS_IOCTL_H__

#include <sys/cdefs.h>

#define TIOCGWINSZ  0x5413
#define TIOCSWINSZ  0x5414
#define FIONREAD    0x541B
#define TIOCSCTTY   0x540E
#define TIOCGPGRP   0x540F
#define TIOCSPGRP   0x5410
#define TIOCNOTTY   0x5422
#define SIOCGIFNAME   0x8910
#define SIOCGIFCONF   0x8912
#define SIOCGIFFLAGS  0x8913
#define SIOCGIFADDR   0x8915
#define SIOCGIFBRDADDR 0x8919
#define SIOCGIFNETMASK 0x891B
#define SIOCGIFINDEX  0x8933

struct winsize {
    unsigned short ws_row;
    unsigned short ws_col;
    unsigned short ws_xpixel;
    unsigned short ws_ypixel;
};

__BEGIN_DECLS

extern int ioctl(int fd, unsigned long request, ...);

__END_DECLS

#endif /* __SYS_IOCTL_H__ */
