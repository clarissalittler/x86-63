        .section .text
        .global writeInt

    # Function: writeInt
    # -------------------
    # Writes the integer value in %rdi to the standard output.
    #
    # This function converts the integer to its ASCII string representation
    # and outputs it using the sys_write system call.
    #
    # Inputs:
    #   %rdi - The integer to be printed.
    #
    # Clobbers (caller-saved, so no need to preserve):
    #   %rax, %rcx, %rdx, %rsi, %r8
    #
    # Preserves (callee-saved, saved/restored in the prologue/epilogue):
    #   %rbx, %rbp
    #
    # Stack layout (below %rbp):
    #   -32 .. -1   : 32-byte buffer for the string representation
    #   -40         : saved %rbx
    #   -48         : padding (keeps %rsp 16-byte aligned)
    #
    # Notes:
    #   - The function handles negative integers, including LONG_MIN (-2^63),
    #     which cannot be negated in two's complement representation.
    #   - The integer is converted to a string by repeatedly dividing by 10
    #     and storing the digits in reverse order in the buffer.

writeInt:
        # Function prologue: set up the stack frame
        pushq   %rbp                # Save the caller's base pointer on the stack
        movq    %rsp, %rbp          # Set base pointer (%rbp) to current stack pointer (%rsp)
        subq    $48, %rsp           # 32-byte buffer + 8 for saved %rbx + 8 pad (keeps %rsp aligned)
        movq    %rbx, -40(%rbp)     # Save callee-saved %rbx (we use it as the divisor below)

        # Variables:
        # We will use a buffer of 32 bytes on the stack to store the string representation
        # of the integer. The buffer extends from address (%rbp - 32) to (%rbp - 1).
        # We will use %rsi as a pointer to the current position in the buffer.

        # Initialize buffer pointer to the end of the buffer (start filling from the end)
        leaq    -1(%rbp), %rsi      # Load effective address (%rbp - 1) into %rsi
                                    # %rsi now points to the last byte of the buffer
        movq    $0, %rcx            # Initialize digit count to 0
        movq    %rdi, %rax          # Move the input integer from %rdi to %rax for processing
        movq    $0, %r8             # Initialize negative flag in %r8 to 0 (0 means positive)

        # Handle the special case when the input integer is zero
        cmpq    $0, %rax            # Compare %rax with zero
        jne     check_negative      # If %rax is not zero, skip to check_negative
        # If %rax is zero:
        movb    $'0', (%rsi)        # Store ASCII character '0' into the buffer at (%rsi)
        movq    $1, %rcx            # Set digit count to 1
        jmp     write_output        # Jump to write_output to print the character

check_negative:
        # Check if the number is negative
        testq   %rax, %rax          # Test %rax; sets flags based on the value
        jge     convert_number      # If %rax >= 0, skip to convert_number
        # If the number is negative:
        movq    $1, %r8             # Set negative flag (%r8) to 1
        negq    %rax                # Negate %rax to get the absolute value
        jno     convert_number      # If no overflow, proceed to convert_number
        # Handle LONG_MIN (-2^63), which cannot be negated
        # Note: we need movabsq here, not movq — the value 2^63 doesn't fit
        # in a 32-bit sign-extended immediate, so a plain movq won't encode it.
        movabsq $9223372036854775808, %rax  # Set %rax to 2^63 (absolute value of LONG_MIN)

convert_number:
        # Convert the integer to its string representation
convert_loop:
        xorq    %rdx, %rdx          # Clear %rdx (set to zero) before division
        movq    $10, %rbx           # Load divisor 10 into %rbx
        divq    %rbx                # Unsigned divide %rdx:%rax by %rbx
                                    # After division:
                                    # - %rax: quotient
                                    # - %rdx: remainder
        addb    $'0', %dl           # Convert remainder to ASCII character
        subq    $1, %rsi            # Move back one byte in the buffer
        movb    %dl, (%rsi)         # Store character in buffer at (%rsi)
        incq    %rcx                # Increment digit count
        cmpq    $0, %rax            # Compare quotient %rax with zero
        jne     convert_loop        # If quotient is not zero, repeat loop

        # Add negative sign if necessary
        cmpq    $0, %r8             # Check negative flag (%r8)
        je      write_output        # If number is positive, skip to write_output
        subq    $1, %rsi            # Move back one byte for '-'
        movb    $'-', (%rsi)        # Store '-' in buffer at (%rsi)
        incq    %rcx                # Increment digit count

write_output:
        # Prepare for sys_write system call
        # sys_write arguments:
        #   %rax - syscall number (1)
        #   %rdi - file descriptor (1 for stdout)
        #   %rsi - pointer to buffer (already set)
        #   %rdx - number of bytes to write
        movq    $1, %rax            # Syscall number for sys_write
        movq    $1, %rdi            # File descriptor for stdout
        # %rsi already points to the start of the string
        movq    %rcx, %rdx          # Set %rdx to digit count (number of bytes)
        syscall                     # Make the system call

        # Function epilogue: restore callee-saved register and stack frame
        movq    -40(%rbp), %rbx     # Restore %rbx before we tear down the frame
        leave                       # Restore stack frame (movq %rbp, %rsp; popq %rbp)
        ret                         # Return from the function
