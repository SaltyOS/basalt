/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef SECURITY_PAM_APPL_H_INCLUDED
#define SECURITY_PAM_APPL_H_INCLUDED

#include <security/pam_types.h>
#include <security/pam_constants.h>
#include <security/openpam_attr.h>

#ifdef __cplusplus
extern "C" {
#endif

int pam_acct_mgmt(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_authenticate(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_chauthtok(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_close_session(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_end(pam_handle_t *_pamh, int _status);
int pam_get_data(const pam_handle_t *_pamh, const char *_module_data_name, const void **_data)
    OPENPAM_NONNULL((1, 2, 3));
int pam_get_item(const pam_handle_t *_pamh, int _item_type, const void **_item)
    OPENPAM_NONNULL((1, 3));
int pam_get_user(pam_handle_t *_pamh, const char **_user, const char *_prompt)
    OPENPAM_NONNULL((1, 2));
const char *pam_getenv(pam_handle_t *_pamh, const char *_name) OPENPAM_NONNULL((1, 2));
char **pam_getenvlist(pam_handle_t *_pamh) OPENPAM_NONNULL((1));
int pam_open_session(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_putenv(pam_handle_t *_pamh, const char *_namevalue) OPENPAM_NONNULL((1, 2));
int pam_set_data(
    pam_handle_t *_pamh,
    const char *_module_data_name,
    void *_data,
    void (*_cleanup)(pam_handle_t *_pamh, void *_data, int _pam_end_status)
) OPENPAM_NONNULL((1, 2));
int pam_set_item(pam_handle_t *_pamh, int _item_type, const void *_item) OPENPAM_NONNULL((1));
int pam_setcred(pam_handle_t *_pamh, int _flags) OPENPAM_NONNULL((1));
int pam_start(
    const char *_service,
    const char *_user,
    const struct pam_conv *_pam_conv,
    pam_handle_t **_pamh
) OPENPAM_NONNULL((4));
const char *pam_strerror(const pam_handle_t *_pamh, int _error_number);

#ifdef __cplusplus
}
#endif

#endif
