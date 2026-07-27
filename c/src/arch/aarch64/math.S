.section .text

.p2align 2
.global basaltc_aarch64_sqrt_f64
.type basaltc_aarch64_sqrt_f64, %function
basaltc_aarch64_sqrt_f64:
	fsqrt	d0, d0
	ret
.size basaltc_aarch64_sqrt_f64, . - basaltc_aarch64_sqrt_f64

.p2align 2
.global basaltc_aarch64_sqrt_f32
.type basaltc_aarch64_sqrt_f32, %function
basaltc_aarch64_sqrt_f32:
	fsqrt	s0, s0
	ret
.size basaltc_aarch64_sqrt_f32, . - basaltc_aarch64_sqrt_f32

.p2align 2
.global basaltc_aarch64_floor_f64
.type basaltc_aarch64_floor_f64, %function
basaltc_aarch64_floor_f64:
	frintm	d0, d0
	ret
.size basaltc_aarch64_floor_f64, . - basaltc_aarch64_floor_f64

.p2align 2
.global basaltc_aarch64_ceil_f64
.type basaltc_aarch64_ceil_f64, %function
basaltc_aarch64_ceil_f64:
	frintp	d0, d0
	ret
.size basaltc_aarch64_ceil_f64, . - basaltc_aarch64_ceil_f64

.p2align 2
.global basaltc_aarch64_trunc_f64
.type basaltc_aarch64_trunc_f64, %function
basaltc_aarch64_trunc_f64:
	frintz	d0, d0
	ret
.size basaltc_aarch64_trunc_f64, . - basaltc_aarch64_trunc_f64

.p2align 2
.global basaltc_aarch64_rint_f64
.type basaltc_aarch64_rint_f64, %function
basaltc_aarch64_rint_f64:
	frintx	d0, d0
	ret
.size basaltc_aarch64_rint_f64, . - basaltc_aarch64_rint_f64
