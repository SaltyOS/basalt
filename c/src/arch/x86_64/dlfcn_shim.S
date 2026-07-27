// SPDX-License-Identifier: GPL-2.0-only
// dlsym / dlfunc caller-PC capture trampolines for RTLD_NEXT.
//
// SysV AMD64 ABI: arguments in %rdi, %rsi, %rdx, %rcx, %r8, %r9.
// At function entry the call instruction has already pushed the return
// address; (%rsp) holds the caller's PC. We grab it into %rdx as the
// `caller_pc` argument and tail-jmp into the Rust shim, leaving the stack
// untouched so the eventual `ret` returns to the original caller.

    .text
    .globl __dlsym_capture_pc
    .type __dlsym_capture_pc, @function
__dlsym_capture_pc:
    movq (%rsp), %rdx
    jmp __basaltc_dlsym_with_pc
    .size __dlsym_capture_pc, .-__dlsym_capture_pc

    .globl __dlfunc_capture_pc
    .type __dlfunc_capture_pc, @function
__dlfunc_capture_pc:
    movq (%rsp), %rdx
    jmp __basaltc_dlfunc_with_pc
    .size __dlfunc_capture_pc, .-__dlfunc_capture_pc

    .section .note.GNU-stack, "", @progbits
