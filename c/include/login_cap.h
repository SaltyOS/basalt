/* SPDX-License-Identifier: GPL-2.0-only */
/* FreeBSD login class capability — implemented in basaltc compat/freebsd */
#ifndef __LOGIN_CAP_H__
#define __LOGIN_CAP_H__

#include <sys/types.h>
#include <pwd.h>

#define LOGIN_DEFCLASS      "default"
#define LOGIN_DEFROOTCLASS  "root"
#define LOGIN_DEFSTYLE      "passwd"
#define LOGIN_DEFSERVICE    "login"
#define _PATH_LOGIN_CONF    "/etc/login.conf"
#define _FILE_LOGIN_CONF    ".login_conf"

/* LOGIN_SET* flags for setusercontext() — values must match FreeBSD libutil */
#define LOGIN_SETGROUP      0x0001  /* set group */
#define LOGIN_SETLOGIN      0x0002  /* set login (via setlogin) */
#define LOGIN_SETPATH       0x0004  /* set path */
#define LOGIN_SETPRIORITY   0x0008  /* set priority */
#define LOGIN_SETRESOURCES  0x0010  /* set resources (cputime, etc.) */
#define LOGIN_SETUMASK      0x0020  /* set umask */
#define LOGIN_SETUSER       0x0040  /* set user (via setuid) */
#define LOGIN_SETENV        0x0080  /* set user environment */
#define LOGIN_SETMAC        0x0100  /* set user default MAC label */
#define LOGIN_SETCPUMASK    0x0200  /* set user cpumask */
#define LOGIN_SETLOGINCLASS 0x0400  /* set login class in the kernel */
#define LOGIN_SETALL        0x07FF  /* set everything */

typedef struct login_cap {
    char *lc_class;
    char *lc_cap;
    char *lc_style;
} login_cap_t;

__BEGIN_DECLS

extern login_cap_t *login_getclass(const char *cls);
extern login_cap_t *login_getclassbyname(const char *cls, const struct passwd *pwd);
extern login_cap_t *login_getpwclass(const struct passwd *pwd);
extern login_cap_t *login_getuserclass(const struct passwd *pwd);
extern void login_close(login_cap_t *lc);
extern const char *login_getcapstr(login_cap_t *lc, const char *cap,
                                   const char *def, const char *err);
extern int login_getcapbool(login_cap_t *lc, const char *cap, int def);
extern long long login_getcaptime(login_cap_t *lc, const char *cap,
                                  long long def, long long err);
extern long long login_getcapnum(login_cap_t *lc, const char *cap,
                                 long long def, long long err);
extern long long login_getcapsize(login_cap_t *lc, const char *cap,
                                  long long def, long long err);
extern const char *login_getpath(login_cap_t *lc, const char *cap,
                                 const char *def);
extern const char *login_setcryptfmt(login_cap_t *lc, const char *def,
                                     const char *err);
extern int auth_ttyok(login_cap_t *lc, const char *tty);
extern int auth_hostok(login_cap_t *lc, const char *host);
extern int auth_timeok(login_cap_t *lc, long long now);
extern void setclassenvironment(login_cap_t *lc, const struct passwd *pwd, int flags);
extern void setclassresources(login_cap_t *lc);
extern int setusercontext(login_cap_t *lc, const struct passwd *pwd,
                          uid_t uid, unsigned int flags);

__END_DECLS

#endif /* __LOGIN_CAP_H__ */
