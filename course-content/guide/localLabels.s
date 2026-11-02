# Read-only data section (constants)
.section .rodata
    # Define first message string (null-terminated)
    msg1:   .asciz "In the first sublabel\n"
    # Calculate length of first message
    msg1len = . - msg1

    # Define second message string (null-terminated)
    msg2:   .asciz "In the second sublabel\n"
    # Calculate length of second message
    msg2len = . - msg2

# Executable code section
.section .text
    # Make _start symbol globally accessible (entry point)
    .globl _start

# Program entry point
_start:
    # Call functions in order: biglabel2 first, then biglabel1
    call biglabel2
    call biglabel1

    # Exit program gracefully
1:
    # syscall for program exit
    # rax = 60 (exit syscall)
    # rdi = 0 (exit status code)
    mov $60,%rax
    xor %rdi,%rdi
    syscall

# First function: print first message
biglabel1:
    # Local jump to next instruction (obviously for toy purposes)
    jmp 1f
1:
    # Prepare arguments for write syscall
    # rsi = message address (using RIP-relative addressing)
    # rdx = message length
    # rax = syscall number for write (1)
    # rdi = file descriptor (1 = standard output)
    lea msg1(%rip),%rsi
    mov $msg1len,%rdx
    mov $1,%rax
    mov $1,%rdi
    syscall
    ret

# Second function: print second message
biglabel2:
    # Local jump to next instruction (obviously for toy purposes)
    jmp 1f
1:
    # Same syscall setup as biglabel1, but with second message
    lea msg2(%rip),%rsi
    mov $msg2len,%rdx
    mov $1,%rax
    mov $1,%rdi
    syscall
    ret
