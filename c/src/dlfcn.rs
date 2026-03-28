//! Dynamic linking — dladdr, dlopen, dlsym, dlclose, dl_iterate_phdr
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! `dladdr()` is fully implemented: it walks the rtld's link_map chain
//! (via `__rtld_global`) to resolve an address to its containing DSO and
//! nearest preceding symbol. `dlopen`/`dlsym`/`dlclose` remain stubs
//! (runtime loading is not yet supported).

use crate::errno;

static mut DLERROR_MSG: [u8; 64] = [0; 64];

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

// ---------------------------------------------------------------------------
// Mirrored rtld structures (must match rtld_internal.h exactly)
// ---------------------------------------------------------------------------

/// ELF64 symbol table entry (matches Elf64_Sym).
#[repr(C)]
struct Elf64Sym {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
}

/// Per-DSO metadata (mirrors rtld's `struct link_map`).
#[repr(C)]
struct LinkMap {
    base: u64,
    name: *const u8,
    symtab: *const Elf64Sym,
    symtab_count: u64,
    sym_ent_size: u64,
    strtab: *const u8,
    strtab_size: u64,
    gnu_hash: *const u32,
    jmprel: *const u8,
    jmprel_count: u64,
    pltgot: *const u64,
    rela: *const u8,
    rela_count: u64,
    load_size: u64,
    init_fn: *const u8,
    init_array: *const u8,
    init_array_count: u64,
    tls_template: u64,
    tls_filesz: u64,
    tls_memsz: u64,
    tls_align: u64,
    tls_tpoff: i64,
    tls_module_id: u64,
    next: *const LinkMap,
}

/// Global rtld state (mirrors rtld's `struct rtld_state`).
/// We only need the `head` pointer at offset `objects[8] + nobjects`.
/// Rather than mirror the full struct, we access `__rtld_global` as an
/// opaque pointer and use the head field.
#[repr(C)]
struct RtldState {
    objects: [LinkMap; 8],
    nobjects: i32,
    head: *const LinkMap,
    // remaining fields omitted — we only need head
}

unsafe extern "C" {
    /// Pointer to rtld's global state, exported by ld-besalt.so.
    #[linkage = "extern_weak"]
    static __rtld_global: *const RtldState;
}

// ---------------------------------------------------------------------------
// dladdr — resolve address to DSO + nearest symbol
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct DlInfo {
    pub dli_fname: *const u8,
    pub dli_fbase: *mut u8,
    pub dli_sname: *const u8,
    pub dli_saddr: *mut u8,
}

/// Find the DSO that contains `addr` by walking the link_map chain.
unsafe fn addr_to_dso(addr: u64) -> *const LinkMap {
    unsafe {
        let rtld = *(&raw const __rtld_global);
        if rtld.is_null() {
            return core::ptr::null();
        }

        let mut map = (*rtld).head;
        while !map.is_null() {
            let base = (*map).base;
            let end = base + (*map).load_size;
            if addr >= base && addr < end {
                return map;
            }
            map = (*map).next;
        }
        core::ptr::null()
    }
}

/// Search a DSO's .dynsym for the nearest symbol with address <= `addr`.
unsafe fn find_nearest_symbol(map: *const LinkMap, addr: u64) -> (*const u8, u64) {
    unsafe {
        let symtab = (*map).symtab;
        let strtab = (*map).strtab;
        if symtab.is_null() || strtab.is_null() {
            return (core::ptr::null(), 0);
        }

        let count = if (*map).symtab_count > 0 {
            (*map).symtab_count as usize
        } else {
            // No count available — can't safely iterate
            return (core::ptr::null(), 0);
        };

        let base = (*map).base;
        let mut best_name: *const u8 = core::ptr::null();
        let mut best_addr: u64 = 0;

        for i in 0..count {
            let sym = &*symtab.add(i);

            // Skip undefined, section, and file symbols
            let stype = sym.st_info & 0xf;
            if sym.st_shndx == 0 || stype == 3 || stype == 4 {
                continue; // STT_SECTION=3, STT_FILE=4, SHN_UNDEF=0
            }

            let sym_addr = base + sym.st_value;
            if sym_addr <= addr && sym_addr > best_addr {
                best_addr = sym_addr;
                if (*map).strtab_size > 0 && (sym.st_name as u64) < (*map).strtab_size {
                    best_name = strtab.add(sym.st_name as usize);
                }
            }
        }

        (best_name, best_addr)
    }
}

/// Resolve an address to its containing DSO and nearest symbol.
///
/// Returns non-zero on success, 0 on failure (per POSIX).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dladdr(addr: *const u8, info: *mut DlInfo) -> i32 {
    unsafe {
        if info.is_null() {
            return 0;
        }

        let map = addr_to_dso(addr as u64);
        if map.is_null() {
            return 0;
        }

        (*info).dli_fname = (*map).name;
        (*info).dli_fbase = (*map).base as *mut u8;

        let (sym_name, sym_addr) = find_nearest_symbol(map, addr as u64);
        (*info).dli_sname = sym_name;
        (*info).dli_saddr = sym_addr as *mut u8;

        1 // success
    }
}

// ---------------------------------------------------------------------------
// dl_iterate_phdr — iterate over loaded shared objects
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct DlPhdrInfo {
    pub dlpi_addr: usize,
    pub dlpi_name: *const u8,
    pub dlpi_phdr: *const u8,
    pub dlpi_phnum: u16,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dl_iterate_phdr(
    callback: Option<unsafe extern "C" fn(*mut DlPhdrInfo, usize, *mut u8) -> i32>,
    data: *mut u8,
) -> i32 {
    let Some(cb) = callback else {
        return 0;
    };

    unsafe {
        let rtld = *(&raw const __rtld_global);
        if rtld.is_null() {
            return 0;
        }

        let mut map = (*rtld).head;
        while !map.is_null() {
            let mut info = DlPhdrInfo {
                dlpi_addr: (*map).base as usize,
                dlpi_name: (*map).name,
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
            map = (*map).next;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// dlopen / dlsym / dlclose / dlerror — stubs
// ---------------------------------------------------------------------------

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
