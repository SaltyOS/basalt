//! FreeBSD _umtx_op compatibility — maps to SaltyOS futex syscall
//! SPDX-License-Identifier: GPL-2.0-only

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

const UMTX_OP_WAIT: i32 = 2;
const UMTX_OP_WAKE: i32 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _umtx_op(
    obj: *mut core::ffi::c_void,
    op: i32,
    val: u64,
    _uaddr: *mut core::ffi::c_void,
    uaddr2: *mut core::ffi::c_void,
) -> i32 {
    unsafe {
        match op {
            UMTX_OP_WAIT => {
                let addr = obj as *const u32;
                let expected = val as u32;
                if uaddr2.is_null() {
                    salty::syscall::futex_wait(addr, expected);
                } else {
                    let ts = &*(uaddr2 as *const Timespec);
                    let timeout_ns =
                        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64);
                    salty::syscall::futex_wait_timeout(addr, expected, timeout_ns);
                }
                0
            }
            UMTX_OP_WAKE => {
                let addr = obj as *const u32;
                let count = val as u32;
                salty::syscall::futex_wake(addr, count);
                0
            }
            _ => {
                crate::errno::set_errno(crate::errno::EINVAL);
                -1
            }
        }
    }
}
