.intel_syntax noprefix
.section .text

.global basaltc_x86_memcpy_sse2
.type basaltc_x86_memcpy_sse2, @function
basaltc_x86_memcpy_sse2:
	mov	rax, rdi
	xor	rcx, rcx
1:
	lea	r8, [rcx + 16]
	cmp	r8, rdx
	ja	2f
	movdqu	xmm0, xmmword ptr [rsi + rcx]
	movdqu	xmmword ptr [rdi + rcx], xmm0
	add	rcx, 16
	jmp	1b
2:
	lea	r8, [rcx + 8]
	cmp	r8, rdx
	ja	3f
	mov	r9, qword ptr [rsi + rcx]
	mov	qword ptr [rdi + rcx], r9
	add	rcx, 8
3:
	cmp	rcx, rdx
	jae	5f
4:
	mov	r8b, byte ptr [rsi + rcx]
	mov	byte ptr [rdi + rcx], r8b
	inc	rcx
	cmp	rcx, rdx
	jb	4b
5:
	ret
.size basaltc_x86_memcpy_sse2, . - basaltc_x86_memcpy_sse2

.global basaltc_x86_memset_sse2
.type basaltc_x86_memset_sse2, @function
basaltc_x86_memset_sse2:
	mov	rax, rdi
	movzx	r8d, sil
	movd	xmm0, r8d
	punpcklbw	xmm0, xmm0
	pshuflw	xmm0, xmm0, 0
	pshufd	xmm0, xmm0, 0
	mov	rcx, 0
1:
	lea	r9, [rcx + 16]
	cmp	r9, rdx
	ja	2f
	movdqu	xmmword ptr [rdi + rcx], xmm0
	add	rcx, 16
	jmp	1b
2:
	cmp	rcx, rdx
	jae	4f
3:
	mov	byte ptr [rdi + rcx], r8b
	inc	rcx
	cmp	rcx, rdx
	jb	3b
4:
	ret
.size basaltc_x86_memset_sse2, . - basaltc_x86_memset_sse2

.global basaltc_x86_memmove_sse2
.type basaltc_x86_memmove_sse2, @function
basaltc_x86_memmove_sse2:
	mov	rax, rdi
	cmp	rdx, 0
	je	5f
	cmp	rdi, rsi
	jb	basaltc_x86_memcpy_sse2
	lea	r8, [rsi + rdx]
	cmp	rdi, r8
	jae	basaltc_x86_memcpy_sse2
	mov	rcx, rdx
1:
	cmp	rcx, 16
	jb	2f
	sub	rcx, 16
	movdqu	xmm0, xmmword ptr [rsi + rcx]
	movdqu	xmmword ptr [rdi + rcx], xmm0
	jmp	1b
2:
	cmp	rcx, 0
	je	5f
3:
	dec	rcx
	mov	r8b, byte ptr [rsi + rcx]
	mov	byte ptr [rdi + rcx], r8b
	cmp	rcx, 0
	jne	3b
5:
	ret
.size basaltc_x86_memmove_sse2, . - basaltc_x86_memmove_sse2

.global basaltc_x86_memcmp_sse2
.type basaltc_x86_memcmp_sse2, @function
basaltc_x86_memcmp_sse2:
	xor	rcx, rcx
1:
	lea	r8, [rcx + 16]
	cmp	r8, rdx
	ja	3f
	movdqu	xmm0, xmmword ptr [rdi + rcx]
	movdqu	xmm1, xmmword ptr [rsi + rcx]
	pcmpeqb	xmm0, xmm1
	pmovmskb	eax, xmm0
	cmp	eax, 0xffff
	jne	2f
	add	rcx, 16
	jmp	1b
2:
	not	eax
	and	eax, 0xffff
	bsf	eax, eax
	add	rcx, rax
	movzx	eax, byte ptr [rdi + rcx]
	movzx	r8d, byte ptr [rsi + rcx]
	sub	eax, r8d
	ret
3:
	cmp	rcx, rdx
	jae	5f
4:
	movzx	eax, byte ptr [rdi + rcx]
	movzx	r8d, byte ptr [rsi + rcx]
	cmp	eax, r8d
	jne	6f
	inc	rcx
	cmp	rcx, rdx
	jb	4b
5:
	xor	eax, eax
	ret
6:
	sub	eax, r8d
	ret
.size basaltc_x86_memcmp_sse2, . - basaltc_x86_memcmp_sse2

.global basaltc_x86_memchr_sse2
.type basaltc_x86_memchr_sse2, @function
basaltc_x86_memchr_sse2:
	movzx	eax, sil
	movd	xmm1, eax
	punpcklbw	xmm1, xmm1
	pshuflw	xmm1, xmm1, 0
	pshufd	xmm1, xmm1, 0
	xor	rcx, rcx
1:
	lea	r8, [rcx + 16]
	cmp	r8, rdx
	ja	3f
	movdqu	xmm0, xmmword ptr [rdi + rcx]
	pcmpeqb	xmm0, xmm1
	pmovmskb	eax, xmm0
	test	eax, eax
	jnz	2f
	add	rcx, 16
	jmp	1b
2:
	bsf	eax, eax
	add	rax, rcx
	add	rax, rdi
	ret
3:
	cmp	rcx, rdx
	jae	5f
	movzx	r8d, sil
4:
	cmp	byte ptr [rdi + rcx], r8b
	je	6f
	inc	rcx
	cmp	rcx, rdx
	jb	4b
5:
	xor	eax, eax
	ret
6:
	lea	rax, [rdi + rcx]
	ret
.size basaltc_x86_memchr_sse2, . - basaltc_x86_memchr_sse2
