# saltyc — SaltyOS C Standard Library (Rust + ASM → libc.so)
# SPDX-License-Identifier: GPL-2.0-only

saltyc_link = meson.current_source_dir() / 'arch' / arch / 'saltyc.ld'
rust_build_dir = meson.project_build_root() / 'rust'

# Step 1: Rust -> .o + .rmeta (depends on libbesalt for --extern salty)
saltyc_rust = custom_target('saltyc_rust',
  input: files('src/lib.rs'),
  output: ['saltyc.o', 'saltyc.rmeta'],
  depfile: 'saltyc.d',
  command: [rustc,
    rust_userland_args,
    '--emit=obj=@OUTPUT0@,metadata=@OUTPUT1@,dep-info=@DEPFILE@',
    '--crate-name=saltyc',
    '--crate-type=lib',
    '--extern', 'salty=' + libbesalt_build_dir / 'libbesalt.rmeta',
    '@INPUT@',
  ],
  depends: [core, compiler_builtins, libbesalt_rust],
  build_by_default: true,
)

saltyc_obj = saltyc_rust[0]

# Step 2: crt_start_{arch}.S -> crt_start.o
crt_start_obj = custom_target('crt_start_asm',
  input: files('src/arch' / arch / 'crt_start.S'),
  output: 'crt_start.o',
  command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3: setjmp_{arch}.S -> setjmp.o
setjmp_obj = custom_target('setjmp_asm',
  input: files('src/arch' / arch / 'setjmp.S'),
  output: 'setjmp.o',
  command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Need clang's resource dir for compiler-provided headers (stddef.h, etc.)
_clang_resource_dir = run_command(
  cc_cmd[0], '--print-resource-dir', capture: true, check: true,
).stdout().strip()

# Common C compile flags for freestanding libc components linked into libc.so.
# Keep libc internals freestanding, but force PIC for shared-library linkage.
_saltyc_c_flags = [
  '--target=' + c_target,
  '-ffreestanding', '-nostdinc', '-fno-stack-protector',
  '-fPIC',
  '-isystem', meson.current_source_dir() / 'include',
  '-isystem', _clang_resource_dir / 'include',
]

# Step 3b: fts.c -> fts.o (BSD file tree stream, compiled as C)
fts_obj = custom_target('fts_c',
  input: files('src/fts.c'),
  output: 'fts.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3c: getopt.c -> getopt.o (POSIX getopt + GNU getopt_long)
getopt_obj = custom_target('getopt_c',
  input: files('src/getopt.c'),
  output: 'getopt.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3d: libutil_compat.c -> libutil_compat.o (expand_number, fgetln)
libutil_compat_obj = custom_target('libutil_compat_c',
  input: files('src/compat/libutil_compat.c'),
  output: 'libutil_compat.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3e: md5.c -> md5.o (RFC 1321 MD5)
md5_obj = custom_target('md5_c',
  input: files('src/compat/md5.c'),
  output: 'md5.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3f: cap_fileargs.c -> cap_fileargs.o (Capsicum fileargs wrapper)
cap_fileargs_obj = custom_target('cap_fileargs_c',
  input: files('src/compat/cap_fileargs.c'),
  output: 'cap_fileargs.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3g: xo_stub.c -> xo_stub.o (libxo text-mode stub)
xo_stub_obj = custom_target('xo_stub_c',
  input: files('src/compat/xo_stub.c'),
  output: 'xo_stub.o',
  command: [cc_cmd, _saltyc_c_flags, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 4: saltyc.o + setjmp.o + fts.o + C libs + core.o -> libc.so
# NOTE: crt_start_obj is NOT linked into libc.so — it is a separate static
# object that C programs link directly, so that libc.so does not carry an
# unresolved reference to main().
libc_so = custom_target('libc_so',
  input: [saltyc_obj, setjmp_obj, fts_obj, getopt_obj, libutil_compat_obj,
          md5_obj, cap_fileargs_obj, xo_stub_obj, core_obj, compiler_builtins_obj],
  output: 'libc.so',
  command: [cc_cmd,
    '--target=' + c_target,
    '-shared',
    '-nostdlib', '-nostartfiles',
    '-fno-stack-protector',
    '-Wl,-soname,libc.so',
    '-T', saltyc_link,
    '-o', '@OUTPUT@',
    '@INPUT@',
  ],
  build_by_default: true,
)

saltyc_inc = include_directories('include')
saltyc_inc_dir = meson.current_source_dir() / 'include'

# Aliases used by the outer repo's userland/ports meson.build files.
besaltc_inc_dir = saltyc_inc_dir
besaltc_build_dir = meson.current_build_dir()

# Collect all C headers for port dependency tracking.
# Ports compile with -isystem pointing here; when headers change, ports
# must rebuild.  Mirrors the kernel's find-based .S discovery.
_saltyc_hdr_list = run_command(
  'find', 'include', '-name', '*.h',
  capture: true, check: true,
).stdout().strip()

saltyc_header_files = []
if _saltyc_hdr_list != ''
  foreach h : _saltyc_hdr_list.split('\n')
    saltyc_header_files += files(h)
  endforeach
endif

besaltc_header_files = saltyc_header_files
