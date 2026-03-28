/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __AR_H__
#define __AR_H__

#define ARMAG   "!<arch>\n"     /* ar magic string */
#define SARMAG  8               /* strlen(ARMAG) */
#define ARFMAG  "`\n"           /* header trailer string */

struct ar_hdr {
    char ar_name[16];   /* member name */
    char ar_date[12];   /* file date, decimal seconds since epoch */
    char ar_uid[6];     /* user id, decimal */
    char ar_gid[6];     /* group id, decimal */
    char ar_mode[8];    /* file mode, octal */
    char ar_size[10];   /* file size, decimal */
    char ar_fmag[2];    /* always ARFMAG */
};

#endif /* __AR_H__ */
