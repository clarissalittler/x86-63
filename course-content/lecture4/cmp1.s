	.section .text
	.global _start
	# cmp
	# jmp and friends
_start:
	mov $10,%rbx
	mov $20,%rcx
	mov $-1,%rdi
	cmp %rbx,%rcx # cmp S,D --> D - S
	jge greater
	mov $1,%rdi
greater:
	mov $60,%rax
	syscall
