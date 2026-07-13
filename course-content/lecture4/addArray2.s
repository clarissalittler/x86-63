	.section .data
num:	.quad 100,200,300,400

	.section .text
	.global _start

	# rip-relative addressing
	# this is a way of calculating
	# the position of the memory you're accessing by where it will be with respect to
	# the instruction pointer

_start:
	lea num(%rip),%rbx # lea is the equivalent of the & operator
	# this means I've just loaded the pointer
	# into %rbx instead of just the value
	addq $8,%rbx
	addq $10,(%rbx) # parentheses dereference
	movq (%rbx),%rdi
	mov $60,%rax
	syscall
