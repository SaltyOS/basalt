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
//! SaltyOS-private startup state arrives through one versioned startup block
//! referenced by `AT_SALTYOS_STARTUP`. The substrate validates that block,
//! installs the startup cap-table into `__trona_cap_*`, and exposes the
//! derived CSpace / bootstrap metadata through `trona_runtime::runtime_*` helpers
//! before libc common init runs.

use crate::env;

const CAP_SELF_TCB: u64 = 0;

/// Maximum number of atexit handlers
const ATEXIT_MAX: usize = 32;

static mut ATEXIT_FUNCS: [Option<unsafe extern "C" fn()>; ATEXIT_MAX] = [None; ATEXIT_MAX];
static mut ATEXIT_COUNT: usize = 0;

/// Mutex protecting ATEXIT_FUNCS/ATEXIT_COUNT and CXA_ATEXIT_FUNCS/CXA_ATEXIT_COUNT.
static ATEXIT_LOCK: trona_runtime::thread::sync::Mutex = trona_runtime::thread::sync::Mutex::new();

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
            let auxv = ep.add(1) as *const u64;
            core::ptr::addr_of_mut!(SAVED_AUXV).write(auxv);
            trona_runtime::runtime_set_auxv(auxv);
        }

        // Common runtime init: IPC buffer, slot allocator, mmsrv, TLS.
        // Shared by all dynamically-linked processes (C and Rust alike).
        common_init(stack_ptr);

        // Set program name from argv[0] for BSD err(3) functions
        if argc > 0 && !(*argv).is_null() {
            crate::compat::freebsd::bsd_misc::setprogname(*argv);
        }

        // Initialize stdio pointers (stdin/stdout/stderr) so that programs
        // using fprintf(stderr, ...) etc. get valid FILE* from the GOT.
        //
        // CRT no longer issues the legacy `posix_dup(0)` probe and
        // `posix_open("/dev/console")` here: every dynamically-linked
        // process used to pay a synchronous VFS round-trip before
        // `main()` for this stdio bootstrap, which made any service
        // spawned while VFS was in its between-`signal_ready`-and-
        // `run_owner_loop` window block indefinitely. Stdio slots
        // for `STDIO_MODE_PTY` children arrive pre-populated via
        // init's spawn-time `POSIX_TTYSRV_PTY_ALLOC` + VFS dup —
        // the `preinstalled_slot_bitmap` in the startup block records
        // which client-state slots are already valid. Children
        // spawned under `STDIO_MODE_CONSOLE` (the default) see an
        // all-zero bitmap and fall through to a lazy `/dev/console`
        // bind on first stdio use; services whose spawner has no
        // console at all (init-spawned pre-VFS services) must not
        // touch stdio at startup and should log through
        // `trona_runtime::uinfo!`/`uwarn!`/`uerror!` on the direct console
        // capability instead.
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

unsafe fn resolve_runtime_mm_state(_auxv: *const u64) -> u64 {
    unsafe { *(&raw const trona_runtime::__trona_sc_cap) }
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
        let ipc_buf_vaddr =
            trona_runtime::runtime_get_ipc_buffer_vaddr().unwrap_or(0x0000_0000_0020_0000);
        trona_kernel::invoke::tcb_set_ipc_buffer(
            trona_kernel::core_types::CapRef::flat(CAP_SELF_TCB),
            ipc_buf_vaddr,
        );
        trona_kernel::ipc::ipc_context_init(
            &raw mut trona_runtime::__trona_ipc_ctx,
            ipc_buf_vaddr as *mut uapi::kernite_ipc_buffer,
        );
    }
}

/// Initialize the per-process slot allocator and POSIX memory manager from the
/// already-installed startup block metadata.
unsafe fn init_mm_from_auxv(stack_ptr: *const u64) {
    unsafe {
        let _ = stack_ptr;

        let sc_cap = resolve_runtime_mm_state(core::ptr::null());

        // Write to substrate global so TLS init can pick it up
        *(&raw mut trona_runtime::__trona_sc_cap) = sc_cap;

        // Initialize per-process slot allocator. Self-expansion is enabled
        // separately by trona_runtime_install once the rsrcsrv authority is
        // available; the allocator falls back to fail-fast if expansion is
        // attempted before that.
        if !trona_runtime::runtime_init_slot_allocator() {
            if let Some((slot_base, slot_count)) = trona_runtime::runtime_get_slot_pool() {
                trona_runtime::core::slot_alloc::slot_alloc_init(slot_base, slot_count);
            }
        }

        // posix_mm reads the mmsrv slot directly from `trona_runtime::client::caps::mmsrv_ep()`
        // (populated from the startup cap_table), so no explicit init call is
        // needed here.
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
            if func as usize != 0 {
                func();
            }
            p = p.add(1);
        }
    }
}

/// Call all function pointers in the .init_array section (forward order)
unsafe fn call_init_array(
    mut p: *const unsafe extern "C" fn(),
    end: *const unsafe extern "C" fn(),
) {
    unsafe {
        while p < end {
            let func = core::ptr::read(p);
            if func as usize != 0 {
                func();
            }
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
            if func as usize != 0 {
                func();
            }
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
