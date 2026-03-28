/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NET_IF_H__
#define __NET_IF_H__

#include <sys/cdefs.h>
#include <sys/socket.h>

#define IFNAMSIZ     16
#define IF_NAMESIZE  16

#define IFF_UP         0x1
#define IFF_BROADCAST  0x2
#define IFF_RUNNING    0x40
#define IFF_MULTICAST  0x1000

struct if_nameindex {
    unsigned int if_index;
    char        *if_name;
};

struct ifreq {
    char ifr_name[IFNAMSIZ];
    union {
        struct sockaddr ifru_addr;
        struct sockaddr ifru_netmask;
        struct sockaddr ifru_broadaddr;
        short           ifru_flags;
        int             ifru_ifindex;
    } ifr_ifru;
};

#define ifr_addr      ifr_ifru.ifru_addr
#define ifr_netmask   ifr_ifru.ifru_netmask
#define ifr_broadaddr ifr_ifru.ifru_broadaddr
#define ifr_flags     ifr_ifru.ifru_flags
#define ifr_ifindex   ifr_ifru.ifru_ifindex

struct ifconf {
    int ifc_len;
    union {
        char         *ifcu_buf;
        struct ifreq *ifcu_req;
    } ifc_ifcu;
};

#define ifc_buf ifc_ifcu.ifcu_buf
#define ifc_req ifc_ifcu.ifcu_req

__BEGIN_DECLS

extern struct if_nameindex *if_nameindex(void);
extern void if_freenameindex(struct if_nameindex *ptr);
extern unsigned int if_nametoindex(const char *ifname);
extern char *if_indextoname(unsigned int ifindex, char *ifname);

__END_DECLS

#endif /* __NET_IF_H__ */
