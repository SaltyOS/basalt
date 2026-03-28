/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __IFADDRS_H__
#define __IFADDRS_H__

#include <sys/cdefs.h>
#include <sys/socket.h>

struct ifaddrs {
    struct ifaddrs  *ifa_next;
    char            *ifa_name;
    unsigned int     ifa_flags;
    struct sockaddr *ifa_addr;
    struct sockaddr *ifa_netmask;
    union {
        struct sockaddr *ifu_broadaddr;
        struct sockaddr *ifu_dstaddr;
    } ifa_ifu;
    void            *ifa_data;
};

#define ifa_broadaddr ifa_ifu.ifu_broadaddr
#define ifa_dstaddr   ifa_ifu.ifu_dstaddr

__BEGIN_DECLS

extern int getifaddrs(struct ifaddrs **ifap);
extern void freeifaddrs(struct ifaddrs *ifa);

__END_DECLS

#endif /* __IFADDRS_H__ */
