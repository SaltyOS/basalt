/* SPDX-License-Identifier: GPL-2.0-only */
/* FreeBSD db(3) compatibility — stub for SaltyOS */
#ifndef __DB_H__
#define __DB_H__

#include <sys/types.h>
#include <sys/cdefs.h>

#define DB_BTREE  1
#define DB_HASH   2
#define DB_RECNO  3

#define R_CURSOR   1
#define R_FIRST    3
#define R_LAST     6
#define R_NEXT     7
#define R_NOOVERWRITE 8
#define R_PREV    10
#define R_SETCURSOR  11

#define RET_ERROR  -1
#define RET_SUCCESS 0
#define RET_SPECIAL 1

typedef struct {
    void  *data;
    size_t size;
} DBT;

typedef struct __db {
    int (*close)(struct __db *);
    int (*del)(const struct __db *, const DBT *, unsigned int);
    int (*get)(const struct __db *, const DBT *, DBT *, unsigned int);
    int (*put)(const struct __db *, DBT *, const DBT *, unsigned int);
    int (*seq)(const struct __db *, DBT *, DBT *, unsigned int);
    int (*sync)(const struct __db *, unsigned int);
    int type;
    void *internal;
    int (*fd)(const struct __db *);
} DB;

typedef struct {
    unsigned int bsize;
    unsigned int ffactor;
    unsigned int nelem;
    unsigned int cachesize;
    unsigned int (*hash)(const void *, size_t);
    int lorder;
} HASHINFO;

typedef struct {
    unsigned long flags;
    unsigned int cachesize;
    int maxkeypage;
    int minkeypage;
    unsigned int psize;
    int (*compare)(const DBT *, const DBT *);
    size_t (*prefix)(const DBT *, const DBT *);
    int lorder;
} BTREEINFO;

__BEGIN_DECLS
extern DB *dbopen(const char *file, int flags, int mode, int type, const void *openinfo);
__END_DECLS

#endif /* __DB_H__ */
