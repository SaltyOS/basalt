//! Dynamic linking — dladdr, dlopen, dlsym, dlclose, dl_iterate_phdr
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! SaltyOS already has a startup-time rtld.  This file adds the user-visible
//! `dlopen(3)` family on top of it by:
//! - reusing the rtld link-map chain for already-loaded DSOs, and
//! - loading extra ET_DYN objects from the filesystem into anonymous memory.
//!
//! Dynamic TLS for `dlopen()` objects is still unsupported because the runtime
//! loader itself only supports static TLS established at process startup.

use crate::{errno, malloc, string};
use core::mem::{MaybeUninit, size_of};
use core::ptr;

const PAGE_SIZE: usize = trona::consts::kernel::ELF_PAGE_SIZE as usize;
const DL_HANDLE_MAGIC: u64 = 0x444c_4844_4c4f_4144;
const RTLD_MAX_OBJECTS: usize = 16;
const RTLD_MAX_OBJECT_NAME: usize = 96;
const EI_NIDENT: usize = 16;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u32 = 1;
const ET_DYN: u16 = 3;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_TLS: u32 = 7;

const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;
const PF_R: u32 = 0x4;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_STRSZ: i64 = 10;
const DT_SYMENT: i64 = 11;
const DT_INIT: i64 = 12;
const DT_FINI: i64 = 13;
const DT_SONAME: i64 = 14;
const DT_PLTGOT: i64 = 3;
const DT_PLTRELSZ: i64 = 2;
const DT_PLTREL: i64 = 20;
const DT_JMPREL: i64 = 23;
const DT_INIT_ARRAY: i64 = 25;
const DT_FINI_ARRAY: i64 = 26;
const DT_INIT_ARRAYSZ: i64 = 27;
const DT_FINI_ARRAYSZ: i64 = 28;
const DT_GNU_HASH: i64 = 0x6ffffef5;

const SHN_UNDEF: u16 = 0;
const STB_LOCAL: u8 = 0;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;
const STT_SECTION: u8 = 3;
const STT_FILE: u8 = 4;

#[cfg(target_arch = "x86_64")]
const R_NONE: u32 = 0;
#[cfg(target_arch = "x86_64")]
const R_ABS64: u32 = 1;
#[cfg(target_arch = "x86_64")]
const R_GLOB_DAT: u32 = 6;
#[cfg(target_arch = "x86_64")]
const R_JUMP_SLOT: u32 = 7;
#[cfg(target_arch = "x86_64")]
const R_RELATIVE: u32 = 8;

#[cfg(target_arch = "aarch64")]
const R_NONE: u32 = 0;
#[cfg(target_arch = "aarch64")]
const R_ABS64: u32 = 257;
#[cfg(target_arch = "aarch64")]
const R_GLOB_DAT: u32 = 1025;
#[cfg(target_arch = "aarch64")]
const R_JUMP_SLOT: u32 = 1026;
#[cfg(target_arch = "aarch64")]
const R_RELATIVE: u32 = 1027;

const DL_MAIN_HANDLE: *mut u8 = 1 as *mut u8;

static DL_LOCK: trona::sync::Mutex = trona::sync::Mutex::new();
static mut DL_HEAD: *mut DlHandle = ptr::null_mut();
static mut DLERROR_MSG: [u8; 128] = [0; 128];
static mut DLERROR_RET: [u8; 128] = [0; 128];

#[repr(C)]
struct Elf64Ehdr {
    e_ident: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[repr(C)]
struct Elf64Dyn {
    d_tag: i64,
    d_val: u64,
}

#[repr(C)]
struct Elf64Sym {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
}

#[repr(C)]
struct Elf64Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

#[repr(C)]
struct LinkMap {
    base: u64,
    name: *const u8,
    name_storage: [u8; RTLD_MAX_OBJECT_NAME],
    symtab: *const Elf64Sym,
    symtab_count: u64,
    sym_ent_size: u64,
    strtab: *const u8,
    strtab_size: u64,
    gnu_hash: *const u32,
    jmprel: *const Elf64Rela,
    jmprel_count: u64,
    jmprel_ent_size: u64,
    pltgot: *const u64,
    rela: *const Elf64Rela,
    rela_count: u64,
    rela_ent_size: u64,
    load_size: u64,
    init_fn: *const u8,
    init_array: *const *const u8,
    init_array_count: u64,
    tls_template: u64,
    tls_filesz: u64,
    tls_memsz: u64,
    tls_align: u64,
    tls_tpoff: i64,
    tls_module_id: u64,
    dyn_section: *const Elf64Dyn,
    next: *const LinkMap,
}

#[repr(C)]
struct RtldState {
    objects: [LinkMap; RTLD_MAX_OBJECTS],
    nobjects: i32,
    head: *const LinkMap,
}

#[repr(C)]
struct DlHandle {
    magic: u64,
    next: *mut DlHandle,
    path: *mut u8,
    mapping: *mut u8,
    mapping_len: usize,
    phdr: *const Elf64Phdr,
    phnum: u16,
    refcount: usize,
    fini_fn: *const u8,
    fini_array: *const *const u8,
    fini_array_count: usize,
    map: LinkMap,
}

const DEP_SEARCH_PATHS: [&[u8]; 3] = [b"/usr/lib/", b"/lib/", b"/usr/local/lib/"];

// Patched by ld-trona.so after process startup so libc can walk the rtld link map.
#[unsafe(no_mangle)]
pub static mut __rtld_global: *const RtldState = ptr::null();

#[repr(C)]
pub struct DlInfo {
    pub dli_fname: *const u8,
    pub dli_fbase: *mut u8,
    pub dli_sname: *const u8,
    pub dli_saddr: *mut u8,
}

#[repr(C)]
pub struct DlPhdrInfo {
    pub dlpi_addr: usize,
    pub dlpi_name: *const u8,
    pub dlpi_phdr: *const Elf64Phdr,
    pub dlpi_phnum: u16,
}

fn elf_r_sym(info: u64) -> u32 {
    (info >> 32) as u32
}

fn elf_r_type(info: u64) -> u32 {
    info as u32
}

fn elf_st_bind(info: u8) -> u8 {
    info >> 4
}

fn page_down(v: u64) -> u64 {
    v & !((PAGE_SIZE as u64) - 1)
}

fn page_up(v: u64) -> u64 {
    (v + PAGE_SIZE as u64 - 1) & !((PAGE_SIZE as u64) - 1)
}

fn elf_machine_ok(machine: u16) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        machine == trona::consts::kernel::EM_X86_64
    }
    #[cfg(target_arch = "aarch64")]
    {
        machine == trona::consts::kernel::EM_AARCH64
    }
}

fn elf_flags_to_prot(flags: u32) -> i32 {
    let mut prot = 0;
    if (flags & PF_R) != 0 {
        prot |= trona::consts::posix::PROT_READ;
    }
    if (flags & PF_W) != 0 {
        prot |= trona::consts::posix::PROT_WRITE;
    }
    if (flags & PF_X) != 0 {
        prot |= trona::consts::posix::PROT_EXEC;
    }
    prot
}

unsafe fn set_dlerror(msg: &[u8]) {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    let limit = 127usize.min(msg.len());
    for i in 0..limit {
        unsafe { *buf.add(i) = msg[i] };
    }
    unsafe { *buf.add(limit) = 0 };
}

unsafe fn dlerror_is_set() -> bool {
    unsafe { *(core::ptr::addr_of!(DLERROR_MSG) as *const u8) != 0 }
}

fn relocation_type_name(rtype: u32) -> &'static [u8] {
    match rtype {
        R_NONE => b"R_NONE",
        R_ABS64 => b"R_ABS64",
        R_GLOB_DAT => b"R_GLOB_DAT",
        R_JUMP_SLOT => b"R_JUMP_SLOT",
        R_RELATIVE => b"R_RELATIVE",
        _ => b"R_UNKNOWN",
    }
}

unsafe fn set_dlerror_relocation(reason: &[u8], symbol: *const u8, rtype: u32) {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    let mut pos = 0usize;

    for &b in b"dlopen: relocation failed: " {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    for &b in reason {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    if !symbol.is_null() && pos < 127 {
        unsafe { *buf.add(pos) = b' ' };
        pos += 1;
        let mut p = symbol;
        unsafe {
            while *p != 0 && pos < 127 {
                *buf.add(pos) = *p;
                pos += 1;
                p = p.add(1);
            }
        }
    }
    if pos < 127 {
        unsafe { *buf.add(pos) = b' ' };
        pos += 1;
    }
    for &b in b"type=" {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    for &b in relocation_type_name(rtype) {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    unsafe { *buf.add(pos.min(127)) = 0 };
}

unsafe fn report_relocation_failure(
    _handle: *mut DlHandle,
    reloc: *const Elf64Rela,
    symbol: *const u8,
    reason: &[u8],
) {
    unsafe {
        let rtype = elf_r_type((*reloc).r_info);
        set_dlerror_relocation(reason, symbol, rtype);
    }
}

unsafe fn set_dlerror_cstr(prefix: &[u8], cstr: *const u8) {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    let mut pos = 0usize;
    for &b in prefix {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    if !cstr.is_null() {
        let mut p = cstr;
        unsafe {
            while *p != 0 && pos < 127 {
                *buf.add(pos) = *p;
                pos += 1;
                p = p.add(1);
            }
        }
    }
    unsafe { *buf.add(pos) = 0 };
}

unsafe fn set_dlerror_bytes(msg: &[u8]) {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    let mut pos = 0usize;
    for &b in msg {
        if pos >= 127 {
            break;
        }
        unsafe { *buf.add(pos) = b };
        pos += 1;
    }
    unsafe { *buf.add(pos) = 0 };
}

unsafe fn clear_dlerror() {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    unsafe { *buf = 0 };
}

unsafe fn parse_dynamic(map: *mut LinkMap, dynv: *const Elf64Dyn, base: u64) {
    unsafe {
        (*map).base = base;
        (*map).symtab = ptr::null();
        (*map).symtab_count = 0;
        (*map).sym_ent_size = size_of::<Elf64Sym>() as u64;
        (*map).strtab = ptr::null();
        (*map).strtab_size = 0;
        (*map).gnu_hash = ptr::null();
        (*map).jmprel = ptr::null();
        (*map).jmprel_count = 0;
        (*map).jmprel_ent_size = size_of::<Elf64Rela>() as u64;
        (*map).pltgot = ptr::null();
        (*map).rela = ptr::null();
        (*map).rela_count = 0;
        (*map).rela_ent_size = size_of::<Elf64Rela>() as u64;
        (*map).tls_template = 0;
        (*map).tls_filesz = 0;
        (*map).tls_memsz = 0;
        (*map).tls_align = 1;
        (*map).tls_tpoff = 0;
        (*map).tls_module_id = 0;
        (*map).dyn_section = dynv;
        (*map).init_fn = ptr::null();
        (*map).init_array = ptr::null();
        (*map).init_array_count = 0;

        let mut rela_size = 0u64;
        let mut jmprel_size = 0u64;
        let mut pltrel_type = DT_RELA;

        let mut i = 0usize;
        loop {
            let d = &*dynv.add(i);
            if d.d_tag == DT_NULL {
                break;
            }
            match d.d_tag {
                DT_SYMTAB => (*map).symtab = (base + d.d_val) as *const Elf64Sym,
                DT_STRTAB => (*map).strtab = (base + d.d_val) as *const u8,
                DT_STRSZ => (*map).strtab_size = d.d_val,
                DT_SYMENT => {
                    if d.d_val != 0 {
                        (*map).sym_ent_size = d.d_val;
                    }
                }
                DT_GNU_HASH => (*map).gnu_hash = (base + d.d_val) as *const u32,
                DT_JMPREL => (*map).jmprel = (base + d.d_val) as *const Elf64Rela,
                DT_PLTRELSZ => jmprel_size = d.d_val,
                DT_PLTREL => pltrel_type = d.d_val as i64,
                DT_PLTGOT => (*map).pltgot = (base + d.d_val) as *const u64,
                DT_RELA => (*map).rela = (base + d.d_val) as *const Elf64Rela,
                DT_RELASZ => rela_size = d.d_val,
                DT_RELAENT => {
                    if d.d_val >= size_of::<Elf64Rela>() as u64 {
                        (*map).rela_ent_size = d.d_val;
                        (*map).jmprel_ent_size = d.d_val;
                    }
                }
                DT_INIT => (*map).init_fn = (base + d.d_val) as *const u8,
                DT_INIT_ARRAY => (*map).init_array = (base + d.d_val) as *const *const u8,
                DT_INIT_ARRAYSZ => {
                    (*map).init_array_count = d.d_val / size_of::<*const u8>() as u64
                }
                _ => {}
            }
            i += 1;
        }

        if !(*map).rela.is_null() && (*map).rela_ent_size >= size_of::<Elf64Rela>() as u64 {
            (*map).rela_count = rela_size / (*map).rela_ent_size;
        }
        if pltrel_type == DT_RELA
            && !(*map).jmprel.is_null()
            && (*map).jmprel_ent_size >= size_of::<Elf64Rela>() as u64
        {
            (*map).jmprel_count = jmprel_size / (*map).jmprel_ent_size;
        } else {
            (*map).jmprel = ptr::null();
            (*map).jmprel_count = 0;
        }

        if !(*map).symtab.is_null()
            && !(*map).strtab.is_null()
            && (*map).sym_ent_size != 0
            && ((*map).strtab as usize) > ((*map).symtab as usize)
        {
            let bytes = ((*map).strtab as usize - (*map).symtab as usize) as u64;
            (*map).symtab_count = bytes / (*map).sym_ent_size;
        }
    }
}

unsafe fn parse_fini(handle: *mut DlHandle, dynv: *const Elf64Dyn, base: u64) {
    unsafe {
        let mut i = 0usize;
        loop {
            let d = &*dynv.add(i);
            if d.d_tag == DT_NULL {
                break;
            }
            match d.d_tag {
                DT_FINI => (*handle).fini_fn = (base + d.d_val) as *const u8,
                DT_FINI_ARRAY => (*handle).fini_array = (base + d.d_val) as *const *const u8,
                DT_FINI_ARRAYSZ => {
                    (*handle).fini_array_count = (d.d_val / size_of::<*const u8>() as u64) as usize
                }
                _ => {}
            }
            i += 1;
        }
    }
}

unsafe fn addr_to_custom_dso(addr: u64) -> *const LinkMap {
    unsafe {
        let mut handle = *(&raw const DL_HEAD);
        while !handle.is_null() {
            let map = &(*handle).map;
            let end = map.base + map.load_size;
            if addr >= map.base && addr < end {
                return map as *const LinkMap;
            }
            handle = (*handle).next;
        }
        ptr::null()
    }
}

unsafe fn addr_to_dso(addr: u64) -> *const LinkMap {
    unsafe {
        let rtld = *(&raw const __rtld_global);
        if !rtld.is_null() {
            let mut map = (*rtld).head;
            while !map.is_null() {
                let base = (*map).base;
                let end = base + (*map).load_size;
                if addr >= base && addr < end {
                    return map;
                }
                map = (*map).next;
            }
        }
        addr_to_custom_dso(addr)
    }
}

unsafe fn find_nearest_symbol(map: *const LinkMap, addr: u64) -> (*const u8, u64) {
    unsafe {
        if map.is_null() || (*map).symtab.is_null() || (*map).strtab.is_null() {
            return (ptr::null(), 0);
        }

        let count = if (*map).symtab_count != 0 {
            (*map).symtab_count as usize
        } else {
            4096
        };

        let mut best_name = ptr::null();
        let mut best_addr = 0u64;
        for i in 0..count {
            let sym = &*(*map).symtab.add(i);
            let stype = sym.st_info & 0x0f;
            if sym.st_shndx == SHN_UNDEF || stype == STT_SECTION || stype == STT_FILE {
                continue;
            }
            let sym_addr = (*map).base + sym.st_value;
            if sym_addr <= addr && sym_addr > best_addr {
                best_addr = sym_addr;
                if (*map).strtab_size == 0 || (sym.st_name as u64) < (*map).strtab_size {
                    best_name = (*map).strtab.add(sym.st_name as usize);
                }
            }
        }
        (best_name, best_addr)
    }
}

unsafe fn gnu_hash(name: *const u8) -> u32 {
    let mut h = 5381u32;
    unsafe {
        let mut p = name;
        while !p.is_null() && *p != 0 {
            h = (h << 5).wrapping_add(h).wrapping_add(*p as u32);
            p = p.add(1);
        }
    }
    h
}

unsafe fn gnu_hash_lookup(map: *const LinkMap, name: *const u8) -> u64 {
    unsafe {
        if map.is_null() || (*map).gnu_hash.is_null() || (*map).symtab.is_null() || (*map).strtab.is_null() {
            return 0;
        }

        let hashtab = (*map).gnu_hash;
        let nbuckets = *hashtab.add(0);
        let symoffset = *hashtab.add(1);
        let bloom_size = *hashtab.add(2);
        let bloom_shift = *hashtab.add(3);
        if nbuckets == 0 || bloom_size == 0 {
            return 0;
        }
        if (*map).symtab_count != 0 && symoffset as u64 >= (*map).symtab_count {
            return 0;
        }

        let bloom = hashtab.add(4) as *const u64;
        let buckets = bloom.add(bloom_size as usize) as *const u32;
        let chain = buckets.add(nbuckets as usize);
        let h = gnu_hash(name);

        let word = *bloom.add(((h / 64) % bloom_size) as usize);
        let mask = (1u64 << (h % 64)) | (1u64 << ((h >> bloom_shift) % 64));
        if (word & mask) != mask {
            return 0;
        }

        let mut idx = *buckets.add((h % nbuckets) as usize);
        if idx < symoffset {
            return 0;
        }
        if (*map).symtab_count != 0 && idx as u64 >= (*map).symtab_count {
            return 0;
        }

        let max_steps = if (*map).symtab_count != 0 && (*map).symtab_count > symoffset as u64 {
            ((*map).symtab_count - symoffset as u64) as u32
        } else {
            4096
        };

        let mut steps = 0u32;
        loop {
            if steps >= max_steps {
                return 0;
            }
            steps += 1;

            let chain_hash = *chain.add((idx - symoffset) as usize);
            if (h | 1) == (chain_hash | 1) {
                let sym = &*(*map).symtab.add(idx as usize);
                if (*map).strtab_size == 0 || (sym.st_name as u64) < (*map).strtab_size {
                    let sym_name = (*map).strtab.add(sym.st_name as usize);
                    if string::strcmp(name, sym_name) == 0 && sym.st_shndx != SHN_UNDEF {
                        return (*map).base + sym.st_value;
                    }
                }
            }

            if (chain_hash & 1) != 0 {
                break;
            }
            idx += 1;
            if (*map).symtab_count != 0 && idx as u64 >= (*map).symtab_count {
                break;
            }
        }

        0
    }
}

unsafe fn linear_lookup(map: *const LinkMap, name: *const u8) -> u64 {
    unsafe {
        if map.is_null() || (*map).symtab.is_null() || (*map).strtab.is_null() {
            return 0;
        }
        let limit = if (*map).symtab_count != 0 {
            (*map).symtab_count as usize
        } else {
            4096
        };
        for i in 0..limit {
            let sym = &*(*map).symtab.add(i);
            if sym.st_name == 0 {
                if sym.st_value == 0 && sym.st_size == 0 && sym.st_info == 0 && i > 1 {
                    break;
                }
                continue;
            }
            if (*map).strtab_size != 0 && (sym.st_name as u64) >= (*map).strtab_size {
                continue;
            }
            if sym.st_shndx == SHN_UNDEF {
                continue;
            }
            let bind = elf_st_bind(sym.st_info);
            if bind != STB_GLOBAL && bind != STB_WEAK {
                continue;
            }
            let sym_name = (*map).strtab.add(sym.st_name as usize);
            if string::strcmp(name, sym_name) == 0 {
                return (*map).base + sym.st_value;
            }
        }
        0
    }
}

unsafe fn lookup_symbol_in_map(map: *const LinkMap, name: *const u8) -> u64 {
    unsafe {
        let addr = gnu_hash_lookup(map, name);
        if addr != 0 {
            return addr;
        }
        linear_lookup(map, name)
    }
}

unsafe fn resolve_symbol_global(preferred: *const LinkMap, name: *const u8) -> u64 {
    unsafe {
        if !preferred.is_null() {
            let addr = lookup_symbol_in_map(preferred, name);
            if addr != 0 {
                return addr;
            }
        }

        let rtld = *(&raw const __rtld_global);
        if !rtld.is_null() {
            let mut map = (*rtld).head;
            while !map.is_null() {
                let addr = lookup_symbol_in_map(map, name);
                if addr != 0 {
                    return addr;
                }
                map = (*map).next;
            }
        }

        let mut handle = *(&raw const DL_HEAD);
        while !handle.is_null() {
            let map = &(*handle).map as *const LinkMap;
            if map != preferred {
                let addr = lookup_symbol_in_map(map, name);
                if addr != 0 {
                    return addr;
                }
            }
            handle = (*handle).next;
        }

        0
    }
}

unsafe fn process_relocation(handle: *mut DlHandle, rela: *const Elf64Rela) -> Result<(), ()> {
    unsafe {
        let map = &mut (*handle).map;
        let reloc = &*rela;
        let target = (map.base + reloc.r_offset) as *mut u64;
        let rtype = elf_r_type(reloc.r_info);
        let sym_idx = elf_r_sym(reloc.r_info) as usize;
        let mut symbol_name: *const u8 = ptr::null();

        match rtype {
            R_NONE => {}
            R_RELATIVE => {
                *target = map.base.wrapping_add(reloc.r_addend as u64);
            }
            R_ABS64 | R_GLOB_DAT | R_JUMP_SLOT => {
                if map.symtab.is_null() {
                    report_relocation_failure(handle, rela, ptr::null(), b"missing symtab");
                    return Err(());
                }
                if map.symtab_count != 0 && sym_idx >= map.symtab_count as usize {
                    report_relocation_failure(handle, rela, ptr::null(), b"symbol index out of range");
                    return Err(());
                }
                let sym = &*map.symtab.add(sym_idx);
                let bind = elf_st_bind(sym.st_info);
                let mut sym_addr = 0u64;
                if !map.strtab.is_null()
                    && (map.strtab_size == 0 || (sym.st_name as u64) < map.strtab_size)
                {
                    symbol_name = map.strtab.add(sym.st_name as usize);
                }

                if bind == STB_LOCAL {
                    sym_addr = map.base + sym.st_value;
                } else if sym.st_shndx != SHN_UNDEF {
                    sym_addr = resolve_symbol_global(map as *const LinkMap, symbol_name);
                    if sym_addr == 0 {
                        sym_addr = map.base + sym.st_value;
                    }
                } else if !symbol_name.is_null() {
                    sym_addr = resolve_symbol_global(ptr::null(), symbol_name);
                }

                if sym_addr == 0 && bind != STB_WEAK {
                    report_relocation_failure(handle, rela, symbol_name, b"unresolved symbol");
                    return Err(());
                }

                if rtype == R_ABS64 {
                    *target = sym_addr.wrapping_add(reloc.r_addend as u64);
                } else {
                    *target = sym_addr;
                }
            }
            _ => {
                report_relocation_failure(handle, rela, ptr::null(), b"unsupported relocation type");
                return Err(());
            }
        }

        Ok(())
    }
}

unsafe fn load_rela_entry(base: *const Elf64Rela, ent_size: u64, index: usize, out: *mut Elf64Rela) -> bool {
    unsafe {
        if base.is_null() || out.is_null() || ent_size < size_of::<Elf64Rela>() as u64 {
            return false;
        }
        ptr::copy_nonoverlapping(
            (base as *const u8).add(index * ent_size as usize),
            out as *mut u8,
            size_of::<Elf64Rela>(),
        );
        true
    }
}

unsafe fn process_relocations(handle: *mut DlHandle) -> Result<(), ()> {
    unsafe {
        let map = &(*handle).map;
        for i in 0..map.rela_count as usize {
            let mut rela = MaybeUninit::<Elf64Rela>::uninit();
            if !load_rela_entry(map.rela, map.rela_ent_size, i, rela.as_mut_ptr()) {
                set_dlerror_bytes(b"dlopen: relocation table metadata invalid");
                return Err(());
            }
            process_relocation(handle, rela.as_ptr())?;
        }
        for i in 0..map.jmprel_count as usize {
            let mut rela = MaybeUninit::<Elf64Rela>::uninit();
            if !load_rela_entry(map.jmprel, map.jmprel_ent_size, i, rela.as_mut_ptr()) {
                set_dlerror_bytes(b"dlopen: relocation table metadata invalid");
                return Err(());
            }
            process_relocation(handle, rela.as_ptr())?;
        }
        Ok(())
    }
}

unsafe fn call_init_functions(handle: *mut DlHandle) {
    unsafe {
        let map = &(*handle).map;
        if !map.init_fn.is_null() {
            let init: extern "C" fn() = core::mem::transmute(map.init_fn);
            init();
        }
        for i in 0..map.init_array_count as usize {
            let f = *map.init_array.add(i);
            if !f.is_null() {
                let init: extern "C" fn() = core::mem::transmute(f);
                init();
            }
        }
    }
}

unsafe fn call_fini_functions(handle: *mut DlHandle) {
    unsafe {
        for i in (0..(*handle).fini_array_count).rev() {
            let f = *(*handle).fini_array.add(i);
            if !f.is_null() {
                let fini: extern "C" fn() = core::mem::transmute(f);
                fini();
            }
        }
        if !(*handle).fini_fn.is_null() {
            let fini: extern "C" fn() = core::mem::transmute((*handle).fini_fn);
            fini();
        }
    }
}

unsafe fn read_entire_file(path: *const u8) -> Result<(*mut u8, usize), ()> {
    unsafe {
        let fd = trona_posix::posix_open(path, trona::consts::posix::O_RDONLY as i32, 0);
        if fd < 0 {
            errno::set_errno(-fd);
            set_dlerror(b"dlopen: open failed");
            return Err(());
        }

        let size = trona_posix::posix_lseek(fd, 0, trona::consts::posix::SEEK_END as i32);
        if size < 0 {
            let _ = trona_posix::posix_close(fd);
            errno::set_errno((-size) as i32);
            set_dlerror(b"dlopen: lseek SEEK_END failed");
            return Err(());
        }
        if trona_posix::posix_lseek(fd, 0, trona::consts::posix::SEEK_SET as i32) < 0 {
            let _ = trona_posix::posix_close(fd);
            errno::set_errno(errno::EIO);
            set_dlerror(b"dlopen: lseek SEEK_SET failed");
            return Err(());
        }

        let len = size as usize;
        if len == 0 {
            let _ = trona_posix::posix_close(fd);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: file is empty");
            return Err(());
        }

        let buf = malloc::malloc(len);
        if buf.is_null() {
            let _ = trona_posix::posix_close(fd);
            errno::set_errno(errno::ENOMEM);
            set_dlerror(b"dlopen: malloc failed");
            return Err(());
        }

        let mut done = 0usize;
        while done < len {
            let ret = trona_posix::posix_read(fd, buf.add(done), (len - done) as u64);
            if ret <= 0 {
                let _ = trona_posix::posix_close(fd);
                malloc::free(buf);
                errno::set_errno(if ret < 0 { (-ret) as i32 } else { errno::EIO });
                set_dlerror(b"dlopen: read failed");
                return Err(());
            }
            done += ret as usize;
        }

        let _ = trona_posix::posix_close(fd);
        Ok((buf, len))
    }
}

unsafe fn find_loaded_handle(path: *const u8) -> *mut DlHandle {
    unsafe {
        let mut handle = *(&raw const DL_HEAD);
        while !handle.is_null() {
            if !(*handle).path.is_null() && string::strcmp((*handle).path, path) == 0 {
                return handle;
            }
            handle = (*handle).next;
        }
        ptr::null_mut()
    }
}

unsafe fn cstr_basename(path: *const u8) -> *const u8 {
    unsafe {
        let base = string::strrchr(path, b'/' as i32);
        if base.is_null() {
            path
        } else {
            base.add(1)
        }
    }
}

unsafe fn find_loaded_handle_by_name(name: *const u8) -> *mut DlHandle {
    unsafe {
        let name_has_slash = !string::strchr(name, b'/' as i32).is_null();
        let mut handle = *(&raw const DL_HEAD);
        while !handle.is_null() {
            if !(*handle).path.is_null() {
                let matched = if name_has_slash {
                    string::strcmp((*handle).path, name) == 0
                } else {
                    string::strcmp(cstr_basename((*handle).path), name) == 0
                };
                if matched {
                    return handle;
                }
            }
            handle = (*handle).next;
        }
        ptr::null_mut()
    }
}

unsafe fn startup_object_loaded(name: *const u8) -> bool {
    unsafe {
        let rtld = *(&raw const __rtld_global);
        if rtld.is_null() || name.is_null() || *name == 0 {
            return false;
        }

        let name_has_slash = !string::strchr(name, b'/' as i32).is_null();
        let mut map = (*rtld).head;
        while !map.is_null() {
            if !(*map).name.is_null() {
                let matched = if name_has_slash {
                    string::strcmp((*map).name, name) == 0
                } else {
                    string::strcmp(cstr_basename((*map).name), name) == 0
                };
                if matched {
                    return true;
                }
            }
            map = (*map).next;
        }
        false
    }
}

unsafe fn insert_handle(handle: *mut DlHandle) {
    unsafe {
        (*handle).next = *(&raw const DL_HEAD);
        *(&raw mut DL_HEAD) = handle;
    }
}

unsafe fn load_object_locked(path: *const u8, flags: i32) -> Result<*mut DlHandle, ()> {
    unsafe {
        let existing = find_loaded_handle_by_name(path);
        if !existing.is_null() {
            (*existing).refcount = (*existing).refcount.saturating_add(1);
            return Ok(existing);
        }

        let handle = load_object_from_file(path, flags)?;
        insert_handle(handle);
        call_init_functions(handle);
        Ok(handle)
    }
}

unsafe fn load_dependency_by_name(name: *const u8, flags: i32) -> Result<(), ()> {
    unsafe {
        if name.is_null() || *name == 0 {
            return Ok(());
        }

        if startup_object_loaded(name) {
            return Ok(());
        }

        if !string::strchr(name, b'/' as i32).is_null() {
            let _ = load_object_locked(name, flags)?;
            return Ok(());
        }

        if !find_loaded_handle_by_name(name).is_null() {
            let _ = load_object_locked(name, flags)?;
            return Ok(());
        }

        let name_len = string::strlen(name);
        let mut full_path = [0u8; 256];

        for prefix in DEP_SEARCH_PATHS {
            if prefix.len() + name_len + 1 > full_path.len() {
                continue;
            }

            let mut pos = 0usize;
            for &b in prefix {
                full_path[pos] = b;
                pos += 1;
            }
            let mut i = 0usize;
            while i < name_len {
                full_path[pos] = *name.add(i);
                pos += 1;
                i += 1;
            }
            full_path[pos] = 0;

            match load_object_locked(full_path.as_ptr(), flags) {
                Ok(_) => return Ok(()),
                Err(()) => {
                    if errno::get_errno() != errno::ENOENT {
                        return Err(());
                    }
                    clear_dlerror();
                }
            }
        }

        errno::set_errno(errno::ENOENT);
        set_dlerror_cstr(b"dlopen: dependency not found: ", name);
        Err(())
    }
}

unsafe fn load_needed_objects(handle: *mut DlHandle, flags: i32) -> Result<(), ()> {
    unsafe {
        let map = &(*handle).map;
        if map.dyn_section.is_null() || map.strtab.is_null() {
            return Ok(());
        }

        let mut i = 0usize;
        loop {
            let dynent = &*map.dyn_section.add(i);
            if dynent.d_tag == DT_NULL {
                break;
            }
            if dynent.d_tag == DT_NEEDED {
                if map.strtab_size != 0 && dynent.d_val >= map.strtab_size {
                    errno::set_errno(errno::ENOEXEC);
                    set_dlerror(b"dlopen: invalid DT_NEEDED entry");
                    return Err(());
                }
                let dep_name = map.strtab.add(dynent.d_val as usize);
                load_dependency_by_name(dep_name, flags)?;
            }
            i += 1;
        }
        Ok(())
    }
}

unsafe fn remove_handle(handle: *mut DlHandle) {
    unsafe {
        let mut prev: *mut DlHandle = ptr::null_mut();
        let mut cur = *(&raw const DL_HEAD);
        while !cur.is_null() {
            if cur == handle {
                if prev.is_null() {
                    *(&raw mut DL_HEAD) = (*cur).next;
                } else {
                    (*prev).next = (*cur).next;
                }
                (*cur).next = ptr::null_mut();
                return;
            }
            prev = cur;
            cur = (*cur).next;
        }
    }
}

unsafe fn load_object_from_file(path: *const u8, flags: i32) -> Result<*mut DlHandle, ()> {
    unsafe {
        let (file_buf, file_len) = read_entire_file(path)?;

        if file_len < size_of::<Elf64Ehdr>() {
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: truncated ELF");
            return Err(());
        }

        let ehdr = &*(file_buf as *const Elf64Ehdr);
        if ehdr.e_ident[0] != 0x7f
            || ehdr.e_ident[1] != b'E'
            || ehdr.e_ident[2] != b'L'
            || ehdr.e_ident[3] != b'F'
            || ehdr.e_ident[4] != ELFCLASS64
            || ehdr.e_ident[5] != ELFDATA2LSB
            || ehdr.e_version != EV_CURRENT
            || ehdr.e_type != ET_DYN
            || !elf_machine_ok(ehdr.e_machine)
        {
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: unsupported ELF object");
            return Err(());
        }

        if ehdr.e_phentsize as usize != size_of::<Elf64Phdr>()
            || ehdr.e_phoff as usize > file_len
            || (ehdr.e_phoff as usize + ehdr.e_phnum as usize * size_of::<Elf64Phdr>()) > file_len
        {
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: invalid program headers");
            return Err(());
        }

        let phdrs = file_buf.add(ehdr.e_phoff as usize) as *const Elf64Phdr;
        let mut min_vaddr = u64::MAX;
        let mut max_vaddr = 0u64;
        let mut dynamic_vaddr = 0u64;

        for i in 0..ehdr.e_phnum as usize {
            let ph = &*phdrs.add(i);
            if ph.p_type == PT_TLS && ph.p_memsz != 0 {
                malloc::free(file_buf);
                errno::set_errno(errno::ENOSYS);
                set_dlerror(b"dlopen: PT_TLS is not supported");
                return Err(());
            }
            if ph.p_type == PT_LOAD {
                if ph.p_vaddr < min_vaddr {
                    min_vaddr = ph.p_vaddr;
                }
                let end = ph.p_vaddr.saturating_add(ph.p_memsz);
                if end > max_vaddr {
                    max_vaddr = end;
                }
            } else if ph.p_type == PT_DYNAMIC {
                dynamic_vaddr = ph.p_vaddr;
            }
        }

        if min_vaddr == u64::MAX || dynamic_vaddr == 0 {
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: missing PT_LOAD/PT_DYNAMIC");
            return Err(());
        }

        let aligned_min = page_down(min_vaddr);
        let aligned_max = page_up(max_vaddr);
        let span = aligned_max.saturating_sub(aligned_min) as usize;
        if span == 0 {
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            set_dlerror(b"dlopen: empty load span");
            return Err(());
        }

        let mapping = trona_posix::mm::posix_mmap(
            ptr::null_mut(),
            span as u64,
            trona::consts::posix::PROT_READ | trona::consts::posix::PROT_WRITE,
            trona::consts::posix::MAP_PRIVATE | trona::consts::posix::MAP_ANONYMOUS,
            -1,
            0,
        );
        if mapping as usize == usize::MAX {
            malloc::free(file_buf);
            set_dlerror(b"dlopen: mmap failed");
            return Err(());
        }

        ptr::write_bytes(mapping, 0, span);

        let page_count = span / PAGE_SIZE;
        let page_prots = malloc::calloc(page_count, size_of::<u8>());
        if page_prots.is_null() {
            let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
            malloc::free(file_buf);
            errno::set_errno(errno::ENOMEM);
            set_dlerror(b"dlopen: no memory for page protections");
            return Err(());
        }

        for i in 0..ehdr.e_phnum as usize {
            let ph = &*phdrs.add(i);
            if ph.p_type != PT_LOAD {
                continue;
            }
            if ph.p_offset.saturating_add(ph.p_filesz) > file_len as u64 {
                malloc::free(page_prots);
                let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
                malloc::free(file_buf);
                errno::set_errno(errno::ENOEXEC);
                set_dlerror(b"dlopen: segment exceeds file size");
                return Err(());
            }

            let dst = (mapping as u64 + (ph.p_vaddr - aligned_min)) as *mut u8;
            if ph.p_filesz != 0 {
                ptr::copy_nonoverlapping(file_buf.add(ph.p_offset as usize), dst, ph.p_filesz as usize);
            }
            if ph.p_memsz > ph.p_filesz {
                ptr::write_bytes(
                    dst.add(ph.p_filesz as usize),
                    0,
                    (ph.p_memsz - ph.p_filesz) as usize,
                );
            }

            let seg_start = page_down(ph.p_vaddr);
            let seg_end = page_up(ph.p_vaddr + ph.p_memsz);
            let prot = elf_flags_to_prot(ph.p_flags) as u8;
            let mut page = seg_start;
            while page < seg_end {
                let idx = ((page - aligned_min) as usize) / PAGE_SIZE;
                *(page_prots.add(idx)) |= prot;
                page += PAGE_SIZE as u64;
            }
        }

        let handle = malloc::calloc(1, size_of::<DlHandle>()) as *mut DlHandle;
        if handle.is_null() {
            malloc::free(page_prots);
            let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
            malloc::free(file_buf);
            errno::set_errno(errno::ENOMEM);
            set_dlerror(b"dlopen: handle allocation failed");
            return Err(());
        }

        let path_dup = malloc::strdup(path);
        if path_dup.is_null() {
            malloc::free(handle as *mut u8);
            malloc::free(page_prots);
            let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
            malloc::free(file_buf);
            errno::set_errno(errno::ENOMEM);
            set_dlerror(b"dlopen: path allocation failed");
            return Err(());
        }

        (*handle).magic = DL_HANDLE_MAGIC;
        (*handle).path = path_dup;
        (*handle).mapping = mapping;
        (*handle).mapping_len = span;
        (*handle).phdr = ptr::null();
        (*handle).phnum = 0;
        (*handle).refcount = 1;
        (*handle).map.name = path_dup;
        (*handle).map.base = mapping as u64 - aligned_min;
        (*handle).map.load_size = span as u64;
        (*handle).map.next = ptr::null();

        let dynv = ((*handle).map.base + dynamic_vaddr) as *const Elf64Dyn;
        parse_dynamic(&mut (*handle).map, dynv, (*handle).map.base);
        parse_fini(handle, dynv, (*handle).map.base);

        if load_needed_objects(handle, flags).is_err() {
            malloc::free(path_dup);
            malloc::free(handle as *mut u8);
            malloc::free(page_prots);
            let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
            malloc::free(file_buf);
            return Err(());
        }

        if process_relocations(handle).is_err() {
            malloc::free(path_dup);
            malloc::free(handle as *mut u8);
            malloc::free(page_prots);
            let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
            malloc::free(file_buf);
            errno::set_errno(errno::ENOEXEC);
            if !dlerror_is_set() {
                set_dlerror_cstr(b"dlopen: relocation failed for ", path);
            }
            return Err(());
        }

        let mut range_start = 0usize;
        while range_start < page_count {
            let prot = *(page_prots.add(range_start));
            let mut range_end = range_start + 1;
            while range_end < page_count && *(page_prots.add(range_end)) == prot {
                range_end += 1;
            }
            if prot != 0 {
                let addr = mapping.add(range_start * PAGE_SIZE);
                let len = (range_end - range_start) * PAGE_SIZE;
                if trona_posix::mm::posix_mprotect(addr, len as u64, prot as i32) < 0 {
                    malloc::free(path_dup);
                    malloc::free(handle as *mut u8);
                    malloc::free(page_prots);
                    let _ = trona_posix::mm::posix_munmap(mapping, span as u64);
                    malloc::free(file_buf);
                    errno::set_errno(errno::EACCES);
                    set_dlerror_cstr(b"dlopen: mprotect failed for ", path);
                    return Err(());
                }
            }
            range_start = range_end;
        }

        malloc::free(page_prots);
        malloc::free(file_buf);
        Ok(handle)
    }
}

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
        1
    }
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
        if !rtld.is_null() {
            let mut map = (*rtld).head;
            while !map.is_null() {
                let mut info = DlPhdrInfo {
                    dlpi_addr: (*map).base as usize,
                    dlpi_name: (*map).name,
                    dlpi_phdr: ptr::null(),
                    dlpi_phnum: 0,
                };
                let ret = cb(&mut info, size_of::<DlPhdrInfo>(), data);
                if ret != 0 {
                    return ret;
                }
                map = (*map).next;
            }
        }

        let mut handle = *(&raw const DL_HEAD);
        while !handle.is_null() {
            let mut info = DlPhdrInfo {
                dlpi_addr: (*handle).map.base as usize,
                dlpi_name: (*handle).path,
                dlpi_phdr: (*handle).phdr,
                dlpi_phnum: (*handle).phnum,
            };
            let ret = cb(&mut info, size_of::<DlPhdrInfo>(), data);
            if ret != 0 {
                return ret;
            }
            handle = (*handle).next;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlopen(filename: *const u8, flags: i32) -> *mut u8 {
    unsafe {
        clear_dlerror();

        if filename.is_null() {
            return DL_MAIN_HANDLE;
        }
        if *filename == 0 {
            errno::set_errno(errno::EINVAL);
            set_dlerror(b"dlopen: empty filename");
            return ptr::null_mut();
        }

        DL_LOCK.lock();

        let result = load_object_locked(filename, flags);
        match result {
            Ok(handle) => {
                DL_LOCK.unlock();
                handle as *mut u8
            }
            Err(()) => {
                DL_LOCK.unlock();
                ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(handle: *mut u8, symbol: *const u8) -> *mut u8 {
    unsafe {
        clear_dlerror();

        if symbol.is_null() || *symbol == 0 {
            errno::set_errno(errno::EINVAL);
            set_dlerror(b"dlsym: invalid symbol");
            return ptr::null_mut();
        }

        DL_LOCK.lock();

        let addr = if handle.is_null() || handle == (usize::MAX as *mut u8) {
            if handle == (usize::MAX as *mut u8) {
                errno::set_errno(errno::ENOSYS);
                set_dlerror(b"dlsym: RTLD_NEXT is not supported");
                DL_LOCK.unlock();
                return ptr::null_mut();
            }
            resolve_symbol_global(ptr::null(), symbol)
        } else if handle == DL_MAIN_HANDLE {
            resolve_symbol_global(ptr::null(), symbol)
        } else {
            let handle = handle as *mut DlHandle;
            if (*handle).magic != DL_HANDLE_MAGIC {
                errno::set_errno(errno::EINVAL);
                set_dlerror(b"dlsym: invalid handle");
                DL_LOCK.unlock();
                return ptr::null_mut();
            }
            let self_addr = lookup_symbol_in_map(&(*handle).map, symbol);
            if self_addr != 0 {
                self_addr
            } else {
                resolve_symbol_global(ptr::null(), symbol)
            }
        };

        DL_LOCK.unlock();

        if addr == 0 {
            errno::set_errno(errno::ENOENT);
            set_dlerror_cstr(b"dlsym: symbol not found: ", symbol);
            ptr::null_mut()
        } else {
            addr as *mut u8
        }
    }
}

/// FreeBSD `dlfunc` — like `dlsym` but returns a function pointer type.
/// This avoids undefined behavior when casting `void *` to a function pointer
/// (which is technically not portable in C, though works on all real platforms).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlfunc(handle: *mut u8, symbol: *const u8) -> Option<unsafe extern "C" fn()> {
    // SAFETY: dlsym returns a valid function address or null.
    unsafe {
        let addr = dlsym(handle, symbol);
        if addr.is_null() {
            None
        } else {
            Some(core::mem::transmute::<*mut u8, unsafe extern "C" fn()>(addr))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlclose(handle: *mut u8) -> i32 {
    unsafe {
        clear_dlerror();

        if handle.is_null() || handle == DL_MAIN_HANDLE {
            return 0;
        }

        let handle = handle as *mut DlHandle;
        if (*handle).magic != DL_HANDLE_MAGIC {
            errno::set_errno(errno::EINVAL);
            set_dlerror(b"dlclose: invalid handle");
            return -1;
        }

        DL_LOCK.lock();

        if (*handle).refcount > 1 {
            (*handle).refcount -= 1;
            DL_LOCK.unlock();
            return 0;
        }

        let mapping = (*handle).mapping;
        let mapping_len = (*handle).mapping_len;
        let path = (*handle).path;
        remove_handle(handle);
        (*handle).magic = 0;
        DL_LOCK.unlock();

        call_fini_functions(handle);
        if !mapping.is_null() && mapping_len != 0 {
            let _ = trona_posix::mm::posix_munmap(mapping, mapping_len as u64);
        }
        if !path.is_null() {
            malloc::free(path);
        }
        malloc::free(handle as *mut u8);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlerror() -> *mut u8 {
    let buf = core::ptr::addr_of_mut!(DLERROR_MSG) as *mut u8;
    let ret_buf = core::ptr::addr_of_mut!(DLERROR_RET) as *mut u8;
    unsafe {
        if *buf == 0 {
            return ptr::null_mut();
        }

        let mut i = 0usize;
        while i < 127 && *buf.add(i) != 0 {
            *ret_buf.add(i) = *buf.add(i);
            i += 1;
        }
        *ret_buf.add(i) = 0;

        *buf = 0;
        ret_buf
    }
}
