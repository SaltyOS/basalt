/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * sys/cpuset.h — CPU affinity stubs for SaltyOS (FreeBSD compat)
 *
 * Provides the types and functions used by LLVM Threading.inc under __FreeBSD__
 * to query CPU affinity. SaltyOS always reports hardware_concurrency() to the
 * caller via the std::thread fallback; cpuset_getaffinity() returns ENOSYS.
 */
#ifndef __SYS_CPUSET_H__
#define __SYS_CPUSET_H__

#include <sys/types.h>
#include <sys/cdefs.h>
#include <errno.h>
#include <string.h>

/* cpuset_t — opaque bitmask, 256 bits (matches FreeBSD ABI) */
#define _NCPUWORDS   4
typedef struct {
    unsigned long __bits[_NCPUWORDS];
} cpuset_t;

/* CPU_* manipulation macros */
#define CPU_ZERO(s)      memset((s), 0, sizeof(*(s)))
#define CPU_SET(n, s)    ((s)->__bits[(n) / (8 * sizeof(unsigned long))] |=  \
                          (1UL << ((n) % (8 * sizeof(unsigned long)))))
#define CPU_CLR(n, s)    ((s)->__bits[(n) / (8 * sizeof(unsigned long))] &= \
                          ~(1UL << ((n) % (8 * sizeof(unsigned long)))))
#define CPU_ISSET(n, s)  (!!((s)->__bits[(n) / (8 * sizeof(unsigned long))] & \
                          (1UL << ((n) % (8 * sizeof(unsigned long))))))

static inline int
__cpuset_count(const cpuset_t *s)
{
    int count = 0;
    unsigned int i;
    for (i = 0; i < _NCPUWORDS; i++) {
        unsigned long w = s->__bits[i];
        while (w) { count += (int)(w & 1); w >>= 1; }
    }
    return count;
}
#define CPU_COUNT(s) __cpuset_count(s)

/* cpuset_getaffinity level / which constants (FreeBSD ABI) */
#define CPU_LEVEL_WHICH   3
#define CPU_WHICH_TID     1
#define CPU_WHICH_PID     2
#define CPU_WHICH_CPUSET  3

__BEGIN_DECLS

/*
 * Retrieve the CPU affinity mask for the entity identified by (level, which, id).
 * SaltyOS does not implement CPU pinning; always returns ENOSYS.
 */
static inline int
cpuset_getaffinity(int level, int which, long id,
                   unsigned long setsize, cpuset_t *mask)
{
    (void)level; (void)which; (void)id; (void)setsize; (void)mask;
    errno = ENOSYS;
    return -1;
}

__END_DECLS

#endif /* __SYS_CPUSET_H__ */
