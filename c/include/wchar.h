/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __WCHAR_H__
#define __WCHAR_H__

#include <stddef.h>
#include <stdarg.h>
#include <sys/cdefs.h>
#include <time.h>       /* struct tm — required by POSIX for wcsftime */

typedef int          wchar_t_saltyc;  /* actual wchar_t from stddef.h */
typedef unsigned int wint_t;
typedef unsigned int mbstate_t;

#define WEOF ((wint_t)0xFFFFFFFF)
#define WINT_MAX ((wint_t)0xFFFFFFFF)

__BEGIN_DECLS

/* Multibyte / wide conversions */
extern size_t mbrtowc(wchar_t *pwc, const char *s, size_t n, mbstate_t *ps);
extern size_t wcrtomb(char *s, wchar_t wc, mbstate_t *ps);
extern int    mblen(const char *s, size_t n);
extern int    mbtowc(wchar_t *pwc, const char *s, size_t n);
extern int    wctomb(char *s, wchar_t wc);
extern size_t mbrlen(const char *s, size_t n, mbstate_t *ps);

extern size_t mbsrtowcs(wchar_t *dst, const char **src, size_t len,
                        mbstate_t *ps);
extern size_t wcsrtombs(char *dst, const wchar_t **src, size_t len,
                        mbstate_t *ps);
extern size_t mbsnrtowcs(wchar_t *dst, const char **src, size_t nms,
                         size_t len, mbstate_t *ps);
extern size_t wcsnrtombs(char *dst, const wchar_t **src, size_t nwc,
                         size_t len, mbstate_t *ps);
extern int    mbsinit(const mbstate_t *ps);

/* Wide string operations */
extern size_t   wcslen(const wchar_t *ws);
extern int      wcscmp(const wchar_t *s1, const wchar_t *s2);
extern int      wcsncmp(const wchar_t *s1, const wchar_t *s2, size_t n);
extern wchar_t *wcscpy(wchar_t *dst, const wchar_t *src);
extern wchar_t *wcsncpy(wchar_t *dst, const wchar_t *src, size_t n);
extern wchar_t *wcschr(const wchar_t *ws, wchar_t wc);
extern wchar_t *wcsrchr(const wchar_t *ws, wchar_t wc);
extern wchar_t *wcscat(wchar_t *dst, const wchar_t *src);
extern wchar_t *wcsncat(wchar_t *dst, const wchar_t *src, size_t n);
extern wchar_t *wcsdup(const wchar_t *src);
extern wchar_t *wcspbrk(const wchar_t *ws, const wchar_t *charset);
extern wchar_t *wcsstr(const wchar_t *haystack, const wchar_t *needle);
extern int      wcscoll(const wchar_t *s1, const wchar_t *s2);
extern size_t   wcsxfrm(wchar_t *dst, const wchar_t *src, size_t n);

/* Wide memory operations */
extern wchar_t *wmemcpy(wchar_t *dst, const wchar_t *src, size_t n);
extern wchar_t *wmemset(wchar_t *dst, wchar_t wc, size_t n);
extern wchar_t *wmemchr(const wchar_t *ws, wchar_t wc, size_t n);
extern int      wmemcmp(const wchar_t *s1, const wchar_t *s2, size_t n);
extern wchar_t *wmemmove(wchar_t *dst, const wchar_t *src, size_t n);

/* Wide character width */
extern int wcwidth(wchar_t wc);

/* Locale / codeset */
extern char *nl_langinfo(int item);
extern size_t __ctype_get_mb_cur_max(void);

/* wint_t conversion */
extern wint_t btowc(int c);
extern int    wctob(wint_t c);

/* Wide character I/O */
#include <stdio.h>
extern wint_t fgetwc(FILE *stream);
extern wint_t fputwc(wint_t wc, FILE *stream);
extern wint_t getwc(FILE *stream);
extern wint_t getwchar(void);
extern wint_t putwchar(wint_t wc);
extern wint_t ungetwc(wint_t wc, FILE *stream);

/* Wide string to number conversions */
extern long               wcstol(const wchar_t *nptr, wchar_t **endptr, int base);
extern unsigned long      wcstoul(const wchar_t *nptr, wchar_t **endptr, int base);
extern long long          wcstoll(const wchar_t *nptr, wchar_t **endptr, int base);
extern unsigned long long wcstoull(const wchar_t *nptr, wchar_t **endptr, int base);
extern float              wcstof(const wchar_t *nptr, wchar_t **endptr);
extern double             wcstod(const wchar_t *nptr, wchar_t **endptr);
extern long double        wcstold(const wchar_t *nptr, wchar_t **endptr);

/* Wide formatted output */
extern int swprintf(wchar_t *s, size_t n, const wchar_t *fmt, ...);

__END_DECLS

#endif /* __WCHAR_H__ */
