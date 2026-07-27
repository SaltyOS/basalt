.intel_syntax noprefix
.section .text

.global basaltc_x86_strlen_sse2
.type basaltc_x86_strlen_sse2, @function
basaltc_x86_strlen_sse2:
	mov	rax, rdi
1:
	test	rax, 15
	je	2f
	cmp	byte ptr [rax], 0
	je	4f
	inc	rax
	jmp	1b
2:
	pxor	xmm0, xmm0
3:
	movdqa	xmm1, xmmword ptr [rax]
	pcmpeqb	xmm1, xmm0
	pmovmskb	ecx, xmm1
	test	ecx, ecx
	jnz	5f
	add	rax, 16
	jmp	3b
4:
	sub	rax, rdi
	ret
5:
	bsf	ecx, ecx
	add	rax, rcx
	sub	rax, rdi
	ret
.size basaltc_x86_strlen_sse2, . - basaltc_x86_strlen_sse2
