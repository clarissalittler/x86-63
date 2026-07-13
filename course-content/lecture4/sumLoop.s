	.section .text
	.global _start

_start:
	# use %rbx as accumulator
	# use %rcx as our counter
	mov $0,%rbx
	mov $1,%rcx
loopStart:
	add %rcx,%rbx
	add $1,%rcx
	cmp $10,%rcx
	jle loopStart

	mov %rbx,%rdi
	mov $60,%rax
	syscall
