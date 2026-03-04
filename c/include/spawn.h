/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SPAWN_H__
#define __SPAWN_H__

#include <sys/types.h>
#include <signal.h>
#include <fcntl.h>
#include <sys/cdefs.h>

/*
 * posix_spawn file action types
 */
#define __SPAWN_ACTION_CLOSE  0
#define __SPAWN_ACTION_DUP2   1
#define __SPAWN_ACTION_OPEN   2

#define POSIX_SPAWN_RESETIDS        0x01
#define POSIX_SPAWN_SETPGROUP       0x02
#define POSIX_SPAWN_SETSIGDEF       0x04
#define POSIX_SPAWN_SETSIGMASK      0x08
#define POSIX_SPAWN_SETSCHEDPARAM   0x10
#define POSIX_SPAWN_SETSCHEDULER    0x20

#define __SPAWN_MAX_FILE_ACTIONS 16

typedef struct {
    int  type;
    int  fd;
    int  newfd;       /* for dup2 */
    const char *path; /* for open */
    int  oflag;       /* for open */
    mode_t mode;      /* for open */
} __spawn_file_action_t;

typedef struct {
    int count;
    __spawn_file_action_t actions[__SPAWN_MAX_FILE_ACTIONS];
} posix_spawn_file_actions_t;

typedef struct {
    short  flags;
    pid_t  pgroup;
    sigset_t sigdefault;
    sigset_t sigmask;
} posix_spawnattr_t;

__BEGIN_DECLS

extern int posix_spawn(pid_t *pid, const char *path,
                       const posix_spawn_file_actions_t *file_actions,
                       const posix_spawnattr_t *attrp,
                       char *const argv[], char *const envp[]);

extern int posix_spawnp(pid_t *pid, const char *file,
                        const posix_spawn_file_actions_t *file_actions,
                        const posix_spawnattr_t *attrp,
                        char *const argv[], char *const envp[]);

/* Spawn attributes */
extern int posix_spawnattr_init(posix_spawnattr_t *attrp);
extern int posix_spawnattr_destroy(posix_spawnattr_t *attrp);
extern int posix_spawnattr_setflags(posix_spawnattr_t *attrp, short flags);
extern int posix_spawnattr_getflags(const posix_spawnattr_t *attrp, short *flags);
extern int posix_spawnattr_setsigmask(posix_spawnattr_t *attrp, const sigset_t *sigmask);
extern int posix_spawnattr_getsigmask(const posix_spawnattr_t *attrp, sigset_t *sigmask);
extern int posix_spawnattr_setsigdefault(posix_spawnattr_t *attrp, const sigset_t *sigdefault);
extern int posix_spawnattr_getsigdefault(const posix_spawnattr_t *attrp, sigset_t *sigdefault);
extern int posix_spawnattr_setpgroup(posix_spawnattr_t *attrp, pid_t pgroup);
extern int posix_spawnattr_getpgroup(const posix_spawnattr_t *attrp, pid_t *pgroup);

/* File actions */
extern int posix_spawn_file_actions_init(posix_spawn_file_actions_t *fact);
extern int posix_spawn_file_actions_destroy(posix_spawn_file_actions_t *fact);
extern int posix_spawn_file_actions_addclose(posix_spawn_file_actions_t *fact, int fd);
extern int posix_spawn_file_actions_adddup2(posix_spawn_file_actions_t *fact, int fd, int newfd);
extern int posix_spawn_file_actions_addopen(posix_spawn_file_actions_t *fact, int fd,
                                            const char *path, int oflag, mode_t mode);

__END_DECLS

#endif /* __SPAWN_H__ */
