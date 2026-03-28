/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __LANGINFO_H__
#define __LANGINFO_H__

#include <sys/cdefs.h>

typedef int nl_item;

#define D_T_FMT     0
#define D_FMT       1
#define T_FMT       2
#define T_FMT_AMPM  3
#define AM_STR      4
#define PM_STR      5
#define DAY_1       6
#define DAY_2       7
#define DAY_3       8
#define DAY_4       9
#define DAY_5       10
#define DAY_6       11
#define DAY_7       12
#define ABDAY_1     13
#define ABDAY_2     14
#define ABDAY_3     15
#define ABDAY_4     16
#define ABDAY_5     17
#define ABDAY_6     18
#define ABDAY_7     19
#define MON_1       20
#define MON_2       21
#define MON_3       22
#define MON_4       23
#define MON_5       24
#define MON_6       25
#define MON_7       26
#define MON_8       27
#define MON_9       28
#define MON_10      29
#define MON_11      30
#define MON_12      31
#define ABMON_1     44
#define ABMON_2     45
#define ABMON_3     46
#define ABMON_4     47
#define ABMON_5     48
#define ABMON_6     49
#define ABMON_7     50
#define ABMON_8     51
#define ABMON_9     52
#define ABMON_10    53
#define ABMON_11    54
#define ABMON_12    55
#define RADIXCHAR   56
#define THOUSEP     57
#define YESEXPR     58
#define NOEXPR      59
#define CRNCYSTR    60
#define ERA         61
#define D_MD_ORDER  62
#define CODESET     63

__BEGIN_DECLS

extern char *nl_langinfo(nl_item item);

__END_DECLS

#endif /* __LANGINFO_H__ */
