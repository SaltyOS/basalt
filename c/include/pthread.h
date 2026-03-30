/* SPDX-License-Identifier: GPL-2.0-only */
/* POSIX threads — types and function prototypes for SaltyOS basaltc */
#ifndef __PTHREAD_H__
#define __PTHREAD_H__

#include <sys/types.h>
#include <time.h>
#include <signal.h>

#ifdef __cplusplus
extern "C" {
#endif

/* =========================================================================
 * Opaque handle types
 * Sizes and layouts MUST match the Rust structs in pthread_impl.rs.
 * ========================================================================= */

/** Thread handle: opaque pointer into the ThreadControl pool. */
typedef void *pthread_t;

/**
 * pthread_mutex_t — 32 bytes, 8-byte aligned.
 * First 24 bytes: mutex state (TypedMutex / Mutex).
 * Byte 24: kind (0=NORMAL, 1=RECURSIVE, 2=ERRORCHECK).
 *
 * Use a union with an explicit 64-bit member rather than relying on
 * `__attribute__((aligned))` on an opaque typedef. Some consumers embed these
 * opaque pthread types inside other structs, and AArch64 atomic instructions
 * require naturally aligned addresses.
 */
typedef union {
    unsigned char __opaque[32];
    unsigned long long __align;
} pthread_mutex_t;

/**
 * pthread_cond_t — 8 bytes, 8-byte aligned.
 * Wraps sync::Condvar (single AtomicU32, padded to 8 bytes).
 */
typedef union {
    unsigned char __opaque[8];
    unsigned long long __align;
} pthread_cond_t;

/**
 * pthread_rwlock_t — 16 bytes, 8-byte aligned.
 * Wraps sync::RWLock (three AtomicU32 = 12 bytes, padded to 16).
 */
typedef union {
    unsigned char __opaque[16];
    unsigned long long __align;
} pthread_rwlock_t;

/**
 * pthread_barrier_t — 16 bytes, 8-byte aligned.
 * Wraps sync::Barrier (u32 + 2×AtomicU32 = 12 bytes, padded to 16).
 */
typedef union {
    unsigned char __opaque[16];
    unsigned long long __align;
} pthread_barrier_t;

/**
 * pthread_once_t — 8 bytes, 8-byte aligned.
 * Wraps sync::Once (single AtomicU32, padded to 8 bytes).
 */
typedef union {
    unsigned char __opaque[8];
    unsigned long long __align;
} pthread_once_t;

/**
 * pthread_attr_t — 16 bytes.
 * stack_size: u64 (8 bytes) + detach_state: u32 (4 bytes) + pad: u32 (4 bytes).
 */
typedef struct {
    unsigned long long __stack_size;
    unsigned int       __detach_state;
    unsigned int       __pad;
} pthread_attr_t;

/** pthread_mutexattr_t — 4 bytes (kind: i32). */
typedef struct {
    int __kind;
} pthread_mutexattr_t;

/** pthread_condattr_t — 4 bytes (placeholder). */
typedef struct {
    unsigned int __unused;
} pthread_condattr_t;

/** pthread_rwlockattr_t — 4 bytes (placeholder). */
typedef struct {
    unsigned int __unused;
} pthread_rwlockattr_t;

/** pthread_barrierattr_t — 4 bytes (placeholder). */
typedef struct {
    unsigned int __unused;
} pthread_barrierattr_t;

/** pthread_key_t — thread-local storage key (unsigned int). */
typedef unsigned int pthread_key_t;

/* =========================================================================
 * Constants
 * ========================================================================= */

#define PTHREAD_MUTEX_NORMAL     0
#define PTHREAD_MUTEX_RECURSIVE  1
#define PTHREAD_MUTEX_ERRORCHECK 2
#define PTHREAD_MUTEX_DEFAULT    PTHREAD_MUTEX_NORMAL

#define PTHREAD_CREATE_JOINABLE  0
#define PTHREAD_CREATE_DETACHED  1

#define PTHREAD_CANCEL_ENABLE    0
#define PTHREAD_CANCEL_DISABLE   1

#define PTHREAD_CANCEL_DEFERRED    0
#define PTHREAD_CANCEL_ASYNCHRONOUS 1

#define PTHREAD_BARRIER_SERIAL_THREAD (-1)

#define PTHREAD_CANCELED ((void *)(unsigned long)-1)

/* Static initializers */
#define PTHREAD_MUTEX_INITIALIZER   { { 0 } }
#define PTHREAD_COND_INITIALIZER    { { 0 } }
#define PTHREAD_RWLOCK_INITIALIZER  { { 0 } }
#define PTHREAD_ONCE_INIT           { { 0 } }

/* =========================================================================
 * Thread lifecycle
 * ========================================================================= */

int  pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                    void *(*start_routine)(void *), void *arg);
int  pthread_join(pthread_t thread, void **retval);
void pthread_exit(void *retval) __attribute__((noreturn));
pthread_t pthread_self(void);
int  pthread_detach(pthread_t thread);
int  pthread_equal(pthread_t t1, pthread_t t2);
int  pthread_cancel(pthread_t thread);
int  pthread_setcancelstate(int state, int *oldstate);
int  pthread_setcanceltype(int type, int *oldtype);
void pthread_testcancel(void);

/* =========================================================================
 * Thread attributes
 * ========================================================================= */

int pthread_attr_init(pthread_attr_t *attr);
int pthread_attr_destroy(pthread_attr_t *attr);
int pthread_attr_setdetachstate(pthread_attr_t *attr, int detachstate);
int pthread_attr_getdetachstate(const pthread_attr_t *attr, int *detachstate);
int pthread_attr_setstacksize(pthread_attr_t *attr, size_t stacksize);
int pthread_attr_getstacksize(const pthread_attr_t *attr, size_t *stacksize);

/* =========================================================================
 * Mutexes
 * ========================================================================= */

int pthread_mutex_init(pthread_mutex_t *mutex, const pthread_mutexattr_t *attr);
int pthread_mutex_lock(pthread_mutex_t *mutex);
int pthread_mutex_trylock(pthread_mutex_t *mutex);
int pthread_mutex_unlock(pthread_mutex_t *mutex);
int pthread_mutex_timedlock(pthread_mutex_t *mutex,
                            const struct timespec *abstime);
int pthread_mutex_destroy(pthread_mutex_t *mutex);

int pthread_mutexattr_init(pthread_mutexattr_t *attr);
int pthread_mutexattr_destroy(pthread_mutexattr_t *attr);
int pthread_mutexattr_settype(pthread_mutexattr_t *attr, int type);
int pthread_mutexattr_gettype(const pthread_mutexattr_t *attr, int *type);

/* =========================================================================
 * Condition variables
 * ========================================================================= */

int pthread_cond_init(pthread_cond_t *cond, const pthread_condattr_t *attr);
int pthread_cond_wait(pthread_cond_t *cond, pthread_mutex_t *mutex);
int pthread_cond_timedwait(pthread_cond_t *cond, pthread_mutex_t *mutex,
                           const struct timespec *abstime);
int pthread_cond_signal(pthread_cond_t *cond);
int pthread_cond_broadcast(pthread_cond_t *cond);
int pthread_cond_destroy(pthread_cond_t *cond);

int pthread_condattr_init(pthread_condattr_t *attr);
int pthread_condattr_destroy(pthread_condattr_t *attr);

/* =========================================================================
 * Reader-writer locks
 * ========================================================================= */

int pthread_rwlock_init(pthread_rwlock_t *rwlock, const pthread_rwlockattr_t *attr);
int pthread_rwlock_rdlock(pthread_rwlock_t *rwlock);
int pthread_rwlock_wrlock(pthread_rwlock_t *rwlock);
int pthread_rwlock_tryrdlock(pthread_rwlock_t *rwlock);
int pthread_rwlock_trywrlock(pthread_rwlock_t *rwlock);
int pthread_rwlock_timedrdlock(pthread_rwlock_t *rwlock,
                               const struct timespec *abstime);
int pthread_rwlock_timedwrlock(pthread_rwlock_t *rwlock,
                               const struct timespec *abstime);
int pthread_rwlock_unlock(pthread_rwlock_t *rwlock);
int pthread_rwlock_destroy(pthread_rwlock_t *rwlock);

/* =========================================================================
 * Barriers
 * ========================================================================= */

int pthread_barrier_init(pthread_barrier_t *barrier,
                         const pthread_barrierattr_t *attr,
                         unsigned count);
int pthread_barrier_wait(pthread_barrier_t *barrier);
int pthread_barrier_destroy(pthread_barrier_t *barrier);

/* =========================================================================
 * Once initialization
 * ========================================================================= */

int pthread_once(pthread_once_t *once_control, void (*init_routine)(void));

/* =========================================================================
 * Thread-local storage
 * ========================================================================= */

int   pthread_key_create(pthread_key_t *key, void (*destructor)(void *));
int   pthread_key_delete(pthread_key_t key);
void *pthread_getspecific(pthread_key_t key);
int   pthread_setspecific(pthread_key_t key, const void *value);

/* =========================================================================
 * Signal mask
 * ========================================================================= */

int pthread_sigmask(int how, const sigset_t *set, sigset_t *oldset);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* __PTHREAD_H__ */
