	.section .text
	.global _start
	.extern writeInt

_start:
	# The intended output is 321, but writeInt may clobber %rcx.
	mov $3,%rcx
.Lprint_loop:
	mov %rcx,%rdi
	call writeInt
	dec %rcx
	cmp $0,%rcx
	jg .Lprint_loop

	xor %rdi,%rdi
	mov $60,%rax
	syscall
