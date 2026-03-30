.text
.balign 4
_fib:
	stp	x29, x30, [sp, -32]!
	mov	x29, sp
	str	x19, [x29, 24]
	cmp	w0, #0
	cset	w1, eq
	cmp	w0, #1
	cset	w2, eq
	orr	w1, w1, w2
	cmp	w1, #0
	bne	L2
	mov	w19, w0
	mov	w0, #1
	sub	w0, w19, w0
	bl	_fib
	mov	w18, w0
	mov	w0, w19
	mov	w19, w18
	mov	w1, #2
	sub	w0, w0, w1
	bl	_fib
	add	w0, w19, w0
L2:
	ldr	x19, [x29, 24]
	ldp	x29, x30, [sp], 32
	ret
/* end function fib */

.text
.balign 4
.globl _main
_main:
	stp	x29, x30, [sp, -16]!
	mov	x29, sp
	mov	w0, #30
	bl	_fib
	mov	x1, #16
	sub	sp, sp, x1
	mov	x1, #0
	add	x1, sp, x1
	str	w0, [x1]
	adrp	x0, _dat_0@page
	add	x0, x0, _dat_0@pageoff
	bl	_printf
	mov	x0, #16
	add	sp, sp, x0
	mov	w0, #0
	ldp	x29, x30, [sp], 16
	ret
/* end function main */

.data
.balign 8
_dat_0:
	.ascii "Result is %d"
	.byte 0
/* end data */

