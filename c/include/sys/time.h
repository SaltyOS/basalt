/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_TIME_H__
#define __SYS_TIME_H__

#include <sys/types.h>
#include <sys/cdefs.h>

#ifndef __TIMEVAL_DEFINED__
#define __TIMEVAL_DEFINED__
struct timeval {
    long tv_sec;
    long tv_usec;
};
#endif

struct timezone {
    int tz_minuteswest;
    int tz_dsttime;
};

__BEGIN_DECLS

extern int gettimeofday(struct timeval *tv, void *tz);
extern int utimes(const char *filename, const struct timeval times[2]);

#define ITIMER_REAL    0
#define ITIMER_VIRTUAL 1
#define ITIMER_PROF    2

struct itimerval {
    struct timeval it_interval;
    struct timeval it_value;
};

extern int setitimer(int which, const struct itimerval *new_value,
                     struct itimerval *old_value);
extern int getitimer(int which, struct itimerval *curr_value);

/* BSD/glibc timeval arithmetic macros. */
#define timerisset(tvp) ((tvp)->tv_sec || (tvp)->tv_usec)

#define timerclear(tvp) ((tvp)->tv_sec = (tvp)->tv_usec = 0)

#define timercmp(a, b, CMP) \
    (((a)->tv_sec == (b)->tv_sec) \
        ? ((a)->tv_usec CMP (b)->tv_usec) \
        : ((a)->tv_sec CMP (b)->tv_sec))

#define timeradd(a, b, result) \
    do { \
        (result)->tv_sec  = (a)->tv_sec  + (b)->tv_sec; \
        (result)->tv_usec = (a)->tv_usec + (b)->tv_usec; \
        if ((result)->tv_usec >= 1000000) { \
            ++(result)->tv_sec; \
            (result)->tv_usec -= 1000000; \
        } \
    } while (0)

#define timersub(a, b, result) \
    do { \
        (result)->tv_sec  = (a)->tv_sec  - (b)->tv_sec; \
        (result)->tv_usec = (a)->tv_usec - (b)->tv_usec; \
        if ((result)->tv_usec < 0) { \
            --(result)->tv_sec; \
            (result)->tv_usec += 1000000; \
        } \
    } while (0)

__END_DECLS

#endif /* __SYS_TIME_H__ */
