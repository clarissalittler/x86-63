	.section .rodata
message: .ascii "Enter a number: "

	.section .text
	.global _start
	.extern readInt
	.extern writeInt

	# factorial gets an argument in rdi
	# if that argument is <=1, we return 1
	# otherwise we call fact recursively
	# and we use imul to multiply
	# the result of the recursive call
	# by our %rdi
	#
fact:
	push %rbp # stack is NOT 16 aligned until after we save the base pointer
	mov %rsp,%rbp
	# make local variables
	# we're going to set aside more
	# space than we need
	# for alignment purposes
	sub $16,%rsp

	cmp $1,%rdi
	jle basecase

	# in the non-base-case
	# we need to save our %rdi into
	# our local storage -8(%rbp)
	# then subtract 1 from %rdi
	# then call fact again
	# after the call we multiply
	# %rax with -8(%rbp)
	# then jump to leave
	mov %rdi,-8(%rbp)
	dec %rdi
	call fact
	# at this point the result in is %rax
	mov -8(%rbp),%rdi
	imul %rdi,%rax
	jmp leavefact
basecase:
	mov $1,%rax
leavefact:
        # mov %rbp,%rsp
        # pop %rbp
	leave
	ret

_start:
	# print prompt
	mov $1,%rax
	mov $1,%rdi
	lea message(%rip),%rsi
	mov $16,%rdx
	syscall

	call readInt
	mov %rax,%rdi
	call fact
	mov %rax,%rdi
	call writeInt

	mov $60,%rax
	xor %rdi,%rdi
	syscall
