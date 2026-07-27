// SPDX-License-Identifier: GPL-2.0-only
// dlsym / dlfunc caller-PC capture trampolines for RTLD_NEXT.
//
// AAPCS64: arguments in x0..x7. The caller's return address is live in x30
// (LR). We move it into x2 as the `caller_pc` argument and tail-branch into
// the Rust shim — `b` preserves x30, so the shim's eventual `ret` returns
// to the original caller of `dlsym` / `dlfunc`.

    .text
    .globl __dlsym_capture_pc
    .type __dlsym_capture_pc, %function
__dlsym_capture_pc:
    mov x2, x30
    b __basaltc_dlsym_with_pc
    .size __dlsym_capture_pc, .-__dlsym_capture_pc

    .globl __dlfunc_capture_pc
    .type __dlfunc_capture_pc, %function
__dlfunc_capture_pc:
    mov x2, x30
    b __basaltc_dlfunc_with_pc
    .size __dlfunc_capture_pc, .-__dlfunc_capture_pc

    .section .note.GNU-stack, "", %progbits
