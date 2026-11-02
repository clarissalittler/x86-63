        .text
        .global _start

# abs(x): returns |x|. x in %rdi, result in %rax.
abs:
        mov %rdi, %rax
        cmp $0, %rax
        jge abs_done
        neg %rax
abs_done:
        ret

_start:
        mov $-42, %rdi
        call abs
        mov %rax, %rdi          # expect 42
        mov $60, %rax
        syscall
