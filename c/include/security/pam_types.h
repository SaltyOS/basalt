/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef SECURITY_PAM_TYPES_H_INCLUDED
#define SECURITY_PAM_TYPES_H_INCLUDED

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

struct pam_message {
    int msg_style;
    char *msg;
};

struct pam_response {
    char *resp;
    int resp_retcode;
};

struct pam_conv {
    int (*conv)(int, const struct pam_message **, struct pam_response **, void *);
    void *appdata_ptr;
};

struct pam_handle;
typedef struct pam_handle pam_handle_t;

typedef struct pam_repository {
    char *type;
    void *scope;
    size_t scope_len;
} pam_repository_t;

#ifdef __cplusplus
}
#endif

#endif
