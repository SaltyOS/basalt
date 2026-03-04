#ifndef _SYS_ENDIAN_H_
#define _SYS_ENDIAN_H_

#include <machine/endian.h>

#define bswap16(x)  __builtin_bswap16(x)
#define bswap32(x)  __builtin_bswap32(x)
#define bswap64(x)  __builtin_bswap64(x)

#if _BYTE_ORDER == _LITTLE_ENDIAN
#define htonl(x)    __builtin_bswap32(x)
#define htons(x)    __builtin_bswap16(x)
#define ntohl(x)    __builtin_bswap32(x)
#define ntohs(x)    __builtin_bswap16(x)
#else
#define htonl(x)    ((uint32_t)(x))
#define htons(x)    ((uint16_t)(x))
#define ntohl(x)    ((uint32_t)(x))
#define ntohs(x)    ((uint16_t)(x))
#endif

#endif /* _SYS_ENDIAN_H_ */
