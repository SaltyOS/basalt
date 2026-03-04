/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_AUXV_H__
#define __SYS_AUXV_H__

#include <sys/cdefs.h>

/* ELF auxiliary vector types */
#define AT_NULL      0
#define AT_IGNORE    1
#define AT_EXECFD    2
#define AT_PHDR      3
#define AT_PHENT     4
#define AT_PHNUM     5
#define AT_PAGESZ    6
#define AT_BASE      7
#define AT_FLAGS     8
#define AT_ENTRY     9
#define AT_NOTELF    10
#define AT_UID       11
#define AT_EUID      12
#define AT_GID       13
#define AT_EGID      14
#define AT_PLATFORM  15
#define AT_HWCAP     16
#define AT_CLKTCK    17
#define AT_SECURE    23
#define AT_BASE_PLATFORM 24
#define AT_RANDOM    25
#define AT_HWCAP2    26
#define AT_EXECFN    31

/* FreeBSD-specific: executable path auxiliary type */
#define AT_EXECPATH  15

__BEGIN_DECLS

/* Return the value of auxiliary vector entry of the given type. */
extern unsigned long getauxval(unsigned long __type);

/*
 * FreeBSD-specific: retrieve an ELF auxiliary vector value into a buffer.
 * Returns 0 on success, -1 on failure.
 */
extern int elf_aux_info(int aux, void *buf, int buflen);

__END_DECLS

#endif /* __SYS_AUXV_H__ */
