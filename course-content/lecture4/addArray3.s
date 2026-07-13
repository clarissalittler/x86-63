	.section .data
num:	.quad 200,300,400,500

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
	mov $1,%rcx
	addq $10,(%rbx,%rcx,8) # parentheses dereference
	movq (%rbx,%rcx,8),%rdi
	mov $60,%rax
	syscall
