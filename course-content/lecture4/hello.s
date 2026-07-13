	.section .data           # Data section (initialized, writable data)
msg:    .asciz "Hello world!\n"   # Define a null-terminated string
hellolen = . - msg         # Calculate length of the string by subtracting start address from current address

        .section .text     # Code section begins
        .globl _start      # Make _start symbol globally visible for linker

_start:
          ## call the write syscall
          ## and exit appropriately
          mov $1,%rax        # Set %rax to 1 (syscall number for write)
          mov $1,%rdi        # File descriptor: 1 is stdout (0 is stdin, 2 is stderr)
          lea msg(%rip),%rsi # Load effective address of message relative to instruction pointer
          mov $hellolen,%rdx # Move message length into %rdx (third argument for write)

          syscall            # Invoke system call to print message

          xor %rdi,%rdi      # Zero out %rdi (sets exit status to 0)
          mov $60,%rax       # Set %rax to 60 (syscall number for exit)
          syscall            # Invoke system call to exit program
