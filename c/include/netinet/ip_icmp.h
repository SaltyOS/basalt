/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __NETINET_IP_ICMP_H__
#define __NETINET_IP_ICMP_H__

#include <stdint.h>
#include <netinet/ip.h>

struct icmp {
    uint8_t  icmp_type;
    uint8_t  icmp_code;
    uint16_t icmp_cksum;
    union {
        struct {
            uint16_t id;
            uint16_t seq;
        } echo;
        uint32_t gateway;
        struct {
            uint16_t __unused;
            uint16_t mtu;
        } frag;
    } icmp_hun;
    union {
        struct {
            uint32_t ts_otime;
            uint32_t ts_rtime;
            uint32_t ts_ttime;
        } ts;
        uint8_t data[1];
        struct ip ip;
    } icmp_dun;
};

#define icmp_id     icmp_hun.echo.id
#define icmp_seq    icmp_hun.echo.seq
#define icmp_gwaddr icmp_hun.gateway
#define icmp_pmtu   icmp_hun.frag.mtu
#define icmp_otime  icmp_dun.ts.ts_otime
#define icmp_rtime  icmp_dun.ts.ts_rtime
#define icmp_ttime  icmp_dun.ts.ts_ttime
#define icmp_ip     icmp_dun.ip
#define icmp_data   icmp_dun.data

/* ICMP types */
#define ICMP_ECHOREPLY      0
#define ICMP_UNREACH        3
#define ICMP_SOURCEQUENCH   4
#define ICMP_REDIRECT       5
#define ICMP_ECHO           8
#define ICMP_ROUTERADVERT   9
#define ICMP_ROUTERSOLICIT  10
#define ICMP_TIMXCEED       11
#define ICMP_PARAMPROB      12
#define ICMP_TSTAMP         13
#define ICMP_TSTAMPREPLY    14
#define ICMP_IREQ           15
#define ICMP_IREQREPLY      16
#define ICMP_MASKREQ        17
#define ICMP_MASKREPLY      18
#define ICMP_MAXTYPE        18

/* ICMP unreach codes */
#define ICMP_UNREACH_NET            0
#define ICMP_UNREACH_HOST           1
#define ICMP_UNREACH_PROTOCOL       2
#define ICMP_UNREACH_PORT           3
#define ICMP_UNREACH_NEEDFRAG       4
#define ICMP_UNREACH_SRCFAIL        5
#define ICMP_UNREACH_NET_UNKNOWN    6
#define ICMP_UNREACH_HOST_UNKNOWN   7

/* Time exceeded codes */
#define ICMP_TIMXCEED_INTRANS   0
#define ICMP_TIMXCEED_REASS     1

#define ICMP_MINLEN 8

#endif /* __NETINET_IP_ICMP_H__ */
