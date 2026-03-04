/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_UMTX_H__
#define __SYS_UMTX_H__

#include <sys/types.h>
#include <sys/cdefs.h>

#define UMTX_OP_WAIT   2
#define UMTX_OP_WAKE   3

__BEGIN_DECLS
extern int _umtx_op(void *obj, int op, unsigned long val,
                    void *uaddr, void *uaddr2);
__END_DECLS

#endif /* __SYS_UMTX_H__ */
