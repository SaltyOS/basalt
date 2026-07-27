.intel_syntax noprefix
.section .text

.global basaltc_x86_sqrt_sse2_f64
.type basaltc_x86_sqrt_sse2_f64, @function
basaltc_x86_sqrt_sse2_f64:
	sqrtsd	xmm0, xmm0
	ret
.size basaltc_x86_sqrt_sse2_f64, . - basaltc_x86_sqrt_sse2_f64

.global basaltc_x86_sqrt_sse2_f32
.type basaltc_x86_sqrt_sse2_f32, @function
basaltc_x86_sqrt_sse2_f32:
	sqrtss	xmm0, xmm0
	ret
.size basaltc_x86_sqrt_sse2_f32, . - basaltc_x86_sqrt_sse2_f32
