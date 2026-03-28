/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __STDIO_EXT_H__
#define __STDIO_EXT_H__

#include <stdio.h>
#include <sys/cdefs.h>

__BEGIN_DECLS

/* Return non-zero if the stream is opened read-only, or if the last
   operation on the stream was a read operation.  */
extern int __freading(FILE *__fp);

/* Return non-zero if the stream is opened write-only or append-only, or if
   the last operation on the stream was a write operation.  */
extern int __fwriting(FILE *__fp);

/* Discard the contents of the stream's buffer.  */
extern void __fpurge(FILE *__fp);

/* Return the size of the buffer of the stream.  */
extern size_t __fbufsize(FILE *__fp);

/* Return non-zero if the stream's buffer is line-buffered.  */
extern int __flbf(FILE *__fp);

/* Return the number of pending (buffered but not yet written) bytes.  */
extern size_t __fpending(FILE *__fp);

__END_DECLS

#endif /* __STDIO_EXT_H__ */
