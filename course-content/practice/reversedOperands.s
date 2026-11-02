	.section .text
	.global _start

_start:
	# AT&T syntax puts the source before the destination.
	mov %rax,$60
	xor %rdi,%rdi
	syscall
