        .section .text
        .global _start

        # f(x) = a + 2*x, with a = 20 and two stack locals.
fun:
        push %rbp
        mov %rsp,%rbp
        sub $16,%rsp
        mov %rdi,-8(%rbp)
        add %rdi,-8(%rbp)
        movq $20,-16(%rbp)
        movq -16(%rbp),%rax
        add -8(%rbp),%rax
        mov %rbp,%rsp
        pop %rbp
        ret

_start:
        mov $10,%rdi
        call fun
        mov %rax,%rdi
        mov $60,%rax
        syscall
