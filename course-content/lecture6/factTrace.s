        # A compact tracing harness for Lecture 6's recursive fact function.
        # It fixes the input at 5 so students can focus on frame growth first.
        .section .text
        .global _start

fact:
        push %rbp
        mov %rsp,%rbp
        sub $16,%rsp
        cmp $1,%rdi
        jle basecase
        mov %rdi,-8(%rbp)
        dec %rdi
        call fact
        mov -8(%rbp),%rdi
        imul %rdi,%rax
        jmp leavefact
basecase:
        mov $1,%rax
leavefact:
        leave
        ret

_start:
        mov $5,%rdi
        call fact
        mov %rax,%rdi
        mov $60,%rax
        syscall
