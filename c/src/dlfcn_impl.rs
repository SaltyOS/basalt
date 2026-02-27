//! Dynamic linking stubs
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Stub implementations of `dlopen`, `dlsym`, `dlclose`, `dlerror`, and
//! `dl_iterate_phdr`. These return ENOSYS / NULL for now. When rtld gains
//! a proper `dlopen` API, these stubs can be replaced with real calls.

use crate::errno;

static mut DLERROR_MSG: [u8; 64] = [0; 64];

/// Copy a static message into the dlerror buffer.
unsafe fn set_dlerror(msg: &[u8]) {
    unsafe {
        let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
        let len = if msg.len() < 63 { msg.len() } else { 63 };
        for i in 0..len {
            *buf.add(i) = msg[i];
        }
        *buf.add(len) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlopen(_filename: *const u8, _flags: i32) -> *mut u8 {
    unsafe {
        set_dlerror(b"dlopen: not supported on SaltyOS");
    }
    errno::set_errno(errno::ENOSYS);
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(_handle: *mut u8, _symbol: *const u8) -> *mut u8 {
    unsafe {
        set_dlerror(b"dlsym: not supported on SaltyOS");
    }
    errno::set_errno(errno::ENOSYS);
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlclose(_handle: *mut u8) -> i32 {
    unsafe {
        set_dlerror(b"dlclose: not supported on SaltyOS");
    }
    errno::set_errno(errno::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlerror() -> *mut u8 {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    unsafe {
        if *buf == 0 {
            return core::ptr::null_mut();
        }
    }
    buf
}

/// Iterate over loaded shared objects.
///
/// Stub that calls the callback once for the main executable (addr=0,
/// name="", no program headers). Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dl_iterate_phdr(
    callback: Option<unsafe extern "C" fn(*mut DlPhdrInfo, usize, *mut u8) -> i32>,
    data: *mut u8,
) -> i32 {
    let Some(cb) = callback else {
        return 0;
    };

    unsafe {
        let mut info = DlPhdrInfo {
            dlpi_addr: 0,
            dlpi_name: b"\0".as_ptr(),
            dlpi_phdr: core::ptr::null(),
            dlpi_phnum: 0,
        };
        let ret = cb(
            &mut info as *mut DlPhdrInfo,
            core::mem::size_of::<DlPhdrInfo>(),
            data,
        );
        if ret != 0 {
            return ret;
        }
    }
    0
}

#[repr(C)]
pub struct DlPhdrInfo {
    pub dlpi_addr: usize,
    pub dlpi_name: *const u8,
    pub dlpi_phdr: *const u8,
    pub dlpi_phnum: u16,
}
