//! Environment variable management
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Stores up to 128 environment variables in a static array. The `environ`
//! pointer is exported for C code. Variables are initialized from the stack-
//! provided `envp` during CRT startup (`init_environ`).

const MAX_ENV: usize = 128;

/// Static storage for environment pointers (null-terminated array)
static mut ENV_PTRS: [*const u8; MAX_ENV + 1] = [core::ptr::null(); MAX_ENV + 1];
static mut ENV_COUNT: usize = 0;

/// The global environ pointer
#[unsafe(no_mangle)]
pub static mut environ: *mut *const u8 = core::ptr::null_mut();

/// RWLock protecting ENV_PTRS/ENV_COUNT. Readers (getenv) can run
/// concurrently; writers (setenv/unsetenv/putenv/clearenv) are exclusive.
static ENV_LOCK: trona::sync::RWLock = trona::sync::RWLock::new();

/// Initialize environ from the stack-provided envp
pub unsafe fn init_environ(envp: *const *const u8) {
    unsafe {
        ENV_COUNT = 0;
        if !envp.is_null() {
            let mut i = 0;
            while !(*envp.add(i)).is_null() && ENV_COUNT < MAX_ENV {
                ENV_PTRS[ENV_COUNT] = *envp.add(i);
                ENV_COUNT += 1;
                i += 1;
            }
        }
        ENV_PTRS[ENV_COUNT] = core::ptr::null();
        environ = (&raw mut ENV_PTRS) as *mut *const u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getenv(name: *const u8) -> *const u8 {
    if name.is_null() {
        return core::ptr::null();
    }
    ENV_LOCK.read_lock();
    let result = unsafe {
        let name_len = crate::string::strlen(name);
        let mut found: *const u8 = core::ptr::null();
        for i in 0..ENV_COUNT {
            let entry = ENV_PTRS[i];
            if entry.is_null() {
                continue;
            }
            // Check if entry starts with name=
            let mut match_ok = true;
            for j in 0..name_len {
                if *entry.add(j) != *name.add(j) {
                    match_ok = false;
                    break;
                }
            }
            if match_ok && *entry.add(name_len) == b'=' {
                found = entry.add(name_len + 1);
                break;
            }
        }
        found
    };
    ENV_LOCK.read_unlock();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setenv(name: *const u8, value: *const u8, overwrite: i32) -> i32 {
    if name.is_null() {
        return -1;
    }
    ENV_LOCK.write_lock();
    let result = unsafe {
        let name_len = crate::string::strlen(name);
        let value_len = crate::string::strlen(value);

        // Check if already exists
        let mut found = false;
        for i in 0..ENV_COUNT {
            let entry = ENV_PTRS[i];
            if entry.is_null() {
                continue;
            }
            let mut match_ok = true;
            for j in 0..name_len {
                if *entry.add(j) != *name.add(j) {
                    match_ok = false;
                    break;
                }
            }
            if match_ok && *entry.add(name_len) == b'=' {
                if overwrite == 0 {
                    found = true;
                    break;
                }
                // Allocate new "name=value" string
                let new_entry = crate::malloc::malloc(name_len + 1 + value_len + 1);
                if new_entry.is_null() {
                    ENV_LOCK.write_unlock();
                    return -1;
                }
                core::ptr::copy_nonoverlapping(name, new_entry, name_len);
                *new_entry.add(name_len) = b'=';
                core::ptr::copy_nonoverlapping(value, new_entry.add(name_len + 1), value_len);
                *new_entry.add(name_len + 1 + value_len) = 0;
                ENV_PTRS[i] = new_entry;
                found = true;
                break;
            }
        }

        if found {
            0
        } else {
            // Add new entry
            if ENV_COUNT >= MAX_ENV {
                -1
            } else {
                let new_entry = crate::malloc::malloc(name_len + 1 + value_len + 1);
                if new_entry.is_null() {
                    -1
                } else {
                    core::ptr::copy_nonoverlapping(name, new_entry, name_len);
                    *new_entry.add(name_len) = b'=';
                    core::ptr::copy_nonoverlapping(value, new_entry.add(name_len + 1), value_len);
                    *new_entry.add(name_len + 1 + value_len) = 0;
                    ENV_PTRS[ENV_COUNT] = new_entry;
                    ENV_COUNT += 1;
                    ENV_PTRS[ENV_COUNT] = core::ptr::null();
                    0
                }
            }
        }
    };
    ENV_LOCK.write_unlock();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsetenv(name: *const u8) -> i32 {
    if name.is_null() {
        return -1;
    }
    ENV_LOCK.write_lock();
    unsafe {
        let name_len = crate::string::strlen(name);
        let mut i = 0;
        while i < ENV_COUNT {
            let entry = ENV_PTRS[i];
            if entry.is_null() {
                i += 1;
                continue;
            }
            let mut match_ok = true;
            for j in 0..name_len {
                if *entry.add(j) != *name.add(j) {
                    match_ok = false;
                    break;
                }
            }
            if match_ok && *entry.add(name_len) == b'=' {
                // Remove by shifting
                let mut j = i;
                while j < ENV_COUNT - 1 {
                    ENV_PTRS[j] = ENV_PTRS[j + 1];
                    j += 1;
                }
                ENV_COUNT -= 1;
                ENV_PTRS[ENV_COUNT] = core::ptr::null();
                continue; // don't increment, check shifted entry
            }
            i += 1;
        }
    }
    ENV_LOCK.write_unlock();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn putenv(string: *mut u8) -> i32 {
    if string.is_null() {
        return -1;
    }
    ENV_LOCK.write_lock();
    let result = unsafe {
        // Find '=' in string
        let mut eq_pos = 0;
        while *string.add(eq_pos) != 0 && *string.add(eq_pos) != b'=' {
            eq_pos += 1;
        }
        if *string.add(eq_pos) != b'=' {
            -1
        } else {
            // Check if exists, replace
            let mut found = false;
            for i in 0..ENV_COUNT {
                let entry = ENV_PTRS[i];
                if entry.is_null() {
                    continue;
                }
                let mut match_ok = true;
                for j in 0..eq_pos {
                    if *entry.add(j) != *string.add(j) {
                        match_ok = false;
                        break;
                    }
                }
                if match_ok && *entry.add(eq_pos) == b'=' {
                    ENV_PTRS[i] = string;
                    found = true;
                    break;
                }
            }

            if found {
                0
            } else if ENV_COUNT >= MAX_ENV {
                -1
            } else {
                ENV_PTRS[ENV_COUNT] = string;
                ENV_COUNT += 1;
                ENV_PTRS[ENV_COUNT] = core::ptr::null();
                0
            }
        }
    };
    ENV_LOCK.write_unlock();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn clearenv() -> i32 {
    ENV_LOCK.write_lock();
    unsafe {
        ENV_COUNT = 0;
        ENV_PTRS[0] = core::ptr::null();
    }
    ENV_LOCK.write_unlock();
    0
}
