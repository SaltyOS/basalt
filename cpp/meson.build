# libc++.so — C++ runtime library for SaltyOS (libcxx + libcxxabi)
# SPDX-License-Identifier: GPL-2.0-only
#
# Compiles libcxx and libcxxabi .cpp sources from toolchain/llvm-project/
# directly via Meson custom_target(). The toolchain source tree is referenced
# in-place; no vendoring is required.
#
# SaltyOS configuration (matches former toolchain-build-runtimes CMake invocation):
#   -fno-exceptions -fno-rtti
#   LIBCXX_ENABLE_FILESYSTEM=OFF  LIBCXX_ENABLE_LOCALIZATION=ON
#   LIBCXX_ENABLE_WIDE_CHARACTERS=ON  LIBCXX_ENABLE_RANDOM_DEVICE=ON
#   LIBCXX_ENABLE_UNICODE=OFF  LIBCXX_ENABLE_TIME_ZONE_DATABASE=OFF
#   LIBCXX_CXX_ABI=libcxxabi  LIBCXX_STATICALLY_LINK_ABI_IN_SHARED_LIBRARY=ON
#   LIBCXXABI_BAREMETAL=ON  LIBCXXABI_SILENT_TERMINATE=ON

_llvm_src     = meson.project_source_root() / 'toolchain' / 'llvm-project'
_libcxx_src   = _llvm_src / 'libcxx'   / 'src'
_libcxxabi_src = _llvm_src / 'libcxxabi' / 'src'
_libcxx_inc   = _llvm_src / 'libcxx'   / 'include'
_libcxxabi_inc = _llvm_src / 'libcxxabi' / 'include'
_config_site  = meson.current_source_dir() / '__config_site'

# Clang resource directory provides freestanding compiler headers
# (stddef.h, stdint.h, float.h, stdarg.h, ...) without host libc pollution.
_clang_resource_dir = run_command(
  cc_cmd[0], '--print-resource-dir', capture: true, check: true,
).stdout().strip()

# C++ compile flags: freestanding, PIC, no exceptions, no RTTI
_cxx_flags = [
  '--target=' + c_target,
  '-std=c++23',
  '-fPIC',
  '-fno-exceptions',
  '-fno-rtti',
  '-ffreestanding',
  '-nostdinc',         # block ALL host system headers (not just C++)
  '-nostdlib',
  '-D_LIBCPP_BUILDING_LIBRARY',
  '-DLIBCXX_BUILDING_LIBCXXABI',   # suppress libcxx's duplicate definitions of type_info/exception/stdexcept
  '-DLIBCXXABI_BAREMETAL',
  '-DLIBCXXABI_SILENT_TERMINATE',
  # Include search order:
  #  1. cpp/ source dir  — __config_site, __assertion_handler
  #  2. libcxx/include   — C++ headers + C wrapper stubs (stddef.h, stdio.h, ...)
  #  3. libcxxabi/include — cxxabi headers
  #  4. clang resource   — freestanding stddef.h, stdint.h, float.h, stdarg.h
  #                        (reached via libc++'s #include_next)
  #  5. besaltc/include  — C library headers (stdio.h, stdlib.h, pthread.h, ...)
  '-isystem', meson.current_source_dir(),
  '-isystem', _libcxx_inc,
  '-isystem', _libcxxabi_inc,
  '-isystem', _clang_resource_dir / 'include',
  '-isystem', besaltc_inc_dir,
  '-I', _libcxx_src,
  # charconv.cpp → from_chars_floating_point.h → shared/fp_bits.h lives in libc/
  '-I', _llvm_src / 'libc',
  '-DLIBC_NAMESPACE=__llvm_libc_common_utils',
  '-include', _config_site,
]

# libcxxabi needs RTTI — it IS the RTTI implementation (dynamic_cast, typeid).
# Upstream LLVM CMake only applies -fno-rtti to libcxx, never to libcxxabi.
_cxxabi_flags = []
foreach f : _cxx_flags
  if f != '-fno-rtti'
    _cxxabi_flags += f
  endif
endforeach

# libcxx source files
# Excluded: fstream.cpp, strstream.cpp (not needed)
#           new.cpp (replaced by libcxxabi stdlib_new_delete.cpp via static ABI link)
#           filesystem/{directory_entry,directory_iterator,operations,int128_builtins}.cpp
#             (full filesystem disabled; clock/error/path stubs are included below)
_cxx_srcs = [
  'algorithm.cpp',
  'any.cpp',
  'atomic.cpp',
  'barrier.cpp',
  'bind.cpp',
  'call_once.cpp',
  'charconv.cpp',
  'chrono.cpp',
  'condition_variable.cpp',
  'condition_variable_destructor.cpp',
  'error_category.cpp',
  'exception.cpp',
  'expected.cpp',
  'functional.cpp',
  'future.cpp',
  'hash.cpp',
  'ios.cpp',
  'ios.instantiations.cpp',
  'iostream.cpp',
  'locale.cpp',
  'memory.cpp',
  'memory_resource.cpp',
  'mutex.cpp',
  'mutex_destructor.cpp',
  'new_handler.cpp',
  'new_helpers.cpp',
  'optional.cpp',
  'ostream.cpp',
  'print.cpp',
  'random.cpp',
  'random_shuffle.cpp',
  'regex.cpp',
  'ryu/d2fixed.cpp',
  'ryu/d2s.cpp',
  'ryu/f2s.cpp',
  'filesystem/filesystem_clock.cpp',
  'filesystem/filesystem_error.cpp',
  'filesystem/path.cpp',
  'shared_mutex.cpp',
  'stdexcept.cpp',
  'string.cpp',
  'system_error.cpp',
  'thread.cpp',
  'typeinfo.cpp',
  'valarray.cpp',
  'variant.cpp',
  'vector.cpp',
  'verbose_abort.cpp',
]

# libcxxabi source files
# Excluded: cxa_exception.cpp, cxa_personality.cpp (LIBCXXABI_ENABLE_EXCEPTIONS=OFF)
_cxxabi_srcs = [
  'abort_message.cpp',
  'cxa_aux_runtime.cpp',
  'cxa_default_handlers.cpp',
  'cxa_demangle.cpp',
  'cxa_exception_storage.cpp',
  'cxa_guard.cpp',
  'cxa_handlers.cpp',
  'cxa_noexception.cpp',
  'cxa_thread_atexit.cpp',
  'cxa_vector.cpp',
  'cxa_virtual.cpp',
  'fallback_malloc.cpp',
  'private_typeinfo.cpp',
  'stdlib_exception.cpp',
  'stdlib_new_delete.cpp',
  'stdlib_stdexcept.cpp',
  'stdlib_typeinfo.cpp',
]

# Compile libcxx sources → object files
_all_objs = []
foreach src : _cxx_srcs
  _name = 'cxx_' + src.replace('/', '_').replace('.cpp', '')
  _all_objs += custom_target(_name,
    output: _name + '.o',
    depfile: _name + '.d',
    command: [cc_cmd,
      _cxx_flags,
      '-MMD', '-MF', '@DEPFILE@',
      '-c', _libcxx_src / src,
      '-o', '@OUTPUT@',
    ],
    build_by_default: true,
  )
endforeach

# Compile libcxxabi sources → object files
foreach src : _cxxabi_srcs
  _name = 'cxxabi_' + src.replace('.cpp', '')
  _all_objs += custom_target(_name,
    output: _name + '.o',
    depfile: _name + '.d',
    command: [cc_cmd,
      _cxxabi_flags,
      '-MMD', '-MF', '@DEPFILE@',
      '-c', _libcxxabi_src / src,
      '-o', '@OUTPUT@',
    ],
    build_by_default: true,
  )
endforeach

# Locate libclang_rt.builtins.a for the target so that 128-bit float builtins
# (__lttf2, __trunctfdf2, etc.) are statically linked into libc++.so.
# This mirrors Fuchsia's approach: each shared library that needs compiler-rt
# builtins links them in at build time.
_clang_rt_builtins = run_command(
  cc_cmd[0], '--target=' + c_target, '-print-libgcc-file-name',
  capture: true, check: true,
).stdout().strip()

# Link libc++.so — libcxx + libcxxabi objects + compiler-rt builtins +
# DT_NEEDED entries for libc/libbesalt
libcxx_so = custom_target('libcxx_so',
  input: _all_objs,
  output: 'libc++.so',
  command: [cc_cmd,
    '--target=' + c_target,
    '-shared',
    '-nostdlib', '-nostartfiles',
    '-fuse-ld=lld',
    '-Wl,-soname,libc++.so',
    '-z', 'max-page-size=4096',
    '-o', '@OUTPUT@',
    '@INPUT@',
    _clang_rt_builtins,
    '-L', besaltc_build_dir,
    '-lc',
    '-L', libbesalt_build_dir,
    '-lbesalt',
  ],
  depends: [libc_so, libbesalt_so],
  build_by_default: true,
)

libcxx_build_dir = meson.current_build_dir()
