/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __LOCALE_H__
#define __LOCALE_H__

#include <sys/cdefs.h>

#define LC_CTYPE    0
#define LC_NUMERIC  1
#define LC_TIME     2
#define LC_COLLATE  3
#define LC_MONETARY 4
#define LC_MESSAGES 5
#define LC_ALL      6

#define LC_COLLATE_MASK   (1 << LC_COLLATE)
#define LC_CTYPE_MASK     (1 << LC_CTYPE)
#define LC_MONETARY_MASK  (1 << LC_MONETARY)
#define LC_NUMERIC_MASK   (1 << LC_NUMERIC)
#define LC_TIME_MASK      (1 << LC_TIME)
#define LC_MESSAGES_MASK  (1 << LC_MESSAGES)
#define LC_ALL_MASK       0x7F

typedef void *locale_t;

struct lconv {
    char *decimal_point;
    char *thousands_sep;
    char *grouping;
    char *int_curr_symbol;
    char *currency_symbol;
    char *mon_decimal_point;
    char *mon_thousands_sep;
    char *mon_grouping;
    char *positive_sign;
    char *negative_sign;
    char  int_frac_digits;
    char  frac_digits;
    char  p_cs_precedes;
    char  p_sep_by_space;
    char  n_cs_precedes;
    char  n_sep_by_space;
    char  p_sign_posn;
    char  n_sign_posn;
    char  int_p_cs_precedes;
    char  int_p_sep_by_space;
    char  int_n_cs_precedes;
    char  int_n_sep_by_space;
    char  int_p_sign_posn;
    char  int_n_sign_posn;
};

__BEGIN_DECLS

extern char        *setlocale(int category, const char *locale);
extern struct lconv *localeconv(void);

extern locale_t newlocale(int mask, const char *locale, locale_t base);
extern void     freelocale(locale_t loc);
extern locale_t uselocale(locale_t loc);

/* gettext stubs — guarded so projects can #define them as macros */
#ifndef textdomain
extern char *textdomain(const char *domainname);
#endif
#ifndef bindtextdomain
extern char *bindtextdomain(const char *domainname, const char *dirname);
#endif
#ifndef gettext
extern char *gettext(const char *msgid);
#endif
#ifndef dgettext
extern char *dgettext(const char *domainname, const char *msgid);
#endif
#ifndef dcgettext
extern char *dcgettext(const char *domainname, const char *msgid, int category);
#endif
#ifndef ngettext
extern char *ngettext(const char *msgid1, const char *msgid2, unsigned long n);
#endif
#ifndef dngettext
extern char *dngettext(const char *domainname, const char *msgid1,
                       const char *msgid2, unsigned long n);
#endif

__END_DECLS

#endif /* __LOCALE_H__ */
