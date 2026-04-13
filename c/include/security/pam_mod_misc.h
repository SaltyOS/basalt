/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef PAM_MOD_MISC_H
#define PAM_MOD_MISC_H

#include <sys/cdefs.h>

#define PAM_OPT_NULLOK "nullok"
#define PAM_OPT_EMPTYOK "emptyok"
#define PAM_OPT_AUTH_AS_SELF "auth_as_self"
#define PAM_OPT_ECHO_PASS "echo_pass"
#define PAM_OPT_DEBUG "debug"

#define PAM_LOG(...) openpam_log(PAM_LOG_DEBUG, __VA_ARGS__)
#define PAM_RETURN(arg) return (arg)
#define PAM_VERBOSE_ERROR(...)                                    \
    do {                                                          \
        if (!(flags & PAM_SILENT) && !openpam_get_option(pamh, "no_warn")) \
            pam_error(pamh, __VA_ARGS__);                         \
    } while (0)

#endif
