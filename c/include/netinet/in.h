/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NETINET_IN_H__
#define __NETINET_IN_H__

#include <sys/types.h>
#include <sys/endian.h>

typedef unsigned short in_port_t;
typedef unsigned int   in_addr_t;

struct in_addr {
    in_addr_t s_addr;
};

struct sockaddr_in {
    unsigned short  sin_family;
    in_port_t       sin_port;
    struct in_addr  sin_addr;
    unsigned char   sin_zero[8];
};

/* IPv6 stubs for compatibility */
struct in6_addr {
    unsigned char s6_addr[16];
};

struct sockaddr_in6 {
    unsigned short  sin6_family;
    in_port_t       sin6_port;
    unsigned int    sin6_flowinfo;
    struct in6_addr sin6_addr;
    unsigned int    sin6_scope_id;
};

/* Protocol numbers */
#define IPPROTO_IP   0
#define IPPROTO_TCP  6
#define IPPROTO_UDP  17

/* Special addresses */
#define INADDR_ANY       ((in_addr_t)0x00000000)
#define INADDR_LOOPBACK  ((in_addr_t)0x7f000001)
#define INADDR_BROADCAST ((in_addr_t)0xffffffff)

/* Address string lengths (including NUL terminator) */
#define INET_ADDRSTRLEN   16
#define INET6_ADDRSTRLEN  46

#endif /* __NETINET_IN_H__ */
