        .text
        .global _start

# fact(n): returns n!. n in %rdi, result in %rax.
# Stack frame layout (below %rbp):
#   -8 : saved copy of n across the recursive call
fact:
        push %rbp
        mov %rsp, %rbp
        sub $16, %rsp           # 16-byte aligned; -8 used for saved n

        cmp $1, %rdi
        jle fact_base

        mov %rdi, -8(%rbp)      # save n in a local slot
        dec %rdi                # compute (n - 1)
        call fact               # %rax = (n - 1)!
        mov -8(%rbp), %rdi      # reload n
        imul %rdi, %rax         # %rax = n * (n - 1)! = n!

        leave
        ret

fact_base:
        mov $1, %rax            # 0! = 1! = 1
        leave
        ret

_start:
        mov $5, %rdi
        call fact
        mov %rax, %rdi          # expect 5! = 120
        mov $60, %rax
        syscall
