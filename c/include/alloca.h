/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __ALLOCA_H__
#define __ALLOCA_H__

#include <stddef.h>

/* alloca is a compiler builtin in clang/gcc */
#define alloca(size) __builtin_alloca(size)

#endif /* __ALLOCA_H__ */
