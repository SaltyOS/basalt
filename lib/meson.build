# libbesalt.so - SaltyOS System Call Library (Rust)
# SPDX-License-Identifier: GPL-2.0-only

libbesalt_link = meson.current_source_dir() / 'arch' / arch / 'libsalty.ld'
libbesalt_build_dir = meson.current_build_dir()
rust_build_dir = meson.project_build_root() / 'rust'

# Common rustc flags for userland (PIC, small code model)
rust_userland_args = [
  '--edition=2024',
  '--target=' + rust_target,
  '-C', 'panic=abort',
  '-C', 'opt-level=2',
  '-C', 'code-model=small',
  '-C', 'relocation-model=pic',
  '--sysroot=/dev/null',
  '--extern', 'core=' + rust_build_dir / 'libcore.rmeta',
  '--extern', 'compiler_builtins=' + rust_build_dir / 'libcompiler_builtins.rmeta',
  '-L', rust_build_dir,
]

# SSE2-accelerated saltyc routines — auto-enabled on x86_64 (ISA baseline).
if arch == 'x86_64'
  rust_userland_args += ['--cfg=saltyc_sse2',
    '-C', 'target-feature=+sse,+sse2,-soft-float']
endif

# NEON/FPU — auto-enabled on aarch64 (NEON is mandatory in ARMv8-A).
if arch == 'aarch64'
  rust_userland_args += ['--cfg=saltyc_neon']
endif

# fork.S compile-time feature flags
fork_asm_args = []
if arch == 'x86_64'
  fork_asm_args += ['-DSALTY_X86_SIMD=1']
endif

# Step 1: Rust -> .o + .rmeta
libbesalt_rust = custom_target('libbesalt_rust',
  input: files('src/lib.rs'),
  output: ['libbesalt.o', 'libbesalt.rmeta'],
  depfile: 'libbesalt.d',
  command: [rustc,
    rust_userland_args,
    '--emit=obj=@OUTPUT0@,metadata=@OUTPUT1@,dep-info=@DEPFILE@',
    '--crate-name=salty',
    '--crate-type=lib',
    '@INPUT@',
  ],
  depends: [core, compiler_builtins],
  build_by_default: true,
)

libbesalt_obj = libbesalt_rust[0]
libbesalt_rmeta = libbesalt_rust[1]

# Step 2: fork_<arch>.S -> fork.o
fork_obj = custom_target('fork_asm',
  input: files('src/arch' / arch / 'fork.S'),
  output: 'fork.o',
  command: [cc_cmd, '--target=' + c_target, fork_asm_args, '-c', '@INPUT@', '-o', '@OUTPUT@'],
  build_by_default: true,
)

# Step 3: libbesalt.o + fork.o + core.o -> libbesalt.so
libbesalt_so = custom_target('libbesalt_so',
  input: [libbesalt_obj, fork_obj, core_obj, compiler_builtins_obj],
  output: 'libbesalt.so',
  command: [cc_cmd,
    '--target=' + c_target,
    '-shared',
    '-nostdlib', '-nostartfiles',
    '-fno-stack-protector',
    '-Wl,-soname,libbesalt.so',
    '-T', libbesalt_link,
    '-o', '@OUTPUT@',
    '@INPUT@',
  ],
  build_by_default: true,
)
