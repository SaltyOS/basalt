/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __STDINT_H__
#define __STDINT_H__

typedef signed char        int8_t;
typedef short              int16_t;
typedef int                int32_t;
typedef long               int64_t;

typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long      uint64_t;

typedef long               intptr_t;
typedef unsigned long      uintptr_t;

typedef long               intmax_t;
typedef unsigned long      uintmax_t;

/* Minimum-width integer types (C99 7.18.1.2)
 * Use compiler predefined macros so definitions always agree with
 * any other header (FreeBSD sys/stdint.h, clang stdint.h, etc.). */
typedef __INT_LEAST8_TYPE__   int_least8_t;
typedef __INT_LEAST16_TYPE__  int_least16_t;
typedef __INT_LEAST32_TYPE__  int_least32_t;
typedef __INT_LEAST64_TYPE__  int_least64_t;
typedef __UINT_LEAST8_TYPE__  uint_least8_t;
typedef __UINT_LEAST16_TYPE__ uint_least16_t;
typedef __UINT_LEAST32_TYPE__ uint_least32_t;
typedef __UINT_LEAST64_TYPE__ uint_least64_t;

/* Fastest minimum-width integer types (C99 7.18.1.3)
 * Explicit types matching glibc/FreeBSD x86_64 convention (int-promoted).
 * DO NOT use __INT_FAST*_TYPE__ — clang maps them to minimal-width types
 * (signed char, short) for unknown OS targets, conflicting with FreeBSD
 * headers that use int for 8/16/32-bit fast types. C11 allows compatible
 * redefinition, so using the same underlying type as FreeBSD is safe. */
typedef int           int_fast8_t;
typedef int           int_fast16_t;
typedef int           int_fast32_t;
typedef long          int_fast64_t;
typedef unsigned int  uint_fast8_t;
typedef unsigned int  uint_fast16_t;
typedef unsigned int  uint_fast32_t;
typedef unsigned long uint_fast64_t;

#define INT8_MIN    (-128)
#define INT8_MAX    127
#define UINT8_MAX   255

#define INT16_MIN   (-32768)
#define INT16_MAX   32767
#define UINT16_MAX  65535

#define INT32_MIN   (-2147483647 - 1)
#define INT32_MAX   2147483647
#define UINT32_MAX  4294967295U

#define INT64_MIN   (-9223372036854775807L - 1L)
#define INT64_MAX   9223372036854775807L
#define UINT64_MAX  18446744073709551615UL

#define INTPTR_MIN  INT64_MIN
#define INTPTR_MAX  INT64_MAX
#define UINTPTR_MAX UINT64_MAX

#define INTMAX_MIN  INT64_MIN
#define INTMAX_MAX  INT64_MAX
#define UINTMAX_MAX UINT64_MAX

/* Minimum-width type limits */
#define INT_LEAST8_MIN    INT8_MIN
#define INT_LEAST8_MAX    INT8_MAX
#define UINT_LEAST8_MAX   UINT8_MAX
#define INT_LEAST16_MIN   INT16_MIN
#define INT_LEAST16_MAX   INT16_MAX
#define UINT_LEAST16_MAX  UINT16_MAX
#define INT_LEAST32_MIN   INT32_MIN
#define INT_LEAST32_MAX   INT32_MAX
#define UINT_LEAST32_MAX  UINT32_MAX
#define INT_LEAST64_MIN   INT64_MIN
#define INT_LEAST64_MAX   INT64_MAX
#define UINT_LEAST64_MAX  UINT64_MAX

/* Fastest minimum-width type limits
 * On x86_64 clang: int_fast{8,16,32}_t = int, int_fast64_t = long */
#define INT_FAST8_MIN     INT32_MIN
#define INT_FAST8_MAX     INT32_MAX
#define UINT_FAST8_MAX    UINT32_MAX
#define INT_FAST16_MIN    INT32_MIN
#define INT_FAST16_MAX    INT32_MAX
#define UINT_FAST16_MAX   UINT32_MAX
#define INT_FAST32_MIN    INT32_MIN
#define INT_FAST32_MAX    INT32_MAX
#define UINT_FAST32_MAX   UINT32_MAX
#define INT_FAST64_MIN    INT64_MIN
#define INT_FAST64_MAX    INT64_MAX
#define UINT_FAST64_MAX   UINT64_MAX

#define SIZE_MAX    UINT64_MAX
#define PTRDIFF_MIN INT64_MIN
#define PTRDIFF_MAX INT64_MAX

/* Integer constant macros (C99 7.18.4) */
#define INT8_C(x)    (x)
#define INT16_C(x)   (x)
#define INT32_C(x)   (x)
#define INT64_C(x)   (x##L)
#define UINT8_C(x)   (x)
#define UINT16_C(x)  (x)
#define UINT32_C(x)  (x##U)
#define UINT64_C(x)  (x##UL)
#define INTMAX_C(x)  (x##L)
#define UINTMAX_C(x) (x##UL)

#endif /* __STDINT_H__ */
