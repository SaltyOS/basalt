/* saltyc setjmp/longjmp for AArch64
 * SPDX-License-Identifier: GPL-2.0-only
 *
 * jmp_buf layout (14 * 8 + 8 * 8 = 176 bytes):
 *   [0..10]  x19-x29 (callee-saved GPRs, 11 regs)
 *   [11]     x30 (LR)
 *   [12]     sp
 *   [13..20] d8-d15 (callee-saved FP/NEON, 8 regs)
 *
 * sigjmp_buf extends jmp_buf:
 *   [21]     savesigs flag
 *   [22]     saved signal mask (unused for now)
 */

.section .text

/* int setjmp(jmp_buf env) */
.global setjmp
.global _setjmp
.type setjmp, @function
.type _setjmp, @function
.p2align 2
setjmp:
_setjmp:
    stp     x19, x20, [x0, #0]
    stp     x21, x22, [x0, #16]
    stp     x23, x24, [x0, #32]
    stp     x25, x26, [x0, #48]
    stp     x27, x28, [x0, #64]
    stp     x29, x30, [x0, #80]
    mov     x1, sp
    str     x1, [x0, #96]
    stp     d8, d9, [x0, #104]
    stp     d10, d11, [x0, #120]
    stp     d12, d13, [x0, #136]
    stp     d14, d15, [x0, #152]
    mov     w0, #0
    ret

/* void longjmp(jmp_buf env, int val) */
.global longjmp
.global _longjmp
.type longjmp, @function
.type _longjmp, @function
.p2align 2
longjmp:
_longjmp:
    /* x0 = env, w1 = val — restore env BEFORE clobbering x0 */
    ldp     x19, x20, [x0, #0]
    ldp     x21, x22, [x0, #16]
    ldp     x23, x24, [x0, #32]
    ldp     x25, x26, [x0, #48]
    ldp     x27, x28, [x0, #64]
    ldp     x29, x30, [x0, #80]
    ldr     x2, [x0, #96]
    mov     sp, x2
    ldp     d8, d9, [x0, #104]
    ldp     d10, d11, [x0, #120]
    ldp     d12, d13, [x0, #136]
    ldp     d14, d15, [x0, #152]
    cmp     w1, #0
    csinc   w0, w1, wzr, ne    /* w0 = (w1 != 0) ? w1 : 1 */
    ret

/* int sigsetjmp(sigjmp_buf env, int savesigs) */
.global sigsetjmp
.global __sigsetjmp
.type sigsetjmp, @function
.type __sigsetjmp, @function
.p2align 2
sigsetjmp:
__sigsetjmp:
    stp     x19, x20, [x0, #0]
    stp     x21, x22, [x0, #16]
    stp     x23, x24, [x0, #32]
    stp     x25, x26, [x0, #48]
    stp     x27, x28, [x0, #64]
    stp     x29, x30, [x0, #80]
    mov     x2, sp
    str     x2, [x0, #96]
    stp     d8, d9, [x0, #104]
    stp     d10, d11, [x0, #120]
    stp     d12, d13, [x0, #136]
    stp     d14, d15, [x0, #152]
    str     x1, [x0, #168]         /* savesigs flag */
    str     xzr, [x0, #176]        /* saved mask = 0 */
    mov     w0, #0
    ret

/* void siglongjmp(sigjmp_buf env, int val) */
.global siglongjmp
.type siglongjmp, @function
.p2align 2
siglongjmp:
    /* Restore signal mask if savesigs was nonzero (no-op for now) */
    ldp     x19, x20, [x0, #0]
    ldp     x21, x22, [x0, #16]
    ldp     x23, x24, [x0, #32]
    ldp     x25, x26, [x0, #48]
    ldp     x27, x28, [x0, #64]
    ldp     x29, x30, [x0, #80]
    ldr     x2, [x0, #96]
    mov     sp, x2
    ldp     d8, d9, [x0, #104]
    ldp     d10, d11, [x0, #120]
    ldp     d12, d13, [x0, #136]
    ldp     d14, d15, [x0, #152]
    cmp     w1, #0
    csinc   w0, w1, wzr, ne
    ret
