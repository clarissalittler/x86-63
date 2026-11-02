        .text
        .global _start
        .extern readInt
        .extern writeInt

_start:
        call readInt            # N -> %rax
        mov %rax, %rcx          # %rcx = counter, starts at N
        xor %rax, %rax          # %rax = running sum
sum_loop:
        test %rcx, %rcx
        jz   sum_done           # counter hit 0, done
        add %rcx, %rax          # sum += counter
        dec %rcx                # counter--
        jmp sum_loop
sum_done:
        mov %rax, %rdi
        call writeInt

        xor %rdi, %rdi
        mov $60, %rax
        syscall
