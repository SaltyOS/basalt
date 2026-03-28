/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_SELECT_H__
#define __SYS_SELECT_H__

#include <sys/types.h>
#include <sys/time.h>
#include <time.h>
#include <signal.h>
#include <sys/cdefs.h>

/* fd_set, FD_SETSIZE, FD_* macros are in <sys/types.h> */

__BEGIN_DECLS

extern int select(int nfds, fd_set *readfds, fd_set *writefds,
                  fd_set *exceptfds, struct timeval *timeout);
extern int pselect(int nfds, fd_set *readfds, fd_set *writefds,
                   fd_set *exceptfds, const struct timespec *timeout,
                   const sigset_t *sigmask);

__END_DECLS

#endif /* __SYS_SELECT_H__ */
