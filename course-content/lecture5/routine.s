        .section .text
        .global _start

        # A jump-based routine needs a hard-coded jump back.
rdadder:
        add %rdi,%rdi
        mov %rdi,%rax
        jmp rdret

_start:
        mov $20,%rdi
        jmp rdadder
rdret:
        mov %rax,%rdi
        mov $60,%rax
        syscall
