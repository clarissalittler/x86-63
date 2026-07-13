	.section .data
num:	.quad 200

	.section .text
	.global _start

	# rip-relative addressing
	# this is a way of calculating
	# the position of the memory you're accessing by where it will be with respect to
	# the instruction pointer

_start:
	mov num(%rip),%rbx
	add $10,%rbx
	mov %rbx,num(%rip)
	mov num(%rip),%rdi
	mov $60,%rax
	syscall
