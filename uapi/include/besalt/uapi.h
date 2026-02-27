/* besalt-uapi public header
 * SPDX-License-Identifier: GPL-2.0-only
 */

#ifndef BESALT_UAPI_H
#define BESALT_UAPI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define BESALT_UAPI_VERSION 1u

typedef uint64_t besalt_cap_t;

typedef struct besalt_sysret {
    uint64_t value;
    uint64_t error;
} besalt_sysret_t;

enum besalt_syscall {
    BESALT_SYS_SEND = 0,
    BESALT_SYS_RECV = 1,
    BESALT_SYS_CALL = 2,
    BESALT_SYS_REPLY_RECV = 3,
    BESALT_SYS_NBSEND = 4,
    BESALT_SYS_SIGNAL = 5,
    BESALT_SYS_WAIT = 6,
    BESALT_SYS_POLL = 7,
    BESALT_SYS_YIELD = 8,
    BESALT_SYS_INVOKE = 9,
    BESALT_SYS_FUTEX = 18,
    BESALT_SYS_GETRANDOM = 19,
};

enum besalt_error {
    BESALT_OK = 0,
    BESALT_INVALID_CAPABILITY = 1,
    BESALT_INVALID_OPERATION = 2,
    BESALT_INSUFFICIENT_RIGHTS = 3,
    BESALT_INVALID_ARGUMENT = 4,
    BESALT_OUT_OF_MEMORY = 5,
    BESALT_NOT_FOUND = 6,
    BESALT_BUSY = 7,
    BESALT_ALREADY_EXISTS = 8,
    BESALT_WOULD_BLOCK = 9,
};

#ifdef __cplusplus
}
#endif

#endif /* BESALT_UAPI_H */
