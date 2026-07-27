# basaltc — SaltyOS C Standard Library (Rust + ASM → libc.so)
# SPDX-License-Identifier: GPL-2.0-only

basaltc_link = meson.current_source_dir() / 'arch' / arch / 'basaltc.ld'

# Step 1: Rust -> .o + .rmeta (links against the 5 trona crates + trona_posix)
basaltc_rust = custom_target('basaltc_rust',
  input: files('src/lib.rs'),
  output: ['basaltc.o', 'basaltc.rmeta'],
  depfile: 'basaltc.d',
  command: [rustc,
    rust_userland_args,
    '--emit=obj=@OUTPUT0@,metadata=@OUTPUT1@,dep-info=@DEPFILE@',
    '--crate-name=basaltc',
    '--crate-type=lib',
    '--extern', 'uapi=' + trona_uapi_rmeta,
    '--extern', 'trona_kernel=' + trona_kernel_rmeta,
    '--extern', 'trona_protocol=' + trona_protocol_rmeta,
    '--extern', 'trona_server=' + trona_server_rmeta,
    '--extern', 'trona_runtime=' + trona_runtime_rmeta,
    '--extern', 'trona_posix=' + libtrona_build_dir / 'libtrona_posix.rmeta',
    '@INPUT@',
  ],
  depends: [core, compiler_builtins, trona_uapi, trona_kernel, trona_protocol, trona_server, trona_runtime, trona_posix],
  build_by_default: true,
)

basaltc_obj = basaltc_rust[0]

basaltc_misc_obj = custom_target('basaltc_misc_asm',
  input: files('src/arch' / arch / 'misc.S'),
  output: 'basaltc_misc.o',
  command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

basaltc_arch_asm_objs = [basaltc_misc_obj]

if arch == 'x86_64'
  basaltc_math_x87_obj = custom_target('basaltc_math_x87_asm',
    input: files('src/arch/x86_64/math_x87.S'),
    output: 'basaltc_math_x87.o',
    command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
    build_by_default: true,
  )
  basaltc_math_sse2_obj = custom_target('basaltc_math_sse2_asm',
    input: files('src/arch/x86_64/math_sse2.S'),
    output: 'basaltc_math_sse2.o',
    command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
    build_by_default: true,
  )
  basaltc_mem_sse2_obj = custom_target('basaltc_mem_sse2_asm',
    input: files('src/arch/x86_64/mem_sse2.S'),
    output: 'basaltc_mem_sse2.o',
    command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
    build_by_default: true,
  )
  basaltc_string_sse2_obj = custom_target('basaltc_string_sse2_asm',
    input: files('src/arch/x86_64/string_sse2.S'),
    output: 'basaltc_string_sse2.o',
    command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
    build_by_default: true,
  )
  basaltc_arch_asm_objs += [basaltc_math_x87_obj, basaltc_math_sse2_obj, basaltc_mem_sse2_obj, basaltc_string_sse2_obj]
elif arch == 'aarch64'
  basaltc_math_obj = custom_target('basaltc_math_asm',
    input: files('src/arch/aarch64/math.S'),
    output: 'basaltc_math.o',
    command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
    build_by_default: true,
  )
  basaltc_arch_asm_objs += [basaltc_math_obj]
endif

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

# Step 3b: dlfcn_shim_{arch}.S -> dlfcn_shim.o
# Per-arch caller-PC capture trampolines for dlsym/dlfunc; required so the
# rtld can implement RTLD_NEXT lookups against the actual caller LinkMap.
dlfcn_shim_obj = custom_target('dlfcn_shim_asm',
  input: files('src/arch' / arch / 'dlfcn_shim.S'),
  output: 'dlfcn_shim.o',
  command: [cc_cmd, '--target=' + c_target, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 4: basaltc.o + setjmp.o + dlfcn_shim.o + core.o -> libc.so
# NOTE: crt_start_obj is NOT linked into libc.so — it is a separate static
# object that C programs link directly, so that libc.so does not carry an
# unresolved reference to main().
libc_so_inputs = [basaltc_obj, setjmp_obj, dlfcn_shim_obj, core_obj, compiler_builtins_obj] + basaltc_arch_asm_objs
libc_so = custom_target('libc_so',
  input: libc_so_inputs,
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

libc_a_inputs = [basaltc_obj, setjmp_obj, dlfcn_shim_obj] + basaltc_arch_asm_objs
libc_a = custom_target('libc_a',
  input: libc_a_inputs,
  output: 'libc.a',
  command: [llvm_ar, 'rcs', '@OUTPUT@', '@INPUT@'],
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
