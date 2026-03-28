/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * xlocale.h — locale-aware (_l) function stubs for SaltyOS
 *
 * SaltyOS is C-locale-only. All _l variants ignore the locale_t argument
 * and delegate to the corresponding base function. This header satisfies
 * libc++'s __locale_dir/support/bsd_like.h which unconditionally includes
 * <xlocale.h> when __FreeBSD__ is defined.
 */
#ifndef __XLOCALE_H__
#define __XLOCALE_H__

#include <locale.h>
/* NOTE: stdlib.h is NOT included here — xlocale.h is included at the end of
   stdlib.h, so all stdlib declarations are already visible at that point.
   Other headers that xlocale.h needs are included explicitly. */
#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <ctype.h>
#include <wctype.h>
#include <wchar.h>
#include <time.h>

/* MB_CUR_MAX_L: always 4 (UTF-8), locale parameter ignored */
#define MB_CUR_MAX_L(loc) ((size_t)MB_CUR_MAX)

/* ------------------------------------------------------------------ */
/* <stdlib.h> conversions                                               */
/* ------------------------------------------------------------------ */

static inline float
strtof_l(const char *nptr, char **endptr, locale_t loc)
{
    (void)loc;
    return strtof(nptr, endptr);
}

static inline double
strtod_l(const char *nptr, char **endptr, locale_t loc)
{
    (void)loc;
    return strtod(nptr, endptr);
}

static inline long double
strtold_l(const char *nptr, char **endptr, locale_t loc)
{
    (void)loc;
    return strtold(nptr, endptr);
}

/* ------------------------------------------------------------------ */
/* <stdio.h> formatted output                                           */
/* ------------------------------------------------------------------ */

static inline int
snprintf_l(char *str, size_t size, locale_t loc, const char *fmt, ...)
{
    va_list ap;
    int ret;
    (void)loc;
    va_start(ap, fmt);
    ret = vsnprintf(str, size, fmt, ap);
    va_end(ap);
    return ret;
}

static inline int
asprintf_l(char **strp, locale_t loc, const char *fmt, ...)
{
    va_list ap;
    int ret;
    (void)loc;
    va_start(ap, fmt);
    ret = vasprintf(strp, fmt, ap);
    va_end(ap);
    return ret;
}

/* ------------------------------------------------------------------ */
/* <locale.h>                                                           */
/* ------------------------------------------------------------------ */

static inline struct lconv *
localeconv_l(locale_t loc)
{
    (void)loc;
    return localeconv();
}

/* ------------------------------------------------------------------ */
/* <ctype.h>                                                            */
/* ------------------------------------------------------------------ */

static inline int
toupper_l(int c, locale_t loc)
{
    (void)loc;
    return toupper(c);
}

static inline int
tolower_l(int c, locale_t loc)
{
    (void)loc;
    return tolower(c);
}

/* ------------------------------------------------------------------ */
/* <string.h>                                                           */
/* ------------------------------------------------------------------ */

static inline int
strcoll_l(const char *s1, const char *s2, locale_t loc)
{
    (void)loc;
    return strcoll(s1, s2);
}

static inline size_t
strxfrm_l(char *dest, const char *src, size_t n, locale_t loc)
{
    (void)loc;
    return strxfrm(dest, src, n);
}

/* ------------------------------------------------------------------ */
/* <time.h>                                                             */
/* ------------------------------------------------------------------ */

static inline size_t
strftime_l(char *s, size_t max, const char *format,
           const struct tm *tm, locale_t loc)
{
    (void)loc;
    return strftime(s, max, format, tm);
}

/* ------------------------------------------------------------------ */
/* <wctype.h> — isw*_l classification                                  */
/* ------------------------------------------------------------------ */

static inline int
iswctype_l(wint_t wc, wctype_t desc, locale_t loc)
{
    (void)loc;
    return iswctype(wc, desc);
}

static inline int
iswspace_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswspace(wc);
}

static inline int
iswprint_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswprint(wc);
}

static inline int
iswcntrl_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswcntrl(wc);
}

static inline int
iswupper_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswupper(wc);
}

static inline int
iswlower_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswlower(wc);
}

static inline int
iswalpha_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswalpha(wc);
}

static inline int
iswblank_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswblank(wc);
}

static inline int
iswdigit_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswdigit(wc);
}

static inline int
iswpunct_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswpunct(wc);
}

static inline int
iswxdigit_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return iswxdigit(wc);
}

/* ------------------------------------------------------------------ */
/* <wctype.h> — towupper_l / towlower_l                                */
/* ------------------------------------------------------------------ */

static inline wint_t
towupper_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return towupper(wc);
}

static inline wint_t
towlower_l(wint_t wc, locale_t loc)
{
    (void)loc;
    return towlower(wc);
}

/* ------------------------------------------------------------------ */
/* <wchar.h> — wide string collation / transformation                  */
/* ------------------------------------------------------------------ */

static inline int
wcscoll_l(const wchar_t *s1, const wchar_t *s2, locale_t loc)
{
    (void)loc;
    return wcscoll(s1, s2);
}

static inline size_t
wcsxfrm_l(wchar_t *dest, const wchar_t *src, size_t n, locale_t loc)
{
    (void)loc;
    return wcsxfrm(dest, src, n);
}

/* ------------------------------------------------------------------ */
/* <wchar.h> — wide/byte conversion                                    */
/* ------------------------------------------------------------------ */

static inline wint_t
btowc_l(int c, locale_t loc)
{
    (void)loc;
    return btowc(c);
}

static inline int
wctob_l(wint_t c, locale_t loc)
{
    (void)loc;
    return wctob(c);
}

static inline size_t
mbrtowc_l(wchar_t *pwc, const char *s, size_t n, mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return mbrtowc(pwc, s, n, ps);
}

static inline size_t
wcrtomb_l(char *s, wchar_t wc, mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return wcrtomb(s, wc, ps);
}

static inline size_t
mbrlen_l(const char *s, size_t n, mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return mbrlen(s, n, ps);
}

static inline int
mbtowc_l(wchar_t *pwc, const char *s, size_t n, locale_t loc)
{
    (void)loc;
    return mbtowc(pwc, s, n);
}

static inline size_t
mbsrtowcs_l(wchar_t *dst, const char **src, size_t len,
            mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return mbsrtowcs(dst, src, len, ps);
}

static inline size_t
mbsnrtowcs_l(wchar_t *dst, const char **src, size_t nms,
             size_t len, mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return mbsnrtowcs(dst, src, nms, len, ps);
}

static inline size_t
wcsnrtombs_l(char *dst, const wchar_t **src, size_t nwc,
             size_t len, mbstate_t *ps, locale_t loc)
{
    (void)loc;
    return wcsnrtombs(dst, src, nwc, len, ps);
}

#endif /* __XLOCALE_H__ */
