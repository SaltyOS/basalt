/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __STDDEF_H__
#define __STDDEF_H__

typedef unsigned long size_t;
typedef long ptrdiff_t;
#ifndef __cplusplus
typedef __WCHAR_TYPE__ wchar_t;
#endif
#ifndef _WINT_T_DEFINED
#define _WINT_T_DEFINED
typedef unsigned int wint_t;
#endif

#define NULL ((void *)0)
#define offsetof(type, member) __builtin_offsetof(type, member)

#endif /* __STDDEF_H__ */
