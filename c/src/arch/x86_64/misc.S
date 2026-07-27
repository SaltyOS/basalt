.intel_syntax noprefix
.section .text

.global basaltc_abort_trap
.type basaltc_abort_trap, @function
basaltc_abort_trap:
	ud2
1:
	hlt
	jmp	1b
.size basaltc_abort_trap, . - basaltc_abort_trap
