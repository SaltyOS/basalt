/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef SECURITY_PAM_MODULES_H_INCLUDED
#define SECURITY_PAM_MODULES_H_INCLUDED

#include <security/pam_types.h>
#include <security/pam_constants.h>
#include <security/openpam.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(PAM_SM_ACCOUNT)
PAM_EXTERN int pam_sm_acct_mgmt(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
#endif

#if defined(PAM_SM_AUTH)
PAM_EXTERN int pam_sm_authenticate(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
PAM_EXTERN int pam_sm_setcred(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
#endif

#if defined(PAM_SM_PASSWORD)
PAM_EXTERN int pam_sm_chauthtok(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
#endif

#if defined(PAM_SM_SESSION)
PAM_EXTERN int pam_sm_close_session(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
PAM_EXTERN int pam_sm_open_session(pam_handle_t *_pamh, int _flags, int _argc, const char **_argv);
#endif

#ifdef __cplusplus
}
#endif

#endif
