// SPDX-License-Identifier: GPL-2.0-only
//! BSD/POSIX socket wrappers
//!
//! Standard-name socket functions (socket, bind, listen, accept, connect,
//! send, recv, sendto, recvfrom, sendmsg, recvmsg, shutdown, socketpair)
//! plus poll, epoll, and DNS name resolution (getaddrinfo/freeaddrinfo).
//!
//! All functions delegate to `trona_posix::*` or `trona_posix::dns::*`.

use crate::errno;
use trona::consts::SOL_SOCKET;

static mut LOGGED_SOCKET_META_CALLS: u8 = 0;
static mut LOGGED_RECVMSG_INET_RESULTS: u8 = 0;
const BSD_SOL_SOCKET: i32 = 0xFFFF;

#[inline]
fn cmsg_sol_socket_level() -> i32 {
    BSD_SOL_SOCKET
}

#[inline]
fn is_sol_socket_level(level: i32) -> bool {
    level == SOL_SOCKET || level == BSD_SOL_SOCKET
}

fn log_socket_meta(op: &[u8], fd: i32, a: i32, b: i32) {
    unsafe {
        if *(&raw const LOGGED_SOCKET_META_CALLS) >= 24 {
            return;
        }
        *(&raw mut LOGGED_SOCKET_META_CALLS) += 1;
    }
    trona::udebug!(|_lb| {
        _lb.str(b"[libc] ");
        _lb.str(op);
        _lb.str(b" fd=");
        _lb.dec(fd as u64);
        if a >= 0 {
            _lb.str(b" a=");
            _lb.dec(a as u64);
        }
        if b >= 0 {
            _lb.str(b" b=");
            _lb.dec(b as u64);
        }
        _lb.putc(b'\n');
    });
}

fn log_recvmsg_inet_result(fd: i32, ret: isize, data: &[u8], user_data: Option<&[u8]>) {
    unsafe {
        if *(&raw const LOGGED_RECVMSG_INET_RESULTS) >= 24 {
            return;
        }
        *(&raw mut LOGGED_RECVMSG_INET_RESULTS) += 1;
    }
    trona::udebug!(|_lb| {
        _lb.str(b"[libc] recvmsg inet fd=");
        _lb.dec(fd as u64);
        _lb.str(b" ret=");
        if ret >= 0 {
            _lb.dec(ret as u64);
        } else {
            _lb.str(b"-");
            _lb.dec((-ret) as u64);
        }
        let preview_len = core::cmp::min(data.len(), 8);
        if preview_len > 0 {
            _lb.str(b" bytes=");
            let mut i = 0;
            while i < preview_len {
                if i != 0 {
                    _lb.putc(b':');
                }
                _lb.hex(data[i] as u64);
                i += 1;
            }
        }
        if data.len() >= 28 {
            let icmp_type = data[20];
            let icmp_code = data[21];
            let icmp_id_lo = data[24];
            let icmp_id_hi = data[25];
            let icmp_seq_lo = data[26];
            let icmp_seq_hi = data[27];
            _lb.str(b" icmp=");
            _lb.hex(icmp_type as u64);
            _lb.putc(b'/');
            _lb.hex(icmp_code as u64);
            _lb.str(b" id_bytes=");
            _lb.hex(icmp_id_lo as u64);
            _lb.putc(b':');
            _lb.hex(icmp_id_hi as u64);
            _lb.str(b" seq_bytes=");
            _lb.hex(icmp_seq_lo as u64);
            _lb.putc(b':');
            _lb.hex(icmp_seq_hi as u64);
        }
        if let Some(user) = user_data {
            let user_preview_len = core::cmp::min(user.len(), 8);
            if user_preview_len > 0 {
                _lb.str(b" user=");
                let mut i = 0;
                while i < user_preview_len {
                    if i != 0 {
                        _lb.putc(b':');
                    }
                    _lb.hex(user[i] as u64);
                    i += 1;
                }
            }
        }
        _lb.putc(b'\n');
    });
}

unsafe fn decode_sockaddr_in(addr: *const u8, addrlen: u32) -> Option<(u32, u16)> {
    if addr.is_null() || addrlen < 8 {
        return None;
    }

    let family = unsafe { core::ptr::read_unaligned(addr as *const u16) };
    if family == AF_INET as u16 {
        let port = unsafe { u16::from_be(core::ptr::read_unaligned(addr.add(2) as *const u16)) };
        let ip = unsafe { u32::from_be(core::ptr::read_unaligned(addr.add(4) as *const u32)) };
        return Some((ip, port));
    }

    let sa_len = unsafe { *addr };
    let sa_family = unsafe { *addr.add(1) };
    if sa_family == AF_INET as u8 && sa_len as u32 >= 8 {
        let port = unsafe { u16::from_be(core::ptr::read_unaligned(addr.add(2) as *const u16)) };
        let ip = unsafe { u32::from_be(core::ptr::read_unaligned(addr.add(4) as *const u32)) };
        return Some((ip, port));
    }

    None
}

unsafe fn decode_posix_sockaddr_in(addr: *const u8, addrlen: u32) -> Option<trona::types::SockAddrIn> {
    if addr.is_null() || addrlen < 8 {
        return None;
    }

    let family = unsafe { core::ptr::read_unaligned(addr as *const u16) };
    if family == AF_INET as u16 {
        let mut posix = trona::types::SockAddrIn::zeroed();
        posix.family = AF_INET as u16;
        posix.port = unsafe { core::ptr::read_unaligned(addr.add(2) as *const u16) };
        posix.addr = unsafe { core::ptr::read_unaligned(addr.add(4) as *const u32) };
        return Some(posix);
    }

    let sa_len = unsafe { *addr };
    let sa_family = unsafe { *addr.add(1) };
    if sa_family == AF_INET as u8 && sa_len as u32 >= 8 {
        let mut posix = trona::types::SockAddrIn::zeroed();
        posix.family = AF_INET as u16;
        posix.port = unsafe { core::ptr::read_unaligned(addr.add(2) as *const u16) };
        posix.addr = unsafe { core::ptr::read_unaligned(addr.add(4) as *const u32) };
        return Some(posix);
    }

    None
}

unsafe fn fill_sockaddr_in(addr: *mut u8, ip: u32, port: u16) {
    if addr.is_null() {
        return;
    }

    unsafe {
        let sa = &mut *(addr as *mut SockAddrIn);
        sa.sin_family = AF_INET as u16;
        sa.sin_port = port.to_be();
        sa.sin_addr = ip.to_be();
        sa.sin_zero = [0; 8];
    }
}

// ---------------------------------------------------------------------------
// C-compatible structures
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct MsgHdr {
    pub msg_name: *mut u8,
    pub msg_namelen: u32,
    _pad0: u32,
    pub msg_iov: *mut IoVec,
    pub msg_iovlen: i32,
    pub msg_control: *mut u8,
    pub msg_controllen: u32,
    pub msg_flags: i32,
}

#[repr(C)]
pub struct IoVec {
    pub iov_base: *mut u8,
    pub iov_len: usize,
}

#[repr(C)]
pub struct CmsgHdr {
    pub cmsg_len: u32,
    pub cmsg_level: i32,
    pub cmsg_type: i32,
}

#[repr(C)]
struct TimeSpec {
    tv_sec: i64,
    tv_nsec: i64,
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

#[inline]
const fn cmsg_align(len: usize) -> usize {
    let align = core::mem::size_of::<isize>();
    (len + align - 1) & !(align - 1)
}

#[inline]
const fn cmsg_header_len() -> usize {
    cmsg_align(core::mem::size_of::<CmsgHdr>())
}

#[inline]
const fn cmsg_len(payload_len: usize) -> usize {
    cmsg_header_len() + payload_len
}

unsafe fn socket_domain(fd: i32) -> Option<i32> {
    unsafe {
        let mut domain = 0u32;
        let mut domain_len = core::mem::size_of::<u32>() as u32;
        let ret = trona_posix::posix_getsockopt(
            fd,
            SOL_SOCKET,
            trona::consts::SO_DOMAIN,
            &raw mut domain as *mut u32 as *mut u8,
            &raw mut domain_len,
        );
        if ret < 0 || domain_len < core::mem::size_of::<u32>() as u32 {
            None
        } else {
            Some(domain as i32)
        }
    }
}

unsafe fn store_timestamp_cmsg(msg: *mut MsgHdr, timestamp_ns: u64) {
    if msg.is_null() {
        return;
    }

    let hdr = unsafe { &mut *msg };
    hdr.msg_flags = 0;
    if hdr.msg_control.is_null()
        || (hdr.msg_controllen as usize) < cmsg_len(core::mem::size_of::<TimeSpec>())
    {
        hdr.msg_controllen = 0;
        return;
    }

    if timestamp_ns == trona::consts::INET_RECV_TIMESTAMP_NONE {
        hdr.msg_controllen = 0;
        return;
    }

    let ts = TimeSpec {
        tv_sec: (timestamp_ns / 1_000_000_000) as i64,
        tv_nsec: (timestamp_ns % 1_000_000_000) as i64,
    };
    let cmsg = CmsgHdr {
        cmsg_len: cmsg_len(core::mem::size_of::<TimeSpec>()) as u32,
        cmsg_level: cmsg_sol_socket_level(),
        cmsg_type: trona::consts::SCM_TIMESTAMP,
    };
    unsafe {
        core::ptr::write_unaligned(hdr.msg_control as *mut CmsgHdr, cmsg);
        core::ptr::copy_nonoverlapping(
            &raw const ts as *const TimeSpec as *const u8,
            hdr.msg_control.add(cmsg_header_len()),
            core::mem::size_of::<TimeSpec>(),
        );
    }
    hdr.msg_controllen = cmsg_len(core::mem::size_of::<TimeSpec>()) as u32;
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
const SOCK_NONBLOCK: i32 = 0x800;
const SOCK_CLOEXEC: i32 = 0x80000;
const IPPROTO_TCP: i32 = 6;
const IPPROTO_UDP: i32 = 17;
const HOST_NOT_FOUND: i32 = 1;
const TRY_AGAIN: i32 = 2;
const NO_RECOVERY: i32 = 3;

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

#[inline]
fn dns_label_to_eai(label: u64) -> i32 {
    match label {
        trona::consts::TRONA_TIMED_OUT => EAI_AGAIN,
        trona::consts::TRONA_DNS_SERVER_FAIL => EAI_FAIL,
        trona::consts::TRONA_DNS_NXDOMAIN | trona::consts::TRONA_NOT_FOUND => EAI_NONAME,
        trona::consts::TRONA_INVALID_OPERATION
        | trona::consts::TRONA_INVALID_CAPABILITY
        | trona::consts::TRONA_BUSY
        | trona::consts::TRONA_CANCELLED => EAI_AGAIN,
        _ => EAI_AGAIN,
    }
}

#[inline]
unsafe fn set_h_errno_from_dns_label(label: u64) {
    unsafe {
        crate::inet::h_errno = match label {
            trona::consts::TRONA_TIMED_OUT => TRY_AGAIN,
            trona::consts::TRONA_DNS_SERVER_FAIL => NO_RECOVERY,
            trona::consts::TRONA_DNS_NXDOMAIN | trona::consts::TRONA_NOT_FOUND => HOST_NOT_FOUND,
            trona::consts::TRONA_INVALID_OPERATION
            | trona::consts::TRONA_INVALID_CAPABILITY
            | trona::consts::TRONA_BUSY
            | trona::consts::TRONA_CANCELLED => TRY_AGAIN,
            _ => TRY_AGAIN,
        };
    }
}

// ---------------------------------------------------------------------------
// Socket creation / connection
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn socket(domain: i32, sock_type: i32, protocol: i32) -> i32 {
    unsafe {
        let base_type = sock_type & !(SOCK_NONBLOCK | SOCK_CLOEXEC);
        let ret = trona_posix::posix_socket(domain, base_type, protocol);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        if (sock_type & SOCK_NONBLOCK) != 0 {
            let _ = trona_posix::posix_fcntl(ret, 4, trona_posix::O_NONBLOCK as i64);
        }
        if (sock_type & SOCK_CLOEXEC) != 0 {
            let _ = trona_posix::posix_fcntl(ret, 2, 1);
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(fd: i32, addr: *const u8, addrlen: u32) -> i32 {
    unsafe {
        let ret = if let Some(posix_addr) = decode_posix_sockaddr_in(addr, addrlen) {
            trona_posix::posix_bind(
                fd,
                &raw const posix_addr as *const trona::types::SockAddrIn as *const u8,
                core::mem::size_of::<trona::types::SockAddrIn>() as u32,
            )
        } else {
            trona_posix::posix_bind(fd, addr, addrlen)
        };
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
        let ret = trona_posix::posix_listen(fd, backlog);
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
        let ret = trona_posix::posix_accept(fd);
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
        let ret = if let Some(posix_addr) = decode_posix_sockaddr_in(addr, addrlen) {
            trona_posix::posix_connect(
                fd,
                &raw const posix_addr as *const trona::types::SockAddrIn as *const u8,
                core::mem::size_of::<trona::types::SockAddrIn>() as u32,
            )
        } else {
            trona_posix::posix_connect(fd, addr, addrlen)
        };
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
        let ret = trona_posix::posix_shutdown(fd, how);
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
        let ret = trona_posix::posix_socketpair(sv);
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
        let ret = trona_posix::posix_write(fd, buf, len as u64);
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
        let ret = trona_posix::posix_read(fd, buf, len as u64);
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
        let ret = if let Some(posix_addr) = decode_posix_sockaddr_in(addr, addrlen) {
            trona_posix::posix_sendto(
                fd,
                buf,
                len,
                flags,
                &raw const posix_addr as *const trona::types::SockAddrIn as *const u8,
                core::mem::size_of::<trona::types::SockAddrIn>() as u32,
            )
        } else {
            trona_posix::posix_sendto(fd, buf, len, flags, addr, addrlen)
        };
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
        let ret = if !addr.is_null() && !addrlen.is_null() && socket_domain(fd) == Some(AF_INET) {
            let mut host = trona::types::SockAddrIn::zeroed();
            let mut host_len = core::mem::size_of::<trona::types::SockAddrIn>() as u32;
            let ret = trona_posix::posix_recvfrom(
                fd,
                buf,
                len,
                flags,
                &raw mut host as *mut trona::types::SockAddrIn as *mut u8,
                &raw mut host_len,
            );
            if ret >= 0 {
                fill_sockaddr_in(addr, u32::from_be(host.addr), u16::from_be(host.port));
                *addrlen = core::mem::size_of::<SockAddrIn>() as u32;
            }
            ret
        } else {
            trona_posix::posix_recvfrom(fd, buf, len, flags, addr, addrlen)
        };
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
        let hdr = &*msg;
        let mut buf = [0u8; 120];
        let data_len = gather_iovecs(hdr.msg_iov, hdr.msg_iovlen as usize, &mut buf);
        let ret = if !hdr.msg_name.is_null()
            && hdr.msg_namelen >= core::mem::size_of::<SockAddrIn>() as u32
        {
            if let Some(posix_addr) = decode_posix_sockaddr_in(hdr.msg_name as *const u8, hdr.msg_namelen) {
                trona_posix::posix_sendto(
                    fd,
                    buf.as_ptr(),
                    data_len,
                    0,
                    &raw const posix_addr as *const trona::types::SockAddrIn as *const u8,
                    core::mem::size_of::<trona::types::SockAddrIn>() as u32,
                )
            } else {
                trona_posix::posix_sendto(
                    fd,
                    buf.as_ptr(),
                    data_len,
                    0,
                    hdr.msg_name as *const u8,
                    hdr.msg_namelen,
                )
            }
        } else {
            let mut rights = [0i32; 4];
            let rights_count = extract_scm_rights(msg, &mut rights);
            if rights_count > 0 {
                trona_posix::posix_sendmsg(
                    fd,
                    buf.as_ptr(),
                    data_len as u64,
                    rights.as_ptr(),
                    rights_count as u32,
                )
            } else {
                trona_posix::posix_write(fd, buf.as_ptr(), data_len as u64)
            }
        };
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
        let hdr = &mut *msg;
        hdr.msg_flags = 0;
        if !hdr.msg_control.is_null() && hdr.msg_controllen > 0 {
            core::ptr::write_bytes(hdr.msg_control, 0, hdr.msg_controllen as usize);
        }
        let mut cap = 0usize;
        if !hdr.msg_iov.is_null() && hdr.msg_iovlen > 0 {
            for i in 0..hdr.msg_iovlen as usize {
                let iov = &*hdr.msg_iov.add(i);
                cap = cap.saturating_add(iov.iov_len);
            }
        }
        let cap = core::cmp::min(cap, 152);
        let mut buf = [0u8; 152];

        let has_name = !hdr.msg_name.is_null()
            && hdr.msg_namelen >= core::mem::size_of::<SockAddrIn>() as u32;
        let has_control = !hdr.msg_control.is_null()
            && (hdr.msg_controllen as usize) >= cmsg_len(core::mem::size_of::<TimeSpec>());
        let domain = if has_name || has_control {
            socket_domain(fd)
        } else {
            None
        };

        let ret = if domain == Some(AF_INET) && (has_name || has_control) {
            let mut host_name = trona::types::SockAddrIn::zeroed();
            let mut name_len = core::mem::size_of::<trona::types::SockAddrIn>() as u32;
            let mut timestamp_ns = trona::consts::INET_RECV_TIMESTAMP_NONE;
            let ret = trona_posix::posix_recvmsg_inet(
                fd,
                buf.as_mut_ptr(),
                cap as u64,
                if has_name {
                    &raw mut host_name as *mut trona::types::SockAddrIn as *mut u8
                } else {
                    core::ptr::null_mut()
                },
                if has_name { &raw mut name_len } else { core::ptr::null_mut() },
                if has_control {
                    &raw mut timestamp_ns
                } else {
                    core::ptr::null_mut()
                },
            );
            let preview_len = if ret > 0 {
                core::cmp::min(ret as usize, buf.len())
            } else {
                0
            };
            if has_name {
                fill_sockaddr_in(
                    hdr.msg_name,
                    u32::from_be(host_name.addr),
                    u16::from_be(host_name.port),
                );
                hdr.msg_namelen = core::mem::size_of::<SockAddrIn>() as u32;
            }
            if has_control {
                store_timestamp_cmsg(msg, timestamp_ns);
            } else {
                hdr.msg_controllen = 0;
            }
            let user_preview = if ret > 0 && !hdr.msg_iov.is_null() && hdr.msg_iovlen > 0 {
                let first_iov = &*hdr.msg_iov;
                if !first_iov.iov_base.is_null() {
                    let take = core::cmp::min(preview_len, first_iov.iov_len);
                    Some(core::slice::from_raw_parts(first_iov.iov_base, take))
                } else {
                    None
                }
            } else {
                None
            };
            log_recvmsg_inet_result(fd, ret as isize, &buf[..preview_len], user_preview);
            ret
        } else if !hdr.msg_name.is_null()
            && hdr.msg_namelen >= core::mem::size_of::<SockAddrIn>() as u32
        {
            let mut host_name = trona::types::SockAddrIn::zeroed();
            let mut name_len = core::mem::size_of::<trona::types::SockAddrIn>() as u32;
            let ret = trona_posix::posix_recvfrom(
                fd,
                buf.as_mut_ptr(),
                cap,
                0,
                &raw mut host_name as *mut trona::types::SockAddrIn as *mut u8,
                &raw mut name_len,
            );
            if ret >= 0 {
                fill_sockaddr_in(
                    hdr.msg_name,
                    u32::from_be(host_name.addr),
                    u16::from_be(host_name.port),
                );
                hdr.msg_namelen = core::mem::size_of::<SockAddrIn>() as u32;
            }
            hdr.msg_controllen = 0;
            ret
        } else if !hdr.msg_control.is_null()
            && (hdr.msg_controllen as usize) >= cmsg_header_len()
        {
            let mut rights = [0i32; 4];
            let mut rights_len = rights.len() as u32;
            let ret = trona_posix::posix_recvmsg(
                fd,
                buf.as_mut_ptr(),
                cap as u64,
                rights.as_mut_ptr(),
                &raw mut rights_len,
            );
            if ret >= 0 {
                store_scm_rights(msg, &rights[..rights_len as usize]);
            } else {
                hdr.msg_controllen = 0;
            }
            ret
        } else {
            hdr.msg_controllen = 0;
            trona_posix::posix_read(fd, buf.as_mut_ptr(), cap as u64)
        };
        if ret < 0 {
            errno::set_errno((-ret) as i32);
            return -1;
        }
        let _ = scatter_iovecs(hdr.msg_iov, hdr.msg_iovlen as usize, &buf[..ret as usize]);
        ret as isize
    }
}

// ---------------------------------------------------------------------------
// Socket options (stubs)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setsockopt(
    fd: i32,
    level: i32,
    optname: i32,
    optval: *const u8,
    optlen: u32,
) -> i32 {
    log_socket_meta(b"setsockopt", fd, level, optname);
    unsafe {
        let ret = trona_posix::posix_setsockopt(fd, level, optname, optval, optlen);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockopt(
    fd: i32,
    level: i32,
    optname: i32,
    optval: *mut u8,
    optlen: *mut u32,
) -> i32 {
    log_socket_meta(b"getsockopt", fd, level, optname);
    unsafe {
        let ret = trona_posix::posix_getsockopt(fd, level, optname, optval, optlen);
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockname(
    fd: i32,
    addr: *mut u8,
    addrlen: *mut u32,
) -> i32 {
    log_socket_meta(b"getsockname", fd, -1, -1);
    unsafe {
        if addr.is_null() || addrlen.is_null() {
            errno::set_errno(errno::EFAULT);
            return -1;
        }
        if *addrlen < core::mem::size_of::<SockAddrIn>() as u32 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let mut host = trona::types::SockAddrIn::zeroed();
        let mut host_len = core::mem::size_of::<trona::types::SockAddrIn>() as u32;
        let ret = trona_posix::posix_getsockname(
            fd,
            &raw mut host as *mut trona::types::SockAddrIn as *mut u8,
            &raw mut host_len,
        );
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }

        fill_sockaddr_in(addr, host.addr, host.port);
        *addrlen = core::mem::size_of::<SockAddrIn>() as u32;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpeername(
    fd: i32,
    addr: *mut u8,
    addrlen: *mut u32,
) -> i32 {
    log_socket_meta(b"getpeername", fd, -1, -1);
    unsafe {
        if addr.is_null() || addrlen.is_null() {
            errno::set_errno(errno::EFAULT);
            return -1;
        }
        if *addrlen < core::mem::size_of::<SockAddrIn>() as u32 {
            errno::set_errno(errno::EINVAL);
            return -1;
        }

        let mut host = trona::types::SockAddrIn::zeroed();
        let mut host_len = core::mem::size_of::<trona::types::SockAddrIn>() as u32;
        let ret = trona_posix::posix_getpeername(
            fd,
            &raw mut host as *mut trona::types::SockAddrIn as *mut u8,
            &raw mut host_len,
        );
        if ret < 0 {
            errno::set_errno(-ret);
            return -1;
        }

        fill_sockaddr_in(addr, host.addr, host.port);
        *addrlen = core::mem::size_of::<SockAddrIn>() as u32;
        0
    }
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

unsafe fn gather_iovecs(iov: *mut IoVec, iovlen: usize, dst: &mut [u8]) -> usize {
    if iov.is_null() || iovlen == 0 || dst.is_empty() {
        return 0;
    }

    let mut copied = 0usize;
    for i in 0..iovlen {
        let ent = unsafe { &*iov.add(i) };
        if ent.iov_len == 0 || ent.iov_base.is_null() {
            continue;
        }
        let take = core::cmp::min(ent.iov_len, dst.len().saturating_sub(copied));
        if take == 0 {
            break;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(ent.iov_base, dst.as_mut_ptr().add(copied), take);
        }
        copied += take;
        if copied == dst.len() {
            break;
        }
    }
    copied
}

unsafe fn scatter_iovecs(iov: *mut IoVec, iovlen: usize, src: &[u8]) -> usize {
    if iov.is_null() || iovlen == 0 || src.is_empty() {
        return 0;
    }

    let mut copied = 0usize;
    for i in 0..iovlen {
        let ent = unsafe { &mut *iov.add(i) };
        if ent.iov_len == 0 || ent.iov_base.is_null() {
            continue;
        }
        let take = core::cmp::min(ent.iov_len, src.len().saturating_sub(copied));
        if take == 0 {
            break;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr().add(copied), ent.iov_base, take);
        }
        copied += take;
        if copied == src.len() {
            break;
        }
    }
    copied
}

unsafe fn extract_scm_rights(msg: *const MsgHdr, fds: &mut [i32; 4]) -> usize {
    if msg.is_null() {
        return 0;
    }

    let hdr = unsafe { &*msg };
    if hdr.msg_control.is_null() || (hdr.msg_controllen as usize) < cmsg_header_len() {
        return 0;
    }

    let cmsg = unsafe { core::ptr::read_unaligned(hdr.msg_control as *const CmsgHdr) };
    if !is_sol_socket_level(cmsg.cmsg_level) || cmsg.cmsg_type != trona::consts::SCM_RIGHTS {
        return 0;
    }

    let header_len = cmsg_header_len();
    let total_len = cmsg.cmsg_len as usize;
    if total_len < header_len {
        return 0;
    }
    let payload_len = core::cmp::min(
        total_len - header_len,
        (hdr.msg_controllen as usize) - header_len,
    );
    let count = core::cmp::min(payload_len / core::mem::size_of::<i32>(), fds.len());
    if count == 0 {
        return 0;
    }

    let src = unsafe { hdr.msg_control.add(header_len) as *const i32 };
    for (idx, dst) in fds.iter_mut().take(count).enumerate() {
        *dst = unsafe { core::ptr::read_unaligned(src.add(idx)) };
    }
    count
}

unsafe fn store_scm_rights(msg: *mut MsgHdr, fds: &[i32]) {
    if msg.is_null() {
        return;
    }

    let hdr = unsafe { &mut *msg };
    hdr.msg_flags = 0;
    if hdr.msg_control.is_null()
        || (hdr.msg_controllen as usize) < cmsg_header_len()
        || fds.is_empty()
    {
        hdr.msg_controllen = 0;
        return;
    }

    let header_len = cmsg_header_len();
    let payload_len = core::cmp::min(
        fds.len() * core::mem::size_of::<i32>(),
        (hdr.msg_controllen as usize) - header_len,
    );
    let fd_count = payload_len / core::mem::size_of::<i32>();
    if fd_count == 0 {
        hdr.msg_controllen = 0;
        return;
    }

    let cmsg = CmsgHdr {
        cmsg_len: (header_len + fd_count * core::mem::size_of::<i32>()) as u32,
        cmsg_level: cmsg_sol_socket_level(),
        cmsg_type: trona::consts::SCM_RIGHTS,
    };
    unsafe {
        core::ptr::write_unaligned(hdr.msg_control as *mut CmsgHdr, cmsg);
    }

    let dst = unsafe { hdr.msg_control.add(header_len) as *mut i32 };
    for (idx, fd) in fds.iter().take(fd_count).enumerate() {
        unsafe {
            core::ptr::write_unaligned(dst.add(idx), *fd);
        }
    }
    hdr.msg_controllen = (header_len + fd_count * core::mem::size_of::<i32>()) as u32;
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
        trona_posix::dns::parse_ipv4_numeric(core::slice::from_raw_parts(s, len))
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
            let dns = match trona_posix::dns::dns_resolve_multi_result(hostname) {
                Ok(dns) => dns,
                Err(label) => return dns_label_to_eai(label),
            };
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
                let rlen = trona_posix::dns::dns_reverse_lookup(ip, host, (hostlen - 1) as usize);
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
        let ret = trona_posix::posix_poll(fds as *mut trona_posix::PollFd, nfds, timeout);
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
        let ret = trona_posix::posix_epoll_create();
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
        let ret = trona_posix::posix_epoll_create();
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
        let ret = trona_posix::posix_epoll_ctl(epfd, op, fd, events, data);
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
        // EpollEvent layout matches trona::types::EpollEvent (events: u32, data: u64)
        // but we have a padding field. Use salty EpollEvent directly via cast.
        let ret = trona_posix::posix_epoll_wait(
            epfd,
            events as *mut trona::types::EpollEvent,
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

// ---------------------------------------------------------------------------
// gethostbyname / getservbyname — legacy BSD name resolution
// ---------------------------------------------------------------------------

/// Static hostent storage for gethostbyname (single-threaded, non-reentrant).
static mut HOSTENT_NAME: [u8; 256] = [0u8; 256];
static mut HOSTENT_ADDR: [u8; 4] = [0u8; 4];
static mut HOSTENT_ADDR_LIST: [*mut u8; 2] = [core::ptr::null_mut(); 2];
static mut HOSTENT_ALIASES: [*mut u8; 1] = [core::ptr::null_mut()];

#[repr(C)]
pub struct Hostent {
    pub h_name: *mut u8,
    pub h_aliases: *mut *mut u8,
    pub h_addrtype: i32,
    pub h_length: i32,
    pub h_addr_list: *mut *mut u8,
}

static mut HOSTENT: Hostent = Hostent {
    h_name: core::ptr::null_mut(),
    h_aliases: core::ptr::null_mut(),
    h_addrtype: 0,
    h_length: 0,
    h_addr_list: core::ptr::null_mut(),
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethostbyname(name: *const u8) -> *mut Hostent {
    if name.is_null() {
        unsafe { crate::inet::h_errno = HOST_NOT_FOUND; }
        return core::ptr::null_mut();
    }
    unsafe {
        let ip = match trona_posix::dns::posix_gethostbyname_result(name) {
            Ok(ip) => {
                crate::inet::h_errno = 0;
                ip
            }
            Err(label) => {
                set_h_errno_from_dns_label(label);
                return core::ptr::null_mut();
            }
        };

        // Fill static storage
        let bytes = ip.to_be_bytes();
        let addr = &raw mut HOSTENT_ADDR as *mut u8;
        *addr = bytes[0];
        *addr.add(1) = bytes[1];
        *addr.add(2) = bytes[2];
        *addr.add(3) = bytes[3];

        let addr_list = &raw mut HOSTENT_ADDR_LIST;
        (*addr_list)[0] = &raw mut HOSTENT_ADDR as *mut u8;
        (*addr_list)[1] = core::ptr::null_mut();

        // Copy name
        let hname = &raw mut HOSTENT_NAME as *mut u8;
        let mut i = 0usize;
        while *name.add(i) != 0 && i < 255 {
            *hname.add(i) = *name.add(i);
            i += 1;
        }
        *hname.add(i) = 0;

        let he = &raw mut HOSTENT;
        (*he).h_name = hname;
        (*he).h_aliases = (&raw mut HOSTENT_ALIASES) as *mut *mut u8;
        (*he).h_addrtype = 2; // AF_INET
        (*he).h_length = 4;
        (*he).h_addr_list = (*addr_list).as_mut_ptr();

        he
    }
}

#[repr(C)]
pub struct Servent {
    pub s_name: *mut u8,
    pub s_aliases: *mut *mut u8,
    pub s_port: i32,
    pub s_proto: *mut u8,
}

/// getservbyname stub — returns NULL (no /etc/services on SaltyOS).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getservbyname(
    _name: *const u8,
    _proto: *const u8,
) -> *mut Servent {
    core::ptr::null_mut()
}
