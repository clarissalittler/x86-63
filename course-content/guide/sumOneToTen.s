        .text
        .global _start

_start:
        xor %rax, %rax          # accumulator = 0
        mov $1, %rcx            # counter i = 1
loop:
        cmp $10, %rcx
        jg  done                # i > 10, exit loop
        add %rcx, %rax          # sum += i
        inc %rcx                # i++
        jmp loop
done:
        mov %rax, %rdi          # exit code = sum
        mov $60, %rax
        syscall
