/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NETDB_H__
#define __NETDB_H__

#include <sys/types.h>
#include <sys/cdefs.h>
#include <sys/socket.h>

/* Flags for getaddrinfo() */
#define AI_PASSIVE      1
#define AI_CANONNAME    2
#define AI_NUMERICHOST  4
#define AI_NUMERICSERV  0x400
#define AI_ADDRCONFIG   0x20

/* Error codes for getaddrinfo() / getnameinfo() */
#define EAI_AGAIN       2
#define EAI_BADFLAGS    3
#define EAI_FAIL        4
#define EAI_FAMILY      5
#define EAI_MEMORY      6
#define EAI_NONAME      8
#define EAI_SERVICE     9
#define EAI_SOCKTYPE    10
#define EAI_SYSTEM      11
#define EAI_OVERFLOW    14

/* Limits for getnameinfo() */
#define NI_MAXHOST      1025
#define NI_MAXSERV      32
#define NI_NUMERICHOST  1
#define NI_NUMERICSERV  2

struct addrinfo {
    int              ai_flags;
    int              ai_family;
    int              ai_socktype;
    int              ai_protocol;
    socklen_t        ai_addrlen;
    struct sockaddr *ai_addr;
    char            *ai_canonname;
    struct addrinfo *ai_next;
};

__BEGIN_DECLS

extern int   getaddrinfo(const char *node, const char *service,
                         const struct addrinfo *hints, struct addrinfo **res);
extern void  freeaddrinfo(struct addrinfo *res);
extern const char *gai_strerror(int errcode);
extern int   getnameinfo(const struct sockaddr *sa, socklen_t salen,
                         char *host, socklen_t hostlen,
                         char *serv, socklen_t servlen, int flags);

__END_DECLS

#endif /* __NETDB_H__ */
