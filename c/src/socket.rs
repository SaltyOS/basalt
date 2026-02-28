// SPDX-License-Identifier: GPL-2.0-only
//! BSD/POSIX socket wrappers
//!
//! Standard-name socket functions (socket, bind, listen, accept, connect,
//! send, recv, sendto, recvfrom, sendmsg, recvmsg, shutdown, socketpair)
//! plus poll, epoll, and DNS name resolution (getaddrinfo/freeaddrinfo).
//!
//! All functions delegate to `salty::posix::*` or `salty::dns::*`.

use crate::errno;

// ---------------------------------------------------------------------------
// C-compatible structures
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct MsgHdr {
    pub msg_name: *mut u8,
    pub msg_namelen: u32,
    _pad0: u32,
    pub msg_iov: *mut IoVec,
    pub msg_iovlen: usize,
    pub msg_control: *mut u8,
    pub msg_controllen: usize,
    pub msg_flags: i32,
    _pad1: i32,
}

#[repr(C)]
pub struct IoVec {
    pub iov_base: *mut u8,
    pub iov_len: usize,
}

#[repr(C)]
pub struct AddrInfo {
    pub ai_flags: i32,
    pub ai_family: i32,
    pub ai_socktype: i32,
    pub ai_protocol: i32,
    pub ai_addrlen: u32,
    _pad0: u32,
    pub ai_addr: *mut u8,
    pub ai_canonname: *mut u8,
    pub ai_next: *mut AddrInfo,
}

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    _pad0: u32,
    pub data: u64,
}

// ---------------------------------------------------------------------------
// Static storage for getaddrinfo result (no heap)
// ---------------------------------------------------------------------------

static mut STATIC_ADDRINFO: AddrInfo = AddrInfo {
    ai_flags: 0,
    ai_family: 0,
    ai_socktype: 0,
    ai_protocol: 0,
    ai_addrlen: 0,
    _pad0: 0,
    ai_addr: core::ptr::null_mut(),
    ai_canonname: core::ptr::null_mut(),
    ai_next: core::ptr::null_mut(),
};

static mut STATIC_SOCKADDR: SockAddrIn = SockAddrIn {
    sin_family: 0,
    sin_port: 0,
    sin_addr: 0,
    sin_zero: [0; 8],
};

// EAI error codes
const EAI_NONAME: i32 = -2;
const EAI_SYSTEM: i32 = -11;

// ---------------------------------------------------------------------------
// Socket creation / connection
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn socket(domain: i32, sock_type: i32, _protocol: i32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_socket(domain, sock_type);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(fd: i32, addr: *const u8, addrlen: u32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_bind(fd, addr, addrlen);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn listen(fd: i32, backlog: i32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_listen(fd, backlog);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn accept(fd: i32, _addr: *mut u8, _addrlen: *mut u32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_accept(fd);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn connect(fd: i32, addr: *const u8, addrlen: u32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_connect(fd, addr, addrlen);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shutdown(fd: i32, how: i32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_shutdown(fd, how);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn socketpair(
    _domain: i32,
    _sock_type: i32,
    _protocol: i32,
    sv: *mut i32,
) -> i32 {
    unsafe {
        let ret = salty::posix::posix_socketpair(sv);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

// ---------------------------------------------------------------------------
// Send / receive
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn send(fd: i32, buf: *const u8, len: usize, _flags: i32) -> isize {
    unsafe {
        let ret = salty::posix::posix_sendmsg(
            fd,
            buf,
            len as u64,
            core::ptr::null(),
            0,
        );
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn recv(fd: i32, buf: *mut u8, len: usize, _flags: i32) -> isize {
    unsafe {
        let ret = salty::posix::posix_recvmsg(
            fd,
            buf,
            len as u64,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendto(
    fd: i32,
    buf: *const u8,
    len: usize,
    flags: i32,
    addr: *const u8,
    addrlen: u32,
) -> isize {
    unsafe {
        let ret = salty::posix::posix_sendto(fd, buf, len, flags, addr, addrlen);
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvfrom(
    fd: i32,
    buf: *mut u8,
    len: usize,
    flags: i32,
    addr: *mut u8,
    addrlen: *mut u32,
) -> isize {
    unsafe {
        let ret = salty::posix::posix_recvfrom(fd, buf, len, flags, addr, addrlen);
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendmsg(fd: i32, msg: *const MsgHdr, _flags: i32) -> isize {
    if msg.is_null() {
        errno::set_errno(errno::EINVAL);
        return -1;
    }
    unsafe {
        // Extract first iov entry for data (simple single-iov path)
        let (data, data_len) = if (*msg).msg_iov.is_null() || (*msg).msg_iovlen == 0 {
            (core::ptr::null(), 0u64)
        } else {
            let iov = &*(*msg).msg_iov;
            (iov.iov_base as *const u8, iov.iov_len as u64)
        };
        let ret = salty::posix::posix_sendmsg(fd, data, data_len, core::ptr::null(), 0);
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvmsg(fd: i32, msg: *mut MsgHdr, _flags: i32) -> isize {
    if msg.is_null() {
        errno::set_errno(errno::EINVAL);
        return -1;
    }
    unsafe {
        // Extract first iov entry for data buffer
        let (data, data_len) = if (*msg).msg_iov.is_null() || (*msg).msg_iovlen == 0 {
            (core::ptr::null_mut(), 0u64)
        } else {
            let iov = &mut *(*msg).msg_iov;
            (iov.iov_base, iov.iov_len as u64)
        };
        let ret = salty::posix::posix_recvmsg(
            fd,
            data,
            data_len,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        ret as isize
    }
}

// ---------------------------------------------------------------------------
// Socket options (stubs)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setsockopt(
    _fd: i32,
    _level: i32,
    _optname: i32,
    _optval: *const u8,
    _optlen: u32,
) -> i32 {
    // Stub: silently succeed for most socket options
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockopt(
    _fd: i32,
    _level: i32,
    _optname: i32,
    _optval: *mut u8,
    _optlen: *mut u32,
) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockname(
    _fd: i32,
    _addr: *mut u8,
    _addrlen: *mut u32,
) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpeername(
    _fd: i32,
    _addr: *mut u8,
    _addrlen: *mut u32,
) -> i32 {
    errno::set_errno(errno::ENOSYS);
    -1
}

// ---------------------------------------------------------------------------
// DNS name resolution
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getaddrinfo(
    node: *const u8,
    _service: *const u8,
    _hints: *const u8,
    res: *mut *mut AddrInfo,
) -> i32 {
    if res.is_null() {
        return EAI_NONAME;
    }
    unsafe {
        if node.is_null() {
            *res = core::ptr::null_mut();
            return EAI_NONAME;
        }

        let mut dns_result = salty::types::DnsAddrInfo {
            family: 0,
            socktype: 0,
            protocol: 0,
            addr: salty::types::SockAddrIn {
                family: 0,
                port: 0,
                addr: 0,
            },
        };

        let ret = salty::dns::posix_getaddrinfo(node, &raw mut dns_result);
        if ret != 0 {
            *res = core::ptr::null_mut();
            return EAI_NONAME;
        }

        // Fill static SockAddrIn
        let sa = &raw mut STATIC_SOCKADDR;
        (*sa).sin_family = dns_result.addr.family;
        (*sa).sin_port = dns_result.addr.port;
        (*sa).sin_addr = dns_result.addr.addr;
        (*sa).sin_zero = [0; 8];

        // Fill static AddrInfo
        let ai = &raw mut STATIC_ADDRINFO;
        (*ai).ai_flags = 0;
        (*ai).ai_family = dns_result.family;
        (*ai).ai_socktype = dns_result.socktype;
        (*ai).ai_protocol = dns_result.protocol;
        (*ai).ai_addrlen = core::mem::size_of::<SockAddrIn>() as u32;
        (*ai).ai_addr = sa as *mut u8;
        (*ai).ai_canonname = core::ptr::null_mut();
        (*ai).ai_next = core::ptr::null_mut();

        *res = ai;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freeaddrinfo(_res: *mut AddrInfo) {
    // No-op: static allocation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getnameinfo(
    _sa: *const u8,
    _salen: u32,
    _host: *mut u8,
    _hostlen: u32,
    _serv: *mut u8,
    _servlen: u32,
    _flags: i32,
) -> i32 {
    EAI_NONAME
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gai_strerror(_errcode: i32) -> *const u8 {
    b"Name resolution error\0".as_ptr()
}

// ---------------------------------------------------------------------------
// poll / epoll
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn poll(fds: *mut PollFd, nfds: u32, timeout: i32) -> i32 {
    unsafe {
        // PollFd layout matches salty::PollFd exactly (fd: i32, events: i16, revents: i16)
        let ret = salty::posix::posix_poll(fds as *mut salty::PollFd, nfds, timeout);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ppoll(
    fds: *mut PollFd,
    nfds: u32,
    timeout: *const crate::unistd::Timespec,
    _sigmask: *const u8,
) -> i32 {
    unsafe {
        let timeout_ms = if timeout.is_null() {
            -1i32
        } else {
            let ms = (*timeout).tv_sec * 1000 + (*timeout).tv_nsec / 1_000_000;
            if ms > i32::MAX as i64 { i32::MAX } else { ms as i32 }
        };
        poll(fds, nfds, timeout_ms)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn epoll_create(_size: i32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_epoll_create();
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn epoll_create1(_flags: i32) -> i32 {
    unsafe {
        let ret = salty::posix::posix_epoll_create();
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn epoll_ctl(
    epfd: i32,
    op: i32,
    fd: i32,
    event: *mut EpollEvent,
) -> i32 {
    unsafe {
        let (events, data) = if event.is_null() {
            (0u32, 0u64)
        } else {
            ((*event).events, (*event).data)
        };
        let ret = salty::posix::posix_epoll_ctl(epfd, op, fd, events, data);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn epoll_wait(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
) -> i32 {
    unsafe {
        // EpollEvent layout matches salty::types::EpollEvent (events: u32, data: u64)
        // but we have a padding field. Use salty EpollEvent directly via cast.
        let ret = salty::posix::posix_epoll_wait(
            epfd,
            events as *mut salty::types::EpollEvent,
            maxevents,
            timeout,
        );
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn epoll_pwait(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
    _sigmask: *const u8,
) -> i32 {
    unsafe { epoll_wait(epfd, events, maxevents, timeout) }
}
