/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __LINUX_CAPABILITY_H__
#define __LINUX_CAPABILITY_H__

/* Minimal Linux capability ABI shim for source compatibility.
   SaltyOS does not implement Linux POSIX capabilities — capget()/capset()
   syscalls return ENOSYS at runtime, and consumers should treat the result
   as "no elevated privileges". */

#include <stdint.h>

#define _LINUX_CAPABILITY_VERSION_1  0x19980330
#define _LINUX_CAPABILITY_VERSION_2  0x20071026
#define _LINUX_CAPABILITY_VERSION_3  0x20080522

#define _LINUX_CAPABILITY_U32S_1     1
#define _LINUX_CAPABILITY_U32S_2     2
#define _LINUX_CAPABILITY_U32S_3     2

typedef struct __user_cap_header_struct {
    uint32_t version;
    int      pid;
} *cap_user_header_t;

typedef struct __user_cap_data_struct {
    uint32_t effective;
    uint32_t permitted;
    uint32_t inheritable;
} *cap_user_data_t;

/* Capability ids (Linux <linux/capability.h>). Subset commonly read by
   process-introspection tools. */
#define CAP_CHOWN              0
#define CAP_DAC_OVERRIDE       1
#define CAP_DAC_READ_SEARCH    2
#define CAP_FOWNER             3
#define CAP_FSETID             4
#define CAP_KILL               5
#define CAP_SETGID             6
#define CAP_SETUID             7
#define CAP_SETPCAP            8
#define CAP_NET_BIND_SERVICE  10
#define CAP_NET_RAW           13
#define CAP_SYS_CHROOT        18
#define CAP_SYS_PTRACE        19
#define CAP_SYS_ADMIN         21
#define CAP_SYS_NICE          23
#define CAP_SYS_RESOURCE      24
#define CAP_SYS_TIME          25
#define CAP_AUDIT_WRITE       29
#define CAP_AUDIT_CONTROL     30
#define CAP_SETFCAP           31
#define CAP_LAST_CAP          CAP_SETFCAP

#endif /* __LINUX_CAPABILITY_H__ */
