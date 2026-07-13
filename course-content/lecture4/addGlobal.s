	.section .data
num:	.quad 200

	.section .text
	.global _start

_start:
	mov num,%rbx
	add $10,%rbx
	mov %rbx,num
	mov num,%rdi
	mov $60,%rax
	syscall
