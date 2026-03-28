/* SPDX-License-Identifier: GPL-2.0-only */
/* FreeBSD NUMA domainset stubs for SaltyOS */
#ifndef __SYS_DOMAINSET_H__
#define __SYS_DOMAINSET_H__

#include <sys/types.h>

#define MAXMEMDOM 1

typedef struct {
    unsigned long __bits[1];
} domainset_t;

#define DOMAINSET_ISSET(n, set) (((set)->__bits[0] >> (n)) & 1)
#define DOMAINSET_SET(n, set)   ((set)->__bits[0] |= (1UL << (n)))
#define DOMAINSET_CLR(n, set)   ((set)->__bits[0] &= ~(1UL << (n)))
#define DOMAINSET_ZERO(set)     ((set)->__bits[0] = 0)

#define CPU_LEVEL_CPUSET 3
#define CPU_WHICH_PID    2

static inline int cpuset_getdomain(int level, int which, int id,
    size_t setsize, domainset_t *mask, int *policy)
{
    (void)level; (void)which; (void)id; (void)setsize;
    if (mask) { mask->__bits[0] = 1; } /* node 0 */
    if (policy) { *policy = 0; }
    return 0;
}

#endif /* __SYS_DOMAINSET_H__ */
