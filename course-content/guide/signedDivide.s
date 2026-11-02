        # Signed division, following the guide's div/idiv detour.
        # idivq divides the 128-bit pair %rdx:%rax, so a negative dividend
        # must be sign-extended into %rdx with cqo, not zeroed with xor.
        .text
        .global _start

_start:
        mov $-47, %rax          # dividend
        cqo                     # %rdx = all copies of %rax's sign bit
        mov $10, %rbx           # divisor
        idivq %rbx              # %rax = -4 (quotient), %rdx = -7 (remainder)

        mov %rdx, %rdi          # exit with the remainder: -7 shows up as 249
        neg %rdi                # ...so negate it and exit with 7 instead
        mov $60, %rax
        syscall
