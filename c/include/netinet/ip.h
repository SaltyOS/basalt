/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NETINET_IP_H__
#define __NETINET_IP_H__

#include <stdint.h>
#include <netinet/in.h>

struct ip {
    uint8_t  ip_hl:4, ip_v:4;
    uint8_t  ip_tos;
    uint16_t ip_len;
    uint16_t ip_id;
    uint16_t ip_off;
    uint8_t  ip_ttl;
    uint8_t  ip_p;
    uint16_t ip_sum;
    struct in_addr ip_src, ip_dst;
};

#define IPVERSION   4
#define IP_MAXPACKET 65535

#define IPOPT_EOL   0
#define IPOPT_NOP   1
#define IPOPT_RR    7
#define IPOPT_TS    68
#define IPOPT_LSRR  131
#define IPOPT_SSRR  137

#define MAXTTL      255
#define IPDEFTTL    64
#define IP_MSS      576

#endif /* __NETINET_IP_H__ */
