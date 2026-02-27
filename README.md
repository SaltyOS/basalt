# besalt

`besalt` is the SaltyOS runtime monorepo. It contains the userspace ABI
contract, the low-level runtime library, and the C runtime layer in one place.

## Components

1. `uapi/`
   - Kernel-userspace contract surface.
   - Syscall numbers, invoke labels, error codes, shared constants.
   - C-facing ABI headers under `uapi/include/besalt/`.

2. `lib/`
   - Runtime/system library layer (`libsalty.so` output).
   - IPC wrappers, syscall/invoke glue, POSIX delegation helpers, loaders.

3. `c/`
   - C runtime and libc compatibility layer (`libc.so` output).
   - CRT objects (`crt_start.o`), C/POSIX headers, stdio/stdlib/string/unistd APIs.

## Repository Layout

```text
.
  meson.build
  LICENSE.md
  README.md
  uapi/
  lib/
  c/
```

## Build Integration

- Meson entry: `meson.build`
- Parent project includes this directory from top-level `meson.build`.
- Build order is:
  1. `uapi`
  2. `lib`
  3. `c`

## Produced Artifacts

- `besalt/lib/libsalty.so`
- `besalt/lib/libsalty.rmeta`
- `besalt/c/libc.so`
- `besalt/c/crt_start.o`
- `besalt/c/saltyc.rmeta`
- headers from `besalt/c/include/` and `besalt/uapi/include/`

## Dependency Direction

`uapi` -> shared contract only  
`lib` -> depends on `uapi` contract  
`c` -> depends on `lib` (+ `uapi` constants/headers)

Keep this direction strict to avoid cyclic ABI coupling.

## Licensing

- `LICENSE.md` contains the license for this monorepo tree.
