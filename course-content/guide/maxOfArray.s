        .section .data
arr:    .quad 7, 3, 11, 4, 9
arrlen = (. - arr) / 8          # number of elements: 5

        .section .text
        .global _start

_start:
        lea arr(%rip), %rbx     # base address
        mov (%rbx), %rax        # max = arr[0]
        mov $1, %rcx            # index i = 1
loop:
        cmp $arrlen, %rcx
        jge done
        mov (%rbx, %rcx, 8), %rdx   # %rdx = arr[i]
        cmp %rax, %rdx          # compare current max with arr[i]
        jle skip                # if arr[i] <= max, skip
        mov %rdx, %rax          # new max
skip:
        inc %rcx
        jmp loop
done:
        mov %rax, %rdi
        mov $60, %rax
        syscall
