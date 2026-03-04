/* SPDX-License-Identifier: GPL-2.0-only */
/* No include guard — assert.h must be re-includable per C standard. */

#include <sys/cdefs.h>

__BEGIN_DECLS
extern void abort(void);
__END_DECLS

#undef assert

#ifdef NDEBUG
#define assert(expr) ((void)0)
#else
#define assert(expr) \
    ((expr) ? (void)0 : abort())
#endif
