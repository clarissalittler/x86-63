	.section .data
arr:	.quad 1,2,3,4

	.section .text
	.global _start
	.extern writeInt

_start:
	mov $0,%rcx
	mov $0,%rax
	lea arr(%rip),%r15
compare:
	cmp $4,%rcx
	jge done
loop:
	add (%r15,%rcx,8),%rax # rax += *(%r15 + %rcx)
	inc %rcx # rcx++
	jmp compare
done:
	mov %rax,%rdi # put accumulator into writeInt
	call writeInt

	mov $60,%rax
	xor %rdi,%rdi
	syscall
