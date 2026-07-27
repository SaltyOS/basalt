.intel_syntax noprefix
.section .text

.macro STORE_XMM0
	sub	rsp, 24
	movsd	qword ptr [rsp + 8], xmm0
.endm

.macro LOAD_XMM0_RET
	movsd	xmm0, qword ptr [rsp + 8]
	add	rsp, 24
	ret
.endm

.global basaltc_x86_x87_round_mode
.type basaltc_x86_x87_round_mode, @function
basaltc_x86_x87_round_mode:
	STORE_XMM0
	fnstcw	word ptr [rsp]
	mov	ax, word ptr [rsp]
	and	ax, 0xf3ff
	or	ax, si
	mov	word ptr [rsp + 2], ax
	fldcw	word ptr [rsp + 2]
	fld	qword ptr [rsp + 8]
	frndint
	fstp	qword ptr [rsp + 8]
	fldcw	word ptr [rsp]
	LOAD_XMM0_RET
.size basaltc_x86_x87_round_mode, . - basaltc_x86_x87_round_mode

.global basaltc_x86_x87_sqrt
.type basaltc_x86_x87_sqrt, @function
basaltc_x86_x87_sqrt:
	STORE_XMM0
	fld	qword ptr [rsp + 8]
	fsqrt
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_sqrt, . - basaltc_x86_x87_sqrt

.global basaltc_x86_x87_sin
.type basaltc_x86_x87_sin, @function
basaltc_x86_x87_sin:
	STORE_XMM0
	fld	qword ptr [rsp + 8]
	fsin
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_sin, . - basaltc_x86_x87_sin

.global basaltc_x86_x87_cos
.type basaltc_x86_x87_cos, @function
basaltc_x86_x87_cos:
	STORE_XMM0
	fld	qword ptr [rsp + 8]
	fcos
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_cos, . - basaltc_x86_x87_cos

.global basaltc_x86_x87_tan
.type basaltc_x86_x87_tan, @function
basaltc_x86_x87_tan:
	STORE_XMM0
	fld	qword ptr [rsp + 8]
	fptan
	fstp	st(0)
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_tan, . - basaltc_x86_x87_tan

.global basaltc_x86_x87_atan2
.type basaltc_x86_x87_atan2, @function
basaltc_x86_x87_atan2:
	sub	rsp, 24
	movsd	qword ptr [rsp + 8], xmm0
	movsd	qword ptr [rsp + 16], xmm1
	fld	qword ptr [rsp + 16]
	fld	qword ptr [rsp + 8]
	fpatan
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_atan2, . - basaltc_x86_x87_atan2

.global basaltc_x86_x87_log2
.type basaltc_x86_x87_log2, @function
basaltc_x86_x87_log2:
	STORE_XMM0
	fld1
	fld	qword ptr [rsp + 8]
	fyl2x
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_log2, . - basaltc_x86_x87_log2

.global basaltc_x86_x87_log
.type basaltc_x86_x87_log, @function
basaltc_x86_x87_log:
	STORE_XMM0
	fldln2
	fld	qword ptr [rsp + 8]
	fyl2x
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_log, . - basaltc_x86_x87_log

.global basaltc_x86_x87_log10
.type basaltc_x86_x87_log10, @function
basaltc_x86_x87_log10:
	STORE_XMM0
	fldlg2
	fld	qword ptr [rsp + 8]
	fyl2x
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_log10, . - basaltc_x86_x87_log10

.global basaltc_x86_x87_exp2
.type basaltc_x86_x87_exp2, @function
basaltc_x86_x87_exp2:
	STORE_XMM0
	fld	qword ptr [rsp + 8]
	fld	st(0)
	frndint
	fxch	st(1)
	fsub	st(0), st(1)
	f2xm1
	fld1
	faddp
	fscale
	fstp	st(1)
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_exp2, . - basaltc_x86_x87_exp2

.global basaltc_x86_x87_exp
.type basaltc_x86_x87_exp, @function
basaltc_x86_x87_exp:
	STORE_XMM0
	fldl2e
	fmul	qword ptr [rsp + 8]
	fld	st(0)
	frndint
	fxch	st(1)
	fsub	st(0), st(1)
	f2xm1
	fld1
	faddp
	fscale
	fstp	st(1)
	fstp	qword ptr [rsp + 8]
	LOAD_XMM0_RET
.size basaltc_x86_x87_exp, . - basaltc_x86_x87_exp

.global basaltc_x86_x87_fmod
.type basaltc_x86_x87_fmod, @function
basaltc_x86_x87_fmod:
	sub	rsp, 24
	movsd	qword ptr [rsp + 8], xmm0
	movsd	qword ptr [rsp + 16], xmm1
	fld	qword ptr [rsp + 16]
	fld	qword ptr [rsp + 8]
1:
	fprem
	fnstsw	ax
	test	ax, 0x0400
	jnz	1b
	fstp	qword ptr [rsp + 8]
	fstp	st(0)
	LOAD_XMM0_RET
.size basaltc_x86_x87_fmod, . - basaltc_x86_x87_fmod

.global basaltc_x86_x87_remainder
.type basaltc_x86_x87_remainder, @function
basaltc_x86_x87_remainder:
	sub	rsp, 24
	movsd	qword ptr [rsp + 8], xmm0
	movsd	qword ptr [rsp + 16], xmm1
	fld	qword ptr [rsp + 16]
	fld	qword ptr [rsp + 8]
1:
	fprem1
	fnstsw	ax
	test	ax, 0x0400
	jnz	1b
	fstp	qword ptr [rsp + 8]
	fstp	st(0)
	LOAD_XMM0_RET
.size basaltc_x86_x87_remainder, . - basaltc_x86_x87_remainder

.global basaltc_x86_x87_fma
.type basaltc_x86_x87_fma, @function
basaltc_x86_x87_fma:
	sub	rsp, 32
	movsd	qword ptr [rsp + 8], xmm0
	movsd	qword ptr [rsp + 16], xmm1
	movsd	qword ptr [rsp + 24], xmm2
	fld	qword ptr [rsp + 8]
	fmul	qword ptr [rsp + 16]
	fadd	qword ptr [rsp + 24]
	fstp	qword ptr [rsp + 8]
	movsd	xmm0, qword ptr [rsp + 8]
	add	rsp, 32
	ret
.size basaltc_x86_x87_fma, . - basaltc_x86_x87_fma

.global basaltc_x86_x87_scalbn
.type basaltc_x86_x87_scalbn, @function
basaltc_x86_x87_scalbn:
	sub	rsp, 24
	movsd	qword ptr [rsp + 8], xmm0
	cvtsi2sd	xmm1, edi
	movsd	qword ptr [rsp + 16], xmm1
	fld	qword ptr [rsp + 16]
	fld	qword ptr [rsp + 8]
	fscale
	fstp	qword ptr [rsp + 8]
	fstp	st(0)
	LOAD_XMM0_RET
.size basaltc_x86_x87_scalbn, . - basaltc_x86_x87_scalbn
