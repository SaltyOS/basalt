# besalt-uapi — shared userspace/kernel contract surface
# SPDX-License-Identifier: GPL-2.0-only

besalt_uapi_inc = include_directories('include')
besalt_uapi_inc_dir = meson.current_source_dir() / 'include'
besalt_uapi_rust_consts = meson.current_source_dir() / 'rust' / 'consts.rs'

_besalt_uapi_hdr_list = run_command(
  'find', 'include', '-name', '*.h',
  capture: true, check: true,
).stdout().strip()

besalt_uapi_header_files = []
if _besalt_uapi_hdr_list != ''
  foreach h : _besalt_uapi_hdr_list.split('\n')
    besalt_uapi_header_files += files(h)
  endforeach
endif
