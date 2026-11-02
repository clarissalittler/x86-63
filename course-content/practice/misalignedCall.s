	.section .text
	.global _start
	.extern writeInt

_start:
	# push leaves %rsp eight bytes away from call-ready alignment.
	pushq $42
	movq (%rsp),%rdi
	call writeInt
	popq %rbx

	xor %rdi,%rdi
	mov $60,%rax
	syscall
