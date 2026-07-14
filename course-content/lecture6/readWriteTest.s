	.section .text
	.global _start
	.extern readInt
	.extern writeInt

_start:
	call readInt
	mov %rax,%rdi
	call writeInt

	mov $60,%rax
	xor %rdi,%rdi
	syscall
