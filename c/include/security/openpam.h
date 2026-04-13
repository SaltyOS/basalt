/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef SECURITY_OPENPAM_H_INCLUDED
#define SECURITY_OPENPAM_H_INCLUDED

#include <stdarg.h>

#include <security/pam_appl.h>
#include <security/openpam_attr.h>

#ifdef __cplusplus
extern "C" {
#endif

struct passwd;

int openpam_borrow_cred(pam_handle_t *_pamh, const struct passwd *_pwd) OPENPAM_NONNULL((1, 2));
int openpam_subst(const pam_handle_t *_pamh, char *_buf, size_t *_bufsize, const char *_template);
void openpam_free_data(pam_handle_t *_pamh, void *_data, int _status);
void openpam_free_envlist(char **_envlist);
const char *openpam_get_option(pam_handle_t *_pamh, const char *_option);
int openpam_restore_cred(pam_handle_t *_pamh) OPENPAM_NONNULL((1));
int openpam_set_option(pam_handle_t *_pamh, const char *_option, const char *_value);
int pam_error(const pam_handle_t *_pamh, const char *_fmt, ...)
    OPENPAM_FORMAT((__printf__, 2, 3)) OPENPAM_NONNULL((1, 2));
int pam_get_authtok(pam_handle_t *_pamh, int _item, const char **_authtok, const char *_prompt)
    OPENPAM_NONNULL((1, 3));
int pam_info(const pam_handle_t *_pamh, const char *_fmt, ...)
    OPENPAM_FORMAT((__printf__, 2, 3)) OPENPAM_NONNULL((1, 2));
int pam_prompt(const pam_handle_t *_pamh, int _style, char **_resp, const char *_fmt, ...)
    OPENPAM_FORMAT((__printf__, 4, 5)) OPENPAM_NONNULL((1, 4));
int pam_setenv(pam_handle_t *_pamh, const char *_name, const char *_value, int _overwrite)
    OPENPAM_NONNULL((1, 2, 3));
int pam_vinfo(const pam_handle_t *_pamh, const char *_fmt, va_list _ap)
    OPENPAM_FORMAT((__printf__, 2, 0)) OPENPAM_NONNULL((1, 2));
int pam_verror(const pam_handle_t *_pamh, const char *_fmt, va_list _ap)
    OPENPAM_FORMAT((__printf__, 2, 0)) OPENPAM_NONNULL((1, 2));
int pam_vprompt(const pam_handle_t *_pamh, int _style, char **_resp, const char *_fmt, va_list _ap)
    OPENPAM_FORMAT((__printf__, 4, 0)) OPENPAM_NONNULL((1, 4));
int openpam_ttyconv(
    int _num_msg,
    const struct pam_message **_msg,
    struct pam_response **_resp,
    void *_appdata_ptr
) OPENPAM_NONNULL((2, 3));

enum {
    OPENPAM_RESTRICT_SERVICE_NAME,
    OPENPAM_VERIFY_POLICY_FILE,
    OPENPAM_RESTRICT_MODULE_NAME,
    OPENPAM_VERIFY_MODULE_FILE,
    OPENPAM_FALLBACK_TO_OTHER,
    OPENPAM_NUM_FEATURES
};

int openpam_set_feature(int _feature, int _onoff);
int openpam_get_feature(int _feature, int *_onoff);

enum {
    PAM_LOG_LIBDEBUG = -1,
    PAM_LOG_DEBUG,
    PAM_LOG_VERBOSE,
    PAM_LOG_NOTICE,
    PAM_LOG_ERROR
};

void _openpam_log(int _level, const char *_func, const char *_fmt, ...)
    OPENPAM_FORMAT((__printf__, 3, 4)) OPENPAM_NONNULL((3));

#if defined(__STDC_VERSION__) && (__STDC_VERSION__ >= 199901L)
#define openpam_log(lvl, ...) _openpam_log((lvl), __func__, __VA_ARGS__)
#elif defined(__GNUC__) && (__GNUC__ >= 3)
#define openpam_log(lvl, ...) _openpam_log((lvl), __func__, __VA_ARGS__)
#elif defined(__GNUC__) && (__GNUC__ >= 2) && (__GNUC_MINOR__ >= 95)
#define openpam_log(lvl, fmt...) _openpam_log((lvl), __func__, ##fmt)
#elif defined(__GNUC__) && defined(__FUNCTION__)
#define openpam_log(lvl, fmt...) _openpam_log((lvl), __FUNCTION__, ##fmt)
#else
void openpam_log(int _level, const char *_format, ...)
    OPENPAM_FORMAT((__printf__, 2, 3)) OPENPAM_NONNULL((2));
#endif

#ifdef __cplusplus
}
#endif

#endif
