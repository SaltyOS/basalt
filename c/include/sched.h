/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SCHED_H__
#define __SCHED_H__

#include <sys/cdefs.h>

#ifndef __sched_param_defined
#define __sched_param_defined
struct sched_param {
    int sched_priority;
};
#endif

__BEGIN_DECLS

extern int sched_yield(void);

__END_DECLS

#endif /* __SCHED_H__ */
