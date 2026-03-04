# Besalt runtime monorepo (uapi/lib/c/cpp)
# SPDX-License-Identifier: GPL-2.0-only

subdir('uapi')
subdir('lib')
subdir('c')

# libc++.so: build from toolchain/llvm-project sources when available.
# auto  = build if toolchain/llvm-project/libcxx/src/ exists, skip otherwise
# true  = require source tree (error if missing)
# false = always skip
_libcxx_opt = get_option('build_libcxx')
_libcxx_src_dir = meson.project_source_root() / 'toolchain' / 'llvm-project' / 'libcxx' / 'src'
_build_libcxx = (_libcxx_opt == 'true') or (_libcxx_opt == 'auto' and fs.is_dir(_libcxx_src_dir))
if _libcxx_opt == 'true' and not fs.is_dir(_libcxx_src_dir)
  error('build_libcxx=true but toolchain/llvm-project/libcxx/src/ not found. ' +
        'Run: git submodule update --init toolchain/llvm-project')
endif
if _build_libcxx
  subdir('cpp')
endif
