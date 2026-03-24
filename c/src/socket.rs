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
// EAI error codes (must match netdb.h)
// ---------------------------------------------------------------------------

const EAI_AGAIN: i32 = 2;
const EAI_BADFLAGS: i32 = 3;
const EAI_FAIL: i32 = 4;
const EAI_FAMILY: i32 = 5;
const EAI_MEMORY: i32 = 6;
const EAI_NONAME: i32 = 8;
const EAI_SERVICE: i32 = 9;
const EAI_SOCKTYPE: i32 = 10;
const EAI_SYSTEM: i32 = 11;
const EAI_OVERFLOW: i32 = 14;

// AI flags (must match netdb.h)
const AI_PASSIVE: i32 = 1;
const AI_NUMERICHOST: i32 = 4;
const AI_NUMERICSERV: i32 = 0x400;

// Socket constants
const AF_UNSPEC: i32 = 0;
const AF_INET: i32 = 2;
const SOCK_STREAM: i32 = 1;
const SOCK_DGRAM: i32 = 2;
const IPPROTO_TCP: i32 = 6;
const IPPROTO_UDP: i32 = 17;

// Well-known service table: (name, port, preferred socktype)
const SERVICES: &[(&[u8], u16, i32)] = &[
    (b"http", 80, SOCK_STREAM),
    (b"https", 443, SOCK_STREAM),
    (b"ftp", 21, SOCK_STREAM),
    (b"ssh", 22, SOCK_STREAM),
    (b"telnet", 23, SOCK_STREAM),
    (b"smtp", 25, SOCK_STREAM),
    (b"domain", 53, SOCK_DGRAM),
    (b"dns", 53, SOCK_DGRAM),
    (b"ntp", 123, SOCK_DGRAM),
];

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

/// Parse a null-terminated C string as a numeric port number.
/// Returns `Some(port)` for valid u16, `None` otherwise.
unsafe fn parse_numeric_port(s: *const u8) -> Option<u16> {
    if s.is_null() {
        return None;
    }
    unsafe {
        let mut i = 0usize;
        let mut val: u32 = 0;
        if *s == 0 {
            return None;
        }
        while *s.add(i) != 0 {
            let c = *s.add(i);
            if c < b'0' || c > b'9' {
                return None;
            }
            val = val * 10 + (c - b'0') as u32;
            if val > 65535 {
                return None;
            }
            i += 1;
        }
        Some(val as u16)
    }
}

/// Look up a service name in the built-in table.
/// Returns `(port, preferred_socktype)` or `None`.
unsafe fn lookup_service(name: *const u8) -> Option<(u16, i32)> {
    if name.is_null() {
        return None;
    }
    unsafe {
        let mut len = 0usize;
        while *name.add(len) != 0 && len < 32 {
            len += 1;
        }
        if len == 0 {
            return None;
        }
        let name_slice = core::slice::from_raw_parts(name, len);
        for &(svc_name, port, st) in SERVICES {
            if svc_name.len() == name_slice.len() {
                let mut eq = true;
                for j in 0..svc_name.len() {
                    // Case-insensitive compare
                    let a = if name_slice[j] >= b'A' && name_slice[j] <= b'Z' {
                        name_slice[j] + 32
                    } else {
                        name_slice[j]
                    };
                    if a != svc_name[j] {
                        eq = false;
                        break;
                    }
                }
                if eq {
                    return Some((port, st));
                }
            }
        }
        None
    }
}

/// Parse a null-terminated C string as a dotted-decimal IPv4 address.
/// Returns the IP in host byte order, or `None`.
unsafe fn parse_numeric_ipv4(s: *const u8) -> Option<u32> {
    if s.is_null() {
        return None;
    }
    unsafe {
        let mut len = 0usize;
        while *s.add(len) != 0 && len < 64 {
            len += 1;
        }
        if len == 0 {
            return None;
        }
        salty::dns::parse_ipv4_numeric(core::slice::from_raw_parts(s, len))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getaddrinfo(
    node: *const u8,
    service: *const u8,
    hints: *const AddrInfo,
    res: *mut *mut AddrInfo,
) -> i32 {
    if res.is_null() {
        return EAI_SYSTEM;
    }
    unsafe {
        *res = core::ptr::null_mut();

        // Parse hints
        let (hint_family, hint_socktype, hint_protocol, hint_flags) = if !hints.is_null() {
            (
                (*hints).ai_family,
                (*hints).ai_socktype,
                (*hints).ai_protocol,
                (*hints).ai_flags,
            )
        } else {
            (AF_UNSPEC, 0, 0, 0)
        };

        // Validate family
        if hint_family != AF_UNSPEC && hint_family != AF_INET {
            return EAI_FAMILY;
        }
        // Validate socktype
        if hint_socktype != 0 && hint_socktype != SOCK_STREAM && hint_socktype != SOCK_DGRAM {
            return EAI_SOCKTYPE;
        }

        // Parse service -> port
        let port: u16 = if service.is_null() || *service == 0 {
            0
        } else if let Some(p) = parse_numeric_port(service) {
            p
        } else {
            // Named service
            if hint_flags & AI_NUMERICSERV != 0 {
                return EAI_SERVICE;
            }
            match lookup_service(service) {
                Some((p, _)) => p,
                None => return EAI_SERVICE,
            }
        };

        // Resolve addresses
        let mut addrs = [0u32; 4];
        let mut addr_count: usize = 0;

        if node.is_null() || *node == 0 {
            // No node specified
            if hint_flags & AI_PASSIVE != 0 {
                addrs[0] = 0; // INADDR_ANY
            } else {
                addrs[0] = 0x7f_00_00_01; // 127.0.0.1
            }
            addr_count = 1;
        } else if let Some(ip) = parse_numeric_ipv4(node) {
            addrs[0] = ip;
            addr_count = 1;
        } else {
            // Non-numeric host
            if hint_flags & AI_NUMERICHOST != 0 {
                return EAI_NONAME;
            }
            // DNS lookup
            let mut len = 0usize;
            while *node.add(len) != 0 && len < 120 {
                len += 1;
            }
            if len == 0 || len >= 120 {
                return EAI_NONAME;
            }
            let hostname = core::slice::from_raw_parts(node, len);
            let dns = salty::dns::dns_resolve_multi(hostname);
            if dns.count == 0 {
                return EAI_NONAME;
            }
            addr_count = dns.count as usize;
            if addr_count > 4 {
                addr_count = 4;
            }
            for i in 0..addr_count {
                addrs[i] = dns.addrs[i];
            }
        }

        // Determine socktype/protocol combinations
        let combos: &[(i32, i32)] = match hint_socktype {
            SOCK_STREAM => &[(SOCK_STREAM, IPPROTO_TCP)],
            SOCK_DGRAM => &[(SOCK_DGRAM, IPPROTO_UDP)],
            _ => {
                if hint_protocol == IPPROTO_TCP {
                    &[(SOCK_STREAM, IPPROTO_TCP)]
                } else if hint_protocol == IPPROTO_UDP {
                    &[(SOCK_DGRAM, IPPROTO_UDP)]
                } else {
                    &[(SOCK_STREAM, IPPROTO_TCP), (SOCK_DGRAM, IPPROTO_UDP)]
                }
            }
        };

        // Build linked list via malloc
        let ai_size = core::mem::size_of::<AddrInfo>();
        let sa_size = core::mem::size_of::<SockAddrIn>();
        let alloc_size = ai_size + sa_size;

        let mut head: *mut AddrInfo = core::ptr::null_mut();
        let mut tail: *mut AddrInfo = core::ptr::null_mut();

        for i in 0..addr_count {
            for &(socktype, protocol) in combos {
                let ptr = crate::malloc::malloc(alloc_size);
                if ptr.is_null() {
                    freeaddrinfo(head);
                    return EAI_MEMORY;
                }
                // Zero the allocation
                core::ptr::write_bytes(ptr, 0, alloc_size);

                let ai = ptr as *mut AddrInfo;
                let sa = ptr.add(ai_size) as *mut SockAddrIn;

                // Fill SockAddrIn (network byte order for sin_port/sin_addr)
                (*sa).sin_family = AF_INET as u16;
                (*sa).sin_port = port.to_be();
                (*sa).sin_addr = addrs[i].to_be();

                // Fill AddrInfo
                (*ai).ai_flags = hint_flags;
                (*ai).ai_family = AF_INET;
                (*ai).ai_socktype = socktype;
                (*ai).ai_protocol = protocol;
                (*ai).ai_addrlen = sa_size as u32;
                (*ai).ai_addr = sa as *mut u8;
                (*ai).ai_canonname = core::ptr::null_mut();
                (*ai).ai_next = core::ptr::null_mut();

                if head.is_null() {
                    head = ai;
                } else {
                    (*tail).ai_next = ai;
                }
                tail = ai;
            }
        }

        *res = head;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freeaddrinfo(res: *mut AddrInfo) {
    unsafe {
        let mut cur = res;
        while !cur.is_null() {
            let next = (*cur).ai_next;
            crate::malloc::free(cur as *mut u8);
            cur = next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getnameinfo(
    sa: *const u8,
    _salen: u32,
    host: *mut u8,
    hostlen: u32,
    serv: *mut u8,
    servlen: u32,
    flags: i32,
) -> i32 {
    unsafe {
        if sa.is_null() {
            return EAI_FAIL;
        }
        let sin = sa as *const SockAddrIn;

        // Host portion
        if !host.is_null() && hostlen > 0 {
            let ip_be = (*sin).sin_addr;
            let ip = u32::from_be(ip_be);
            let a = (ip >> 24) & 0xff;
            let b = (ip >> 16) & 0xff;
            let c = (ip >> 8) & 0xff;
            let d = ip & 0xff;

            if flags & 1 != 0 {
                // NI_NUMERICHOST: always numeric
                let mut buf = [0u8; 16];
                let len = fmt_ipv4(&mut buf, a, b, c, d);
                if len + 1 > hostlen as usize {
                    return EAI_OVERFLOW;
                }
                core::ptr::copy_nonoverlapping(buf.as_ptr(), host, len);
                *host.add(len) = 0;
            } else {
                // Try reverse DNS, fall back to numeric
                let rlen = salty::dns::dns_reverse_lookup(ip, host, (hostlen - 1) as usize);
                if rlen > 0 {
                    *host.add(rlen) = 0;
                } else {
                    let mut buf = [0u8; 16];
                    let len = fmt_ipv4(&mut buf, a, b, c, d);
                    if len + 1 > hostlen as usize {
                        return EAI_OVERFLOW;
                    }
                    core::ptr::copy_nonoverlapping(buf.as_ptr(), host, len);
                    *host.add(len) = 0;
                }
            }
        }

        // Service portion
        if !serv.is_null() && servlen > 0 {
            let port = u16::from_be((*sin).sin_port);
            let mut buf = [0u8; 6];
            let len = fmt_u16(&mut buf, port);
            if len + 1 > servlen as usize {
                return EAI_OVERFLOW;
            }
            core::ptr::copy_nonoverlapping(buf.as_ptr(), serv, len);
            *serv.add(len) = 0;
        }

        0
    }
}

/// Format an IPv4 address as "a.b.c.d" into `buf`. Returns length written.
fn fmt_ipv4(buf: &mut [u8; 16], a: u32, b: u32, c: u32, d: u32) -> usize {
    let mut pos = 0usize;
    pos += fmt_u32_into(buf, pos, a);
    buf[pos] = b'.';
    pos += 1;
    pos += fmt_u32_into(buf, pos, b);
    buf[pos] = b'.';
    pos += 1;
    pos += fmt_u32_into(buf, pos, c);
    buf[pos] = b'.';
    pos += 1;
    pos += fmt_u32_into(buf, pos, d);
    pos
}

fn fmt_u32_into(buf: &mut [u8; 16], start: usize, val: u32) -> usize {
    if val >= 100 {
        buf[start] = b'0' + (val / 100) as u8;
        buf[start + 1] = b'0' + ((val / 10) % 10) as u8;
        buf[start + 2] = b'0' + (val % 10) as u8;
        3
    } else if val >= 10 {
        buf[start] = b'0' + (val / 10) as u8;
        buf[start + 1] = b'0' + (val % 10) as u8;
        2
    } else {
        buf[start] = b'0' + val as u8;
        1
    }
}

fn fmt_u16(buf: &mut [u8; 6], val: u16) -> usize {
    let v = val as u32;
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut digits = [0u8; 5];
    let mut n = v;
    let mut i = 0usize;
    while n > 0 {
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    for j in 0..i {
        buf[j] = digits[i - 1 - j];
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gai_strerror(errcode: i32) -> *const u8 {
    match errcode {
        0 => b"Success\0".as_ptr(),
        EAI_AGAIN => b"Temporary failure in name resolution\0".as_ptr(),
        EAI_BADFLAGS => b"Invalid value for ai_flags\0".as_ptr(),
        EAI_FAIL => b"Non-recoverable failure in name resolution\0".as_ptr(),
        EAI_FAMILY => b"ai_family not supported\0".as_ptr(),
        EAI_MEMORY => b"Memory allocation failure\0".as_ptr(),
        EAI_NONAME => b"Name or service not known\0".as_ptr(),
        EAI_SERVICE => b"Servname not supported for ai_socktype\0".as_ptr(),
        EAI_SOCKTYPE => b"ai_socktype not supported\0".as_ptr(),
        EAI_SYSTEM => b"System error\0".as_ptr(),
        EAI_OVERFLOW => b"Argument buffer overflow\0".as_ptr(),
        _ => b"Unknown error\0".as_ptr(),
    }
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
