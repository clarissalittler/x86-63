        .section .text
        .global _start

rdadder:
        mov %rdi,%r9
        add %r9,%r9
        mov %r9,%rax
        ret

_start:
        mov $20,%rdi
        call rdadder
        call rdadder
        mov %rax,%rdi
        mov $60,%rax
        syscall
