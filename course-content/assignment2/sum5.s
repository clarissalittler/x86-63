        ## sum5.s -- read 5 ints, sum them, print the result
        ## build: as sum5.s -o sum5.o
        ##        as assembly-examples/05-io/02-read-integer.s -o readInt.o
        ##        as assembly-examples/05-io/03-write-integer.s -o writeInt.o
        ##        ld -o sum5 sum5.o readInt.o writeInt.o

        .extern readInt
        .extern writeInt

        .section .data
nums:   .quad 0,0,0,0,0         # global array of 5 quadwords

        .section .rodata
prompt: .ascii "Enter a number: "
prompt_len = . - prompt
nl:     .ascii "\n"

        .section .text
        .global _start

        ## read_five: prompt for and read 5 integers into the global 'nums' array.
        ## We use %rbx as the loop counter because it's callee-saved, which means
        ## readInt won't clobber it across the call. We push %rbx once on entry
        ## both to preserve it for our caller AND to keep the stack 16-byte
        ## aligned before we call readInt.
read_five:
        push %rbx
        mov $0,%rbx             # i = 0
.Lread_loop:
        cmp $5,%rbx
        jge .Lread_done
        mov $1,%rax             # sys_write
        mov $1,%rdi             # stdout
        lea prompt(%rip),%rsi
        mov $prompt_len,%rdx
        syscall
        call readInt            # result in %rax
        lea nums(%rip),%rcx     # base of the array
        mov %rax,(%rcx,%rbx,8)  # nums[i] = result
        inc %rbx                # i++
        jmp .Lread_loop
.Lread_done:
        pop %rbx
        ret

        ## sum_array: add up all 5 elements of 'nums'. Returns total in %rax.
        ## No callee-saved registers clobbered, no nested calls, so we don't
        ## have to touch the stack at all.
sum_array:
        lea nums(%rip),%rdx     # base address
        mov $0,%rcx             # i = 0
        mov $0,%rax             # accumulator
.Lsum_loop:
        cmp $5,%rcx
        jge .Lsum_done
        add (%rdx,%rcx,8),%rax  # acc += nums[i]
        inc %rcx
        jmp .Lsum_loop
.Lsum_done:
        ret

_start:
        call read_five          # fill nums[] from stdin
        call sum_array          # %rax = sum of nums[]

        mov %rax,%rdi           # writeInt takes its arg in %rdi
        call writeInt

        ## print a trailing newline so the terminal prompt doesn't end up
        ## stuck on the same line as our number
        mov $1,%rax             # sys_write
        mov $1,%rdi             # stdout
        lea nl(%rip),%rsi
        mov $1,%rdx
        syscall

        ## exit cleanly
        xor %rdi,%rdi
        mov $60,%rax
        syscall
