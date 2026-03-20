/* saltyc CRT startup (AArch64)
 * SPDX-License-Identifier: GPL-2.0-only
 *
 * _start is called by the dynamic linker (rtld) or directly for static binaries.
 * The initial stack pointer holds: argc, argv[], envp[], auxv[]
 *
 * Passes main(), the initial stack pointer, and the executable's
 * .preinit_array/.init_array/.fini_array bounds to __libc_start_main.
 *
 * __libc_start_main(main, sp, preinit_start, preinit_end,
 *                   init_start, init_end, fini_start, fini_end)
 */

.section .text
.global _start
.type _start, @function
.p2align 2

_start:
    /* Mark deepest stack frame */
    mov     x29, #0
    mov     x30, #0

    /* arg1 = pointer to main */
    adrp    x0, main
    add     x0, x0, :lo12:main

    /* arg2 = initial stack pointer (argc at [sp]) */
    mov     x1, sp

    /* Align stack to 16 bytes */
    and     sp, x1, #~0xF

    /* arg3 = __preinit_array_start */
    adrp    x2, __preinit_array_start
    add     x2, x2, :lo12:__preinit_array_start

    /* arg4 = __preinit_array_end */
    adrp    x3, __preinit_array_end
    add     x3, x3, :lo12:__preinit_array_end

    /* arg5 = __init_array_start */
    adrp    x4, __init_array_start
    add     x4, x4, :lo12:__init_array_start

    /* arg6 = __init_array_end */
    adrp    x5, __init_array_end
    add     x5, x5, :lo12:__init_array_end

    /* arg7 = __fini_array_start */
    adrp    x6, __fini_array_start
    add     x6, x6, :lo12:__fini_array_start

    /* arg8 = __fini_array_end */
    adrp    x7, __fini_array_end
    add     x7, x7, :lo12:__fini_array_end

    bl      __libc_start_main   /* Never returns */
    brk     #1                  /* Trap if it does */
