        .text
        .global _start

_start:
        mov $-42, %rax          # change to 0 or positive to test other branches
        cmp $0, %rax
        jg  positive
        je  zero
        # fall through: negative
        mov $-1, %rdi
        jmp done
positive:
        mov $1, %rdi
        jmp done
zero:
        mov $0, %rdi
done:
        mov $60, %rax
        syscall
