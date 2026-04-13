/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __PWD_H__
#define __PWD_H__

#include <sys/types.h>
#include <sys/cdefs.h>

#define _PATH_PASSWD        "/etc/passwd"
#define _PATH_MASTERPASSWD  "/etc/master.passwd"
#define _PATH_PWD           "/etc"
#define _PASSWD             "passwd"
#define _MASTERPASSWD       "master.passwd"
#define _PASSWORD_LEN       128

struct passwd {
    char        *pw_name;       /* user name */
    char        *pw_passwd;     /* encrypted password */
    uid_t        pw_uid;        /* user uid */
    gid_t        pw_gid;        /* user gid */
    time_t       pw_change;     /* password change time */
    char        *pw_class;      /* user access class */
    char        *pw_gecos;      /* Honeywell login info */
    char        *pw_dir;        /* home directory */
    char        *pw_shell;      /* default shell */
    time_t       pw_expire;     /* account expiration */
    int          pw_fields;     /* internal: fields filled in */
};

/* Mapping from fields to bits for pw_fields. */
#define _PWF(x)         (1 << (x))
#define _PWF_NAME       _PWF(0)
#define _PWF_PASSWD     _PWF(1)
#define _PWF_UID        _PWF(2)
#define _PWF_GID        _PWF(3)
#define _PWF_CHANGE     _PWF(4)
#define _PWF_CLASS      _PWF(5)
#define _PWF_GECOS      _PWF(6)
#define _PWF_DIR        _PWF(7)
#define _PWF_SHELL      _PWF(8)
#define _PWF_EXPIRE     _PWF(9)

#define _PWF_SOURCE     0x3000
#define _PWF_FILES      0x1000
#define _PWF_NIS        0x2000

__BEGIN_DECLS

extern struct passwd *getpwnam(const char *name);
extern struct passwd *getpwuid(uid_t uid);
extern struct passwd *getpwent(void);
extern void           setpwent(void);
extern void           endpwent(void);
extern int            getpwuid_r(uid_t uid, struct passwd *pwd,
                                 char *buf, size_t buflen,
                                 struct passwd **result);
extern int            getpwnam_r(const char *name, struct passwd *pwd,
                                 char *buf, size_t buflen,
                                 struct passwd **result);

__END_DECLS

#endif /* __PWD_H__ */
