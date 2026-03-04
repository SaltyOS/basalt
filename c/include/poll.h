/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __POLL_H__
#define __POLL_H__

#include <sys/cdefs.h>

#define POLLIN   0x001
#define POLLPRI  0x002
#define POLLOUT  0x004
#define POLLERR  0x008
#define POLLHUP  0x010
#define POLLNVAL 0x020

struct pollfd {
    int   fd;
    short events;
    short revents;
};

typedef unsigned long nfds_t;

__BEGIN_DECLS

extern int poll(struct pollfd *fds, nfds_t nfds, int timeout);

__END_DECLS

#endif /* __POLL_H__ */
