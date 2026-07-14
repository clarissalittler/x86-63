        .section .text
        .global _start

        # rdadder destroys its argument register while it works.
rdadder:
        add %rdi,%rdi
        mov %rdi,%rax
        ret

_start:
        mov $20,%rdi
        call rdadder
        mov %rax,%rdi
        call rdadder
        mov %rax,%rdi
        mov $60,%rax
        syscall
