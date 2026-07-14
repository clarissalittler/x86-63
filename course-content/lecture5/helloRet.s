        .section .data
msg:    .asciz "Hello world!\n"
hellolen = . - msg

        .section .text
        .global _start

_start:
        mov $1,%rax
        mov $1,%rdi
        lea msg(%rip),%rsi
        mov $hellolen,%rdx
        syscall

        # write returns the number of bytes written in %rax
        mov %rax,%rdi
        mov $60,%rax
        syscall
