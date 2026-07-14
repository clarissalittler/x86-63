# This is a simple program that reads input from stdin and then writes it back out

        .section .bss
buff:   .skip 128

        .section .text
        .global _start

_start:
        mov $0,%rax
        mov $0,%rdi
        lea buff(%rip),%rsi
        mov $128,%rdx
        syscall

        # read returns the number of bytes actually read in %rax
        mov %rax,%rdx
        mov $1,%rax
        lea buff(%rip),%rsi
        mov $1,%rdi
        syscall

        xor %rdi,%rdi
        mov $60,%rax
        syscall
