/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * pthread_np.h — BSD non-portable pthread extensions for SaltyOS
 *
 * Provides the minimal set of _np (non-portable) thread functions expected
 * by FreeBSD-targeted code. SaltyOS implements only what is actually used
 * by LLVM's Threading.inc under __FreeBSD__.
 */
#ifndef _PTHREAD_NP_H_
#define _PTHREAD_NP_H_

#include <pthread.h>

__BEGIN_DECLS

/* Return a unique integer ID for the calling thread (like Linux gettid()). */
extern int pthread_getthreadid_np(void);

/* Set the name of the given thread. Name is truncated to implementation limit. */
extern void pthread_set_name_np(pthread_t thread, const char *name);

/* Get the name of the given thread into buf (at most len bytes including NUL). */
extern void pthread_get_name_np(pthread_t thread, char *name, size_t len);

__END_DECLS

#endif /* _PTHREAD_NP_H_ */
