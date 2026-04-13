/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef _CRYPT_H
#define _CRYPT_H

#include <sys/cdefs.h>

struct crypt_data {
    char output[256];
    char internal[256];
};

__BEGIN_DECLS

extern char *crypt(const char *key, const char *salt);
extern char *crypt_r(const char *key, const char *salt, struct crypt_data *data);

__END_DECLS

#endif /* _CRYPT_H */
