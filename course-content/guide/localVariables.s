        .text
        .global _start

# diffsq(a, b) = (a + b)^2 - (a - b)^2   (which equals 4*a*b)
# a in %rdi, b in %rsi, returns in %rax.
diffsq:
        push %rbp
        mov %rsp, %rbp
        sub $16, %rsp           # two 8-byte locals, keeps %rsp aligned

        # local at -8(%rbp) = (a + b)^2
        mov %rdi, %rax
        add %rsi, %rax
        imul %rax, %rax
        mov %rax, -8(%rbp)

        # local at -16(%rbp) = (a - b)^2
        mov %rdi, %rax
        sub %rsi, %rax
        imul %rax, %rax
        mov %rax, -16(%rbp)

        # return local[0] - local[1]
        mov -8(%rbp), %rax
        sub -16(%rbp), %rax

        leave
        ret

_start:
        mov $3, %rdi
        mov $5, %rsi
        call diffsq
        mov %rax, %rdi          # expect 4 * 3 * 5 = 60
        mov $60, %rax
        syscall
