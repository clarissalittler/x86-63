	.section .text
	.global _start

_start:
	mov $10,%rbx
	mov $20,%rcx
	sub %rbx,%rcx
	mov %rcx,%rdi
	mov $60,%rax
	syscall
