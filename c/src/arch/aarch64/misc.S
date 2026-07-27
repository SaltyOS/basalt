.section .text

.p2align 2
.global basaltc_abort_trap
.type basaltc_abort_trap, %function
basaltc_abort_trap:
	udf	#0
	b	basaltc_abort_trap
.size basaltc_abort_trap, . - basaltc_abort_trap
