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

#define htobe16(x)  __builtin_bswap16(x)
#define htobe32(x)  __builtin_bswap32(x)
#define htobe64(x)  __builtin_bswap64(x)
#define be16toh(x)  __builtin_bswap16(x)
#define be32toh(x)  __builtin_bswap32(x)
#define be64toh(x)  __builtin_bswap64(x)

#define htole16(x)  ((uint16_t)(x))
#define htole32(x)  ((uint32_t)(x))
#define htole64(x)  ((uint64_t)(x))
#define le16toh(x)  ((uint16_t)(x))
#define le32toh(x)  ((uint32_t)(x))
#define le64toh(x)  ((uint64_t)(x))

#else /* _BIG_ENDIAN */

#define htonl(x)    ((uint32_t)(x))
#define htons(x)    ((uint16_t)(x))
#define ntohl(x)    ((uint32_t)(x))
#define ntohs(x)    ((uint16_t)(x))

#define htobe16(x)  ((uint16_t)(x))
#define htobe32(x)  ((uint32_t)(x))
#define htobe64(x)  ((uint64_t)(x))
#define be16toh(x)  ((uint16_t)(x))
#define be32toh(x)  ((uint32_t)(x))
#define be64toh(x)  ((uint64_t)(x))

#define htole16(x)  __builtin_bswap16(x)
#define htole32(x)  __builtin_bswap32(x)
#define htole64(x)  __builtin_bswap64(x)
#define le16toh(x)  __builtin_bswap16(x)
#define le32toh(x)  __builtin_bswap32(x)
#define le64toh(x)  __builtin_bswap64(x)

#endif

#endif /* _SYS_ENDIAN_H_ */
