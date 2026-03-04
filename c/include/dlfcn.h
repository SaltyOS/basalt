/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __DLFCN_H__
#define __DLFCN_H__

#include <sys/cdefs.h>

#define RTLD_LAZY     0x0001
#define RTLD_NOW      0x0002
#define RTLD_GLOBAL   0x0100
#define RTLD_LOCAL    0x0000
#define RTLD_NOLOAD   0x0004
#define RTLD_NODELETE 0x1000

#define RTLD_DEFAULT  ((void *)0)
#define RTLD_NEXT     ((void *)-1L)

__BEGIN_DECLS

extern void *dlopen(const char *filename, int flags);
extern void *dlsym(void *handle, const char *symbol);
extern int   dlclose(void *handle);
extern char *dlerror(void);

/* dl_iterate_phdr callback info */
struct dl_phdr_info {
    unsigned long        dlpi_addr;
    const char          *dlpi_name;
    const void          *dlpi_phdr;
    unsigned short       dlpi_phnum;
};

extern int dl_iterate_phdr(
    int (*callback)(struct dl_phdr_info *info, unsigned long size, void *data),
    void *data);

/* Dynamic linker address lookup info */
typedef struct {
    const char *dli_fname;  /* pathname of shared object */
    void       *dli_fbase;  /* base address of shared object */
    const char *dli_sname;  /* name of nearest symbol */
    void       *dli_saddr;  /* exact address of named symbol */
} Dl_info;

extern int dladdr(const void *addr, Dl_info *info);

__END_DECLS

#endif /* __DLFCN_H__ */
