# basaltc — SaltyOS C Standard Library (Rust + ASM → libc.so)
# SPDX-License-Identifier: GPL-2.0-only

basaltc_link = meson.current_source_dir() / 'arch' / arch / 'basaltc.ld'
rust_build_dir = meson.project_build_root() / 'rust'

# Step 1: Rust -> .o + .rmeta (depends on libtrona for --extern trona_posix)
basaltc_rust = custom_target('basaltc_rust',
  input: files('src/lib.rs'),
  output: ['basaltc.o', 'basaltc.rmeta'],
  depfile: 'basaltc.d',
  command: [rustc,
    rust_userland_args,
    '--emit=obj=@OUTPUT0@,metadata=@OUTPUT1@,dep-info=@DEPFILE@',
    '--crate-name=basaltc',
    '--crate-type=lib',
    '--extern', 'trona=' + libtrona_build_dir / 'libtrona.rmeta',
    '--extern', 'trona_posix=' + libtrona_build_dir / 'libtrona_posix.rmeta',
    '@INPUT@',
  ],
  depends: [core, compiler_builtins, libtrona_rust, trona_posix],
  build_by_default: true,
)

basaltc_obj = basaltc_rust[0]

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

# Step 4: basaltc.o + setjmp.o + core.o -> libc.so
# NOTE: crt_start_obj is NOT linked into libc.so — it is a separate static
# object that C programs link directly, so that libc.so does not carry an
# unresolved reference to main().
libc_so = custom_target('libc_so',
  input: [basaltc_obj, setjmp_obj, core_obj, compiler_builtins_obj],
  output: 'libc.so',
  depends: [libtrona_so],
  command: [cc_cmd,
    '--target=' + c_target,
    '-shared',
    '-nostdlib', '-nostartfiles',
    '-fno-stack-protector',
    '-Wl,-soname,libc.so',
    '-T', basaltc_link,
    '-L', libtrona_build_dir,
    '-o', '@OUTPUT@',
    '@INPUT@',
    '-ltrona',
  ],
  build_by_default: true,
)

basaltc_inc = include_directories('include')
basaltc_inc_dir = meson.current_source_dir() / 'include'
basaltc_build_dir = meson.current_build_dir()

# Collect all C headers for port dependency tracking.
# Ports compile with -isystem pointing here; when headers change, ports
# must rebuild.  Mirrors the kernel's find-based .S discovery.
_basaltc_hdr_list = run_command(
  'find', 'include', '-name', '*.h',
  capture: true, check: true,
).stdout().strip()

basaltc_header_files = []
if _basaltc_hdr_list != ''
  foreach h : _basaltc_hdr_list.split('\n')
    basaltc_header_files += files(h)
  endforeach
endif
