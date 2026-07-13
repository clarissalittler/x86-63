	.section .text
	.global _start

_start:
	# use %rbx as accumulator
	# use %rcx as our counter
	mov $0,%rbx
	mov $10,%rcx
loopStart:
	add %rcx,%rbx
	sub $1,%rcx
	cmp $0,%rcx
	jg loopStart

	mov %rbx,%rdi
	mov $60,%rax
	syscall
