/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef _SYS_RANDOM_H
#define _SYS_RANDOM_H

#include <sys/types.h>
#include <sys/cdefs.h>

/* Flags for getrandom(2) */
#define GRND_NONBLOCK   0x0001  /* Don't block; return EAGAIN instead */
#define GRND_RANDOM     0x0002  /* Use /dev/random pool (ignored on SaltyOS) */
#define GRND_INSECURE   0x0004  /* Return non-cryptographic random bytes */

__BEGIN_DECLS

extern ssize_t getrandom(void *buf, size_t buflen, unsigned int flags);
extern int getentropy(void *buf, size_t buflen);

__END_DECLS

#endif /* _SYS_RANDOM_H */
