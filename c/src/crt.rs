//! Process runtime startup
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! Entry point for all dynamically-linked programs on SaltyOS (C and Rust).
//! The dynamic linker (`rtld`) calls `_start` (in `crt_start.S`), which
//! calls `__libc_start_main` here.
//!
//! Initialization is split into two layers:
//!
//! **Common runtime init** (`common_init`) — required by all processes:
//! 1. Set up IPC buffer at fixed vaddr and initialize IPC context
//! 2. Initialize per-process slot allocator (preferring RTLD-exported pool)
//! 3. Initialize `posix_mm` with mmsrv endpoint from auxv (0 = no pager)
//! 4. Initialize TLS for the main thread
//!
//! **C/POSIX layer** (remainder of `__libc_start_main`):
//! 5. Parse argc/argv/envp, initialize `environ`
//! 6. Set program name from `argv[0]` for BSD `err(3)` functions
//! 7. Probe/open stdio fds (fd 0/1/2 → `/dev/console` if absent)
//! 8. Initialize FreeBSD rune locale tables for `ctype.h` compatibility
//! 9. Run executable preinit/init arrays in ELF priority order
//! 10. Call `main(argc, argv, envp)`, then `exit()`
//!
//! Custom auxv tags used by SaltyOS:
//! - `0x1007` (`AT_TRONA_SLOT_BASE`): slot allocator pool base
//! - `0x1008` (`AT_TRONA_SLOT_COUNT`): slot allocator pool size
//! - `0x1009` (`AT_TRONA_EXPAND_EP`): procmgr endpoint used for CSpace expansion
//! - `0x100B` (`AT_TRONA_MM_EP`): mmsrv endpoint cap slot

use crate::env;

const CAP_SELF_TCB: u64 = 0;

/// Maximum number of atexit handlers
const ATEXIT_MAX: usize = 32;

static mut ATEXIT_FUNCS: [Option<unsafe extern "C" fn()>; ATEXIT_MAX] = [None; ATEXIT_MAX];
static mut ATEXIT_COUNT: usize = 0;

/// Mutex protecting ATEXIT_FUNCS/ATEXIT_COUNT and CXA_ATEXIT_FUNCS/CXA_ATEXIT_COUNT.
static ATEXIT_LOCK: trona_posix::sync::Mutex = trona_posix::sync::Mutex::new();

/// Saved pointer to the auxv on the initial stack.
/// Set once by `__libc_start_main`; valid for the process lifetime.
pub(crate) static mut SAVED_AUXV: *const u64 = core::ptr::null();

// --- C++ ABI support ---

#[repr(C)]
struct CxaAtexitEntry {
    destructor: unsafe extern "C" fn(*mut core::ffi::c_void),
    arg: *mut core::ffi::c_void,
    dso_handle: *mut core::ffi::c_void,
}

const CXA_ATEXIT_MAX: usize = 128;
// Use MaybeUninit to avoid needing Default impl
static mut CXA_ATEXIT_FUNCS: [core::mem::MaybeUninit<CxaAtexitEntry>; CXA_ATEXIT_MAX] =
    // SAFETY: An array of MaybeUninit does not require initialization
    unsafe { core::mem::MaybeUninit::uninit().assume_init() };
static mut CXA_ATEXIT_COUNT: usize = 0;

#[repr(transparent)]
struct SyncPtr(*const core::ffi::c_void);
// SAFETY: __dso_handle is a read-only sentinel value (always null for the
// main executable). It is never written to after initialization.
unsafe impl Sync for SyncPtr {}

#[unsafe(no_mangle)]
#[used]
static __dso_handle: SyncPtr = SyncPtr(core::ptr::null());

/// Saved executable .fini_array bounds provided by `_start`.
///
/// These must come from the main executable, not from `libc.so` itself.
static mut SAVED_FINI_ARRAY_START: *const unsafe extern "C" fn() = core::ptr::null();
static mut SAVED_FINI_ARRAY_END: *const unsafe extern "C" fn() = core::ptr::null();

/// Called from _start (crt_start.S). Receives a pointer to main() and the
/// initial stack pointer. Parses the stack to extract argc, argv, envp, and
/// auxv. Initializes the C runtime, then calls main().
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __libc_start_main(
    main_fn: unsafe extern "C" fn(i32, *const *const u8, *const *const u8) -> i32,
    stack_ptr: *const u64,
    preinit_array_start: *const unsafe extern "C" fn(),
    preinit_array_end: *const unsafe extern "C" fn(),
    init_array_start: *const unsafe extern "C" fn(),
    init_array_end: *const unsafe extern "C" fn(),
    fini_array_start: *const unsafe extern "C" fn(),
    fini_array_end: *const unsafe extern "C" fn(),
) -> ! {
    unsafe {
        // Stack layout: argc, argv[0], argv[1], ..., NULL, envp[0], ..., NULL, auxv...
        let argc = *stack_ptr as i32;
        let argv = stack_ptr.add(1) as *const *const u8;

        // Find envp: skip past argv (argc pointers + NULL terminator)
        let envp = argv.add(argc as usize + 1) as *const *const u8;

        // Initialize environ
        env::init_environ(envp);

        // Save auxv pointer for elf_aux_info / getauxval (walk past envp null)
        {
            let mut ep = envp;
            while !(*ep).is_null() {
                ep = ep.add(1);
            }
            core::ptr::addr_of_mut!(SAVED_AUXV).write(ep.add(1) as *const u64);
        }

        // Common runtime init: IPC buffer, slot allocator, mmsrv, TLS.
        // Shared by all dynamically-linked processes (C and Rust alike).
        common_init(stack_ptr);

        // Set program name from argv[0] for BSD err(3) functions
        if argc > 0 && !(*argv).is_null() {
            crate::compat::freebsd::bsd_misc::setprogname(*argv);
        }

        // Probe fd 0: if already open (inherited from exec), skip /dev/console.
        // dup(0) succeeds if fd 0 exists (exec'd process), fails if empty (fresh spawn).
        let probe = trona_posix::posix_dup(0);
        if probe >= 0 {
            // fd 0 exists — inherited from exec caller (getty→bash).
            // Close the test fd and leave fd 0/1/2 as-is.
            trona_posix::posix_close(probe);
        } else {
            // fd 0 doesn't exist — fresh spawn. Open /dev/console.
            let fd0 = trona_posix::posix_open(b"/dev/console\0".as_ptr(), 2, 0); // O_RDWR
            if fd0 >= 0 {
                trona_posix::posix_dup(fd0); // fd 1
                trona_posix::posix_dup(fd0); // fd 2
            }
        }

        // Initialize stdio pointers (stdin/stdout/stderr) so that programs
        // using fprintf(stderr, ...) etc. get valid FILE* from the GOT.
        crate::stdio::ensure_stdio_init();

        // Initialize FreeBSD locale/rune compatibility before user
        // constructors can call into ctype/locale-sensitive helpers.
        crate::compat::freebsd::rune::init_rune_locale();

        core::ptr::addr_of_mut!(SAVED_FINI_ARRAY_START).write(fini_array_start);
        core::ptr::addr_of_mut!(SAVED_FINI_ARRAY_END).write(fini_array_end);

        // Run ELF preinit/init arrays in the same order expected by other ELF
        // environments: preinit first, then init in ascending priority order.
        call_preinit_array(preinit_array_start, preinit_array_end);
        call_init_array(init_array_start, init_array_end);

        // Call main
        let ret = main_fn(argc, argv, envp);

        // Exit
        exit(ret);
    }
}

/// Common runtime initialization for all dynamically-linked SaltyOS processes.
/// Sets up the IPC buffer, per-process slot allocator, mmsrv client, and TLS.
/// Called by `__libc_start_main` before any C/POSIX-specific setup.
unsafe fn common_init(stack_ptr: *const u64) {
    unsafe {
        init_ipc_from_auxv(stack_ptr);
        init_mm_from_auxv(stack_ptr);
        trona_posix::tls::init_main_thread_tls();
    }
}

/// Parse auxv entries from the stack
unsafe fn init_ipc_from_auxv(stack_ptr: *const u64) {
    unsafe {
        let argc = *stack_ptr as usize;
        let argv = stack_ptr.add(1);

        // Skip argv
        let mut p = argv.add(argc + 1); // past NULL terminator

        // Skip envp
        while !(*p as *const u8).is_null() {
            p = p.add(1);
        }
        let _auxv = p.add(1); // past envp NULL terminator — points to auxv pairs
        let ipc_buf_vaddr: u64 = 0x0000_0000_0020_0000;
        trona::invoke::tcb_set_ipc_buffer(CAP_SELF_TCB, ipc_buf_vaddr);
        trona::ipc::ipc_context_init(
            &raw mut trona::__trona_ipc_ctx,
            ipc_buf_vaddr as *mut trona::types::IpcBuffer,
        );
    }
}

/// Initialize the per-process slot allocator and POSIX memory manager from auxv.
///
/// Parses SaltyOS-specific auxiliary vector entries (`AT_TRONA_*`) to discover
/// the slot allocator pool and the mmsrv endpoint capability. The RTLD may
/// have already consumed some slots, so its exported values take precedence
/// over raw auxv. The mmsrv endpoint is provided via `AT_TRONA_MM_EP` by
/// the spawner (init or procmgr). If absent, defaults to 0 (no pager).
unsafe fn init_mm_from_auxv(stack_ptr: *const u64) {
    unsafe {
        let argc = *stack_ptr as usize;
        let argv = stack_ptr.add(1);
        let mut p = argv.add(argc + 1);

        while !(*p as *const u8).is_null() {
            p = p.add(1);
        }
        p = p.add(1);

        // Parse auxv
        let mut slot_base: u64 = 0;
        let mut slot_count: u64 = 0;
        let mut expand_ep: u64 = 0;
        let mut mm_ep: u64 = 0; // default: no mmsrv (overridden by AT_TRONA_MM_EP)
        let mut sc_cap: u64 = 0;

        loop {
            let tag = *p;
            let val = *p.add(1);
            if tag == 0 {
                break; // AT_NULL
            }
            match tag {
                0x1007 => slot_base = val,   // AT_TRONA_SLOT_BASE
                0x1008 => slot_count = val,  // AT_TRONA_SLOT_COUNT
                0x1009 => expand_ep = val,   // AT_TRONA_EXPAND_EP
                0x100B => mm_ep = val,       // AT_TRONA_MM_EP
                0x100E => sc_cap = val,      // AT_TRONA_SC_CAP
                _ => {}
            }
            p = p.add(2);
        }

        // Prefer RTLD-exported values because RTLD advances slot pool past
        // the slots consumed while loading shared libraries, and parses
        // AT_TRONA_SC_CAP itself.
        let rtld_base = *(&raw const trona::__trona_slot_base);
        let rtld_count = *(&raw const trona::__trona_slot_count);
        if rtld_base != 0 && rtld_count != 0 {
            slot_base = rtld_base;
            slot_count = rtld_count;
        }

        let rtld_sc_cap = *(&raw const trona::__trona_sc_cap);
        if rtld_sc_cap != 0 {
            sc_cap = rtld_sc_cap;
        }
        // Write to substrate global so TLS init can pick it up
        *(&raw mut trona::__trona_sc_cap) = sc_cap;

        let cspace_ntfn = *(&raw const trona::__trona_cspace_ntfn);

        // Initialize per-process slot allocator
        if slot_base != 0 {
            trona::slot_alloc::slot_alloc_init(slot_base, slot_count, cspace_ntfn);
            if expand_ep != 0 {
                trona::slot_alloc::slot_alloc_set_procmgr_ep(expand_ep);
            }
        }

        // Initialize posix_mm with the pager endpoint discovered from auxv
        trona_posix::mm::posix_mm_init(mm_ep);
    }
}

/// Register a function to be called at exit
#[unsafe(no_mangle)]
pub unsafe extern "C" fn atexit(func: unsafe extern "C" fn()) -> i32 {
    ATEXIT_LOCK.lock();
    let result = unsafe {
        if ATEXIT_COUNT >= ATEXIT_MAX {
            -1
        } else {
            ATEXIT_FUNCS[ATEXIT_COUNT] = Some(func);
            ATEXIT_COUNT += 1;
            0
        }
    };
    ATEXIT_LOCK.unlock();
    result
}

/// Exit the program, calling atexit handlers in reverse order
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit(status: i32) -> ! {
    unsafe {
        // Call atexit handlers in reverse order (under lock to prevent
        // concurrent registration during teardown)
        ATEXIT_LOCK.lock();
        while ATEXIT_COUNT > 0 {
            ATEXIT_COUNT -= 1;
            if let Some(func) = ATEXIT_FUNCS[ATEXIT_COUNT] {
                // Release lock while calling handler (handler may call atexit)
                ATEXIT_LOCK.unlock();
                func();
                ATEXIT_LOCK.lock();
            }
        }
        ATEXIT_LOCK.unlock();

        // Call C++ destructors registered via __cxa_atexit
        __cxa_finalize(core::ptr::null_mut());

        // Call .fini_array destructors in reverse order
        let fini_start = core::ptr::addr_of!(SAVED_FINI_ARRAY_START).read();
        let fini_end = core::ptr::addr_of!(SAVED_FINI_ARRAY_END).read();
        call_fini_array(fini_start, fini_end);

        // Flush stdio
        crate::stdio::fflush_all();

        // Call _exit
        _exit(status);
    }
}

/// Register a C++ destructor to be called at exit or DSO unload
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cxa_atexit(
    destructor: unsafe extern "C" fn(*mut core::ffi::c_void),
    arg: *mut core::ffi::c_void,
    dso_handle: *mut core::ffi::c_void,
) -> i32 {
    ATEXIT_LOCK.lock();
    let result = unsafe {
        if CXA_ATEXIT_COUNT >= CXA_ATEXIT_MAX {
            -1
        } else {
            CXA_ATEXIT_FUNCS[CXA_ATEXIT_COUNT].write(CxaAtexitEntry {
                destructor,
                arg,
                dso_handle,
            });
            CXA_ATEXIT_COUNT += 1;
            0
        }
    };
    ATEXIT_LOCK.unlock();
    result
}

/// Register a C++ thread-local destructor.
/// Delegates to __cxa_atexit for now; full per-thread cleanup requires TLS destructor
/// infrastructure that SaltyOS does not yet implement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cxa_thread_atexit(
    dtor: unsafe extern "C" fn(*mut core::ffi::c_void),
    obj: *mut core::ffi::c_void,
    dso_handle: *mut core::ffi::c_void,
) -> i32 {
    unsafe { __cxa_atexit(dtor, obj, dso_handle) }
}

/// Call C++ destructors registered via __cxa_atexit.
/// If dso_handle is null, calls all destructors. Otherwise only those
/// matching the given DSO handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __cxa_finalize(dso_handle: *mut core::ffi::c_void) {
    ATEXIT_LOCK.lock();
    unsafe {
        // Call in reverse order
        let mut i = CXA_ATEXIT_COUNT;
        while i > 0 {
            i -= 1;
            let entry = CXA_ATEXIT_FUNCS[i].assume_init_ref();
            if dso_handle.is_null() || entry.dso_handle == dso_handle {
                // Release lock while calling destructor (it may register more)
                ATEXIT_LOCK.unlock();
                (entry.destructor)(entry.arg);
                ATEXIT_LOCK.lock();
            }
        }
        if dso_handle.is_null() {
            CXA_ATEXIT_COUNT = 0;
        }
    }
    ATEXIT_LOCK.unlock();
}

/// Call all function pointers in the .preinit_array section (forward order)
unsafe fn call_preinit_array(
    mut p: *const unsafe extern "C" fn(),
    end: *const unsafe extern "C" fn(),
) {
    unsafe {
        while p < end {
            let func = core::ptr::read(p);
            func();
            p = p.add(1);
        }
    }
}

/// Call all function pointers in the .init_array section (forward order)
unsafe fn call_init_array(mut p: *const unsafe extern "C" fn(), end: *const unsafe extern "C" fn()) {
    unsafe {
        while p < end {
            let func = core::ptr::read(p);
            func();
            p = p.add(1);
        }
    }
}

/// Call all function pointers in the .fini_array section (reverse order)
unsafe fn call_fini_array(
    start: *const unsafe extern "C" fn(),
    mut p: *const unsafe extern "C" fn(),
) {
    unsafe {
        while p > start {
            p = p.sub(1);
            let func = core::ptr::read(p);
            func();
        }
    }
}

/// Immediate exit without cleanup
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _exit(status: i32) -> ! {
    unsafe {
        trona_posix::posix_exit(status);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _Exit(status: i32) -> ! {
    unsafe { _exit(status) }
}
