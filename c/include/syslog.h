/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYSLOG_H__
#define __SYSLOG_H__

#include <sys/cdefs.h>
#include <stdarg.h>

/* Priority levels */
#define LOG_EMERG   0
#define LOG_ALERT   1
#define LOG_CRIT    2
#define LOG_ERR     3
#define LOG_WARNING 4
#define LOG_NOTICE  5
#define LOG_INFO    6
#define LOG_DEBUG   7

/* Facility codes */
#define LOG_KERN     (0 << 3)
#define LOG_USER     (1 << 3)
#define LOG_MAIL     (2 << 3)
#define LOG_DAEMON   (3 << 3)
#define LOG_AUTH     (4 << 3)
#define LOG_SYSLOG   (5 << 3)
#define LOG_LPR      (6 << 3)
#define LOG_NEWS     (7 << 3)
#define LOG_UUCP     (8 << 3)
#define LOG_CRON     (9 << 3)
#define LOG_LOCAL0  (16 << 3)
#define LOG_LOCAL1  (17 << 3)
#define LOG_LOCAL2  (18 << 3)
#define LOG_LOCAL3  (19 << 3)
#define LOG_LOCAL4  (20 << 3)
#define LOG_LOCAL5  (21 << 3)
#define LOG_LOCAL6  (22 << 3)
#define LOG_LOCAL7  (23 << 3)

/* openlog() option flags */
#define LOG_PID    0x01
#define LOG_CONS   0x02
#define LOG_NDELAY 0x08
#define LOG_NOWAIT 0x10
#define LOG_PERROR 0x20

/* Extract priority and facility */
#define LOG_PRI(p)     ((p) & 0x07)
#define LOG_FAC(p)     (((p) >> 3) & 0x3f)
#define LOG_MAKEPRI(f,p) ((f) | (p))
#define LOG_MASK(pri)  (1 << (pri))
#define LOG_UPTO(pri)  ((1 << ((pri) + 1)) - 1)

__BEGIN_DECLS

extern void openlog(const char *ident, int option, int facility);
extern void syslog(int priority, const char *format, ...);
extern void vsyslog(int priority, const char *format, va_list ap);
extern void closelog(void);
extern int  setlogmask(int mask);

__END_DECLS

#endif /* __SYSLOG_H__ */
