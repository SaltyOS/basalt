/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __ARPA_INET_H__
#define __ARPA_INET_H__

#include <sys/types.h>
#include <sys/cdefs.h>
#include <sys/socket.h>
#include <netinet/in.h>

__BEGIN_DECLS

extern const char *inet_ntop(int af, const void *src, char *dst, socklen_t size);
extern int         inet_pton(int af, const char *src, void *dst);
extern in_addr_t   inet_addr(const char *cp);
extern int         inet_aton(const char *cp, struct in_addr *inp);
extern char       *inet_ntoa(struct in_addr in);

__END_DECLS

#endif /* __ARPA_INET_H__ */
