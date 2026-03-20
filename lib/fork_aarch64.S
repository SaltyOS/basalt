/* SaltyOS fork() assembly trampoline (AArch64)
 * SPDX-License-Identifier: GPL-2.0-only
 *
 * posix_fork() saves callee-saved registers on the stack, then calls
 * _posix_fork_impl() which sends PM_FORK to procmgr with the current
 * SP and the address of fork_child_entry.
 *
 * Parent path: _posix_fork_impl returns child PID.
 * Child path: procmgr sets child RIP to fork_child_entry with saved SP.
 */

.text
.global posix_fork
.global fork_child_entry
.hidden fork_child_entry

posix_fork:
    /* Save callee-saved GPRs (x19-x28, x29/FP, x30/LR) */
    stp     x29, x30, [sp, #-16]!
    stp     x27, x28, [sp, #-16]!
    stp     x25, x26, [sp, #-16]!
    stp     x23, x24, [sp, #-16]!
    stp     x21, x22, [sp, #-16]!
    stp     x19, x20, [sp, #-16]!

    /* Save callee-saved NEON/FP regs (d8-d15) */
    stp     d14, d15, [sp, #-16]!
    stp     d12, d13, [sp, #-16]!
    stp     d10, d11, [sp, #-16]!
    stp     d8,  d9,  [sp, #-16]!

    /* Call _posix_fork_impl(saved_sp, child_entry) */
    mov     x0, sp
    adrp    x1, fork_child_entry
    add     x1, x1, :lo12:fork_child_entry
    bl      _posix_fork_impl

    /* Parent: restore NEON regs */
    ldp     d8,  d9,  [sp], #16
    ldp     d10, d11, [sp], #16
    ldp     d12, d13, [sp], #16
    ldp     d14, d15, [sp], #16

    /* Parent: restore GPRs */
    ldp     x19, x20, [sp], #16
    ldp     x21, x22, [sp], #16
    ldp     x23, x24, [sp], #16
    ldp     x25, x26, [sp], #16
    ldp     x27, x28, [sp], #16
    ldp     x29, x30, [sp], #16

    /* Return child PID (already in x0 from _posix_fork_impl) */
    ret

fork_child_entry:
    /* Child starts here with SP pointing to saved state */

    /* Restore NEON regs */
    ldp     d8,  d9,  [sp], #16
    ldp     d10, d11, [sp], #16
    ldp     d12, d13, [sp], #16
    ldp     d14, d15, [sp], #16

    /* Restore GPRs */
    ldp     x19, x20, [sp], #16
    ldp     x21, x22, [sp], #16
    ldp     x23, x24, [sp], #16
    ldp     x25, x26, [sp], #16
    ldp     x27, x28, [sp], #16
    ldp     x29, x30, [sp], #16

    /* Return 0 = child */
    mov     x0, #0
    ret
