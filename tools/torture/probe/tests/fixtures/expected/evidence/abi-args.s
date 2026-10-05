# Synthetic emitted-assembly evidence (NOT target evidence).
	.arch armv8-a
	.file	"abi-args.c"
	.text
	.align	2
	.global	abi_gp_ten
	.type	abi_gp_ten, %function
abi_gp_ten:
	add	x0, x0, x1
	add	x0, x0, x2
	add	x0, x0, x3
	add	x0, x0, x4
	add	x0, x0, x5
	add	x0, x0, x6
	add	x0, x0, x7
	ldr	x1, [sp]
	add	x0, x0, x1
	ldr	x1, [sp, 8]
	add	x0, x0, x1
	ret
	.size	abi_gp_ten, .-abi_gp_ten
# source under <WORKDIR>/build/abi-args.c
	.ident	"GCC: (Ubuntu 14.2.0-4ubuntu2~24.04) 14.2.0"
