/* basaltc CRT startup
 * SPDX-License-Identifier: GPL-2.0-only
 *
 * _start is called by the dynamic linker (rtld) or directly for static binaries.
 * The initial stack pointer holds: argc, argv[], envp[], auxv[]
 *
 * Passes main(), the initial stack pointer, and the executable's
 * .preinit_array/.init_array/.fini_array bounds to __libc_start_main.
 */

.section .text
.global _start
.type _start, @function

_start:
    xorl    %ebp, %ebp          /* Mark deepest stack frame */
    movq    %rsp, %rsi          /* arg2 = initial stack pointer (argc at [rsp]) */
    andq    $-16, %rsp          /* Align stack to 16 bytes */
    subq    $16, %rsp           /* Reserve stack slots for args 7 and 8 */
    leaq    main(%rip), %rdi    /* arg1 = pointer to main */
    leaq    __preinit_array_start(%rip), %rdx  /* arg3 */
    leaq    __preinit_array_end(%rip), %rcx    /* arg4 */
    leaq    __init_array_start(%rip), %r8      /* arg5 */
    leaq    __init_array_end(%rip), %r9        /* arg6 */
    leaq    __fini_array_start(%rip), %rax
    movq    %rax, (%rsp)        /* arg7 */
    leaq    __fini_array_end(%rip), %rax
    movq    %rax, 8(%rsp)       /* arg8 */
    call    __libc_start_main   /* Never returns */
    ud2
