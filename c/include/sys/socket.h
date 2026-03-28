/* SPDX-License-Identifier: GPL-2.0-only */
#ifndef __SYS_SOCKET_H__
#define __SYS_SOCKET_H__

#include <sys/types.h>
#include <sys/cdefs.h>
#include <sys/uio.h>

#define AF_UNSPEC 0
#define AF_UNIX   1
#define AF_LOCAL  AF_UNIX
#define AF_INET   2
#define AF_INET6  10

#define PF_UNSPEC AF_UNSPEC
#define PF_UNIX   AF_UNIX
#define PF_LOCAL  AF_LOCAL
#define PF_INET   AF_INET
#define PF_INET6  AF_INET6

#define SOCK_STREAM    1
#define SOCK_DGRAM     2
#define SOCK_CLOEXEC   0x80000
#define SOCK_NONBLOCK  0x800

#define SOL_SOCKET  1
#define IPPROTO_TCP 6

#define SO_REUSEADDR  2
#define SO_ERROR      4
#define SO_BROADCAST  6
#define SO_KEEPALIVE  9
#define SO_SNDBUF     7
#define SO_RCVBUF     8
#define SO_TIMESTAMP  0x00000400
#define SO_SNDTIMEO  21
#define SO_RCVTIMEO  20
#define SO_TYPE      3
#define SO_TS_CLOCK  0x1017
#define SO_TS_MONOTONIC 3
#define SO_DOMAIN    39
#define SO_PROTOCOL  38
#define SO_REUSEPORT 15

#define TCP_NODELAY 1

#define MSG_DONTWAIT  0x40
#define MSG_PEEK      0x02
#define MSG_NOSIGNAL  0x4000
#define MSG_CMSG_CLOEXEC 0x40000000

#define SHUT_RD   0
#define SHUT_WR   1
#define SHUT_RDWR 2

#define SOMAXCONN 128

#define SCM_RIGHTS 1
#define SCM_TIMESTAMP 2

typedef unsigned int socklen_t;

struct sockaddr {
    unsigned short sa_family;
    char           sa_data[14];
};

struct sockaddr_storage {
    unsigned short ss_family;
    char           __ss_pad1[6];    /* Align to 8 bytes */
    long long      __ss_align;      /* Force 8-byte alignment */
    char           __ss_pad2[112];  /* Total: 128 bytes */
};

#ifndef __SOCKADDR_UN_DEFINED__
#define __SOCKADDR_UN_DEFINED__
struct sockaddr_un {
    unsigned short sun_family;
    char           sun_path[108];
};
#endif

struct msghdr {
    void         *msg_name;
    socklen_t     msg_namelen;
    struct iovec *msg_iov;
    int           msg_iovlen;
    void         *msg_control;
    socklen_t     msg_controllen;
    int           msg_flags;
};

struct cmsghdr {
    socklen_t cmsg_len;
    int       cmsg_level;
    int       cmsg_type;
};

#define __CMSG_ALIGN(n) (((n) + sizeof(long) - 1) & ~(sizeof(long) - 1))
#define CMSG_DATA(cmsg) ((unsigned char *)(cmsg) + __CMSG_ALIGN(sizeof(struct cmsghdr)))
#define CMSG_FIRSTHDR(mhdr) \
    ((mhdr)->msg_controllen >= sizeof(struct cmsghdr) ? \
        (struct cmsghdr *)(mhdr)->msg_control : \
        (struct cmsghdr *)0)
#define CMSG_NXTHDR(mhdr, cmsg) \
    ((char *)(cmsg) + __CMSG_ALIGN((cmsg)->cmsg_len) + __CMSG_ALIGN(sizeof(struct cmsghdr)) > \
     (char *)(mhdr)->msg_control + (mhdr)->msg_controllen ? \
        (struct cmsghdr *)0 : \
        (struct cmsghdr *)((char *)(cmsg) + __CMSG_ALIGN((cmsg)->cmsg_len)))
#define CMSG_LEN(len) (__CMSG_ALIGN(sizeof(struct cmsghdr)) + (len))
#define CMSG_SPACE(len) (__CMSG_ALIGN(sizeof(struct cmsghdr)) + __CMSG_ALIGN(len))

__BEGIN_DECLS

extern int socket(int domain, int type, int protocol);
extern int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
extern int listen(int sockfd, int backlog);
extern int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
extern int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
extern ssize_t send(int sockfd, const void *buf, size_t len, int flags);
extern ssize_t recv(int sockfd, void *buf, size_t len, int flags);
extern ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
                      const struct sockaddr *dest_addr, socklen_t addrlen);
extern ssize_t recvfrom(int sockfd, void *buf, size_t len, int flags,
                        struct sockaddr *src_addr, socklen_t *addrlen);
extern ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags);
extern ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags);
extern int shutdown(int sockfd, int how);
extern int socketpair(int domain, int type, int protocol, int sv[2]);
extern int getsockopt(int sockfd, int level, int optname,
                      void *optval, socklen_t *optlen);
extern int setsockopt(int sockfd, int level, int optname,
                      const void *optval, socklen_t optlen);
extern int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
extern int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen);

__END_DECLS

#endif /* __SYS_SOCKET_H__ */
