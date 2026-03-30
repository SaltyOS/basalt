//! Network interface enumeration helpers.
//! SPDX-License-Identifier: GPL-2.0-only

use crate::errno;
use crate::malloc;

const IFACE_NAME: &[u8] = b"eth0\0";
const IF_INDEX: u32 = 1;
const AF_INET: u16 = 2;
const IFF_UP: u32 = 0x1;
const IFF_BROADCAST: u32 = 0x2;
const IFF_RUNNING: u32 = 0x40;
const IFF_MULTICAST: u32 = 0x1000;

#[repr(C)]
struct SockAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

#[repr(C)]
struct InAddr {
    s_addr: u32,
}

#[repr(C)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: InAddr,
    sin_zero: [u8; 8],
}

#[repr(C)]
pub struct IfNameIndex {
    if_index: u32,
    if_name: *mut u8,
}

#[repr(C)]
union IfaIfu {
    ifu_broadaddr: *mut SockAddr,
    ifu_dstaddr: *mut SockAddr,
}

#[repr(C)]
pub struct IfAddrs {
    ifa_next: *mut IfAddrs,
    ifa_name: *mut u8,
    ifa_flags: u32,
    ifa_addr: *mut SockAddr,
    ifa_netmask: *mut SockAddr,
    ifa_ifu: IfaIfu,
    ifa_data: *mut u8,
}

fn iface_flags(ip: u32) -> u32 {
    if ip != 0 {
        IFF_UP | IFF_BROADCAST | IFF_RUNNING | IFF_MULTICAST
    } else {
        0
    }
}

fn iface_name_matches(ptr: *const u8) -> bool {
    if ptr.is_null() {
        return false;
    }
    // SAFETY: Caller passed a C string pointer.
    unsafe {
        let mut i = 0usize;
        while i < IFACE_NAME.len() {
            if *ptr.add(i) != IFACE_NAME[i] {
                return false;
            }
            if IFACE_NAME[i] == 0 {
                return true;
            }
            i += 1;
        }
    }
    false
}

unsafe fn write_sockaddr_in(dst: *mut SockAddrIn, ip: u32) {
    unsafe {
        (*dst).sin_family = AF_INET;
        (*dst).sin_port = 0;
        (*dst).sin_addr.s_addr = ip.to_be();
        (*dst).sin_zero = [0; 8];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn if_nameindex() -> *mut IfNameIndex {
    let total = 2 * core::mem::size_of::<IfNameIndex>() + IFACE_NAME.len();
    let base = unsafe { malloc::malloc(total) };
    if base.is_null() {
        errno::set_errno(errno::ENOMEM);
        return core::ptr::null_mut();
    }

    // SAFETY: `base` points to `total` writable bytes just allocated.
    unsafe {
        let entries = base as *mut IfNameIndex;
        let name = base.add(2 * core::mem::size_of::<IfNameIndex>());
        core::ptr::copy_nonoverlapping(IFACE_NAME.as_ptr(), name, IFACE_NAME.len());

        (*entries).if_index = IF_INDEX;
        (*entries).if_name = name;
        (*entries.add(1)).if_index = 0;
        (*entries.add(1)).if_name = core::ptr::null_mut();
        entries
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freeif_nameindex(ptr: *mut IfNameIndex) {
    if !ptr.is_null() {
        unsafe { malloc::free(ptr as *mut u8) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn if_nametoindex(ifname: *const u8) -> u32 {
    if iface_name_matches(ifname) {
        IF_INDEX
    } else {
        errno::set_errno(errno::ENXIO);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn if_indextoname(ifindex: u32, ifname: *mut u8) -> *mut u8 {
    if ifindex != IF_INDEX || ifname.is_null() {
        errno::set_errno(if ifname.is_null() { errno::EFAULT } else { errno::ENXIO });
        return core::ptr::null_mut();
    }
    // SAFETY: Caller supplied a writable buffer.
    unsafe {
        core::ptr::copy_nonoverlapping(IFACE_NAME.as_ptr(), ifname, IFACE_NAME.len());
        ifname
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getifaddrs(ifap: *mut *mut IfAddrs) -> i32 {
    if ifap.is_null() {
        errno::set_errno(errno::EFAULT);
        return -1;
    }

    let mut cfg = [0u64; 9];
    let ret = unsafe { trona_posix::posix_net_get_config(&raw mut cfg) };
    if ret < 0 {
        errno::set_errno(-ret);
        return -1;
    }

    let total = core::mem::size_of::<IfAddrs>()
        + IFACE_NAME.len()
        + 3 * core::mem::size_of::<SockAddrIn>();
    let base = unsafe { malloc::calloc(1, total) };
    if base.is_null() {
        errno::set_errno(errno::ENOMEM);
        return -1;
    }

    let our_ip = cfg[1] as u32;
    let mask = cfg[2] as u32;
    let broadcast = if our_ip != 0 && mask != 0 {
        (our_ip & mask) | !mask
    } else {
        0
    };

    // SAFETY: `base` points to `total` zeroed writable bytes.
    unsafe {
        let ifa = base as *mut IfAddrs;
        let name = base.add(core::mem::size_of::<IfAddrs>());
        let addr = name.add(IFACE_NAME.len()) as *mut SockAddrIn;
        let netmask = addr.add(1);
        let broad = addr.add(2);

        core::ptr::copy_nonoverlapping(IFACE_NAME.as_ptr(), name, IFACE_NAME.len());
        write_sockaddr_in(addr, our_ip);
        write_sockaddr_in(netmask, mask);
        write_sockaddr_in(broad, broadcast);

        (*ifa).ifa_next = core::ptr::null_mut();
        (*ifa).ifa_name = name;
        (*ifa).ifa_flags = iface_flags(our_ip);
        (*ifa).ifa_addr = addr as *mut SockAddr;
        (*ifa).ifa_netmask = netmask as *mut SockAddr;
        (*ifa).ifa_ifu.ifu_broadaddr = broad as *mut SockAddr;
        (*ifa).ifa_data = core::ptr::null_mut();

        *ifap = ifa;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freeifaddrs(ifa: *mut IfAddrs) {
    if !ifa.is_null() {
        unsafe { malloc::free(ifa as *mut u8) };
    }
}
