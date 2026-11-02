	.section .data
score:	.long 255
neighbor:	.long 1

	.section .text
	.global _start

_start:
	# This reads both adjacent .long values as one quadword.
	movq score(%rip),%rdi
	mov $60,%rax
	syscall
