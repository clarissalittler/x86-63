	.section .text
	.global _start

_start:
	# Repair this operand so 60 is treated as a literal value.
	mov 60,%rax
	xor %rdi,%rdi
	syscall
