# The CS201 x86-64 subset

This document is the compatibility contract for `x86-63`. The target is not
"whatever x86-64 can do"; it is the coherent source-language subset students
write and reason about in CS201.

## Sources reviewed

The compatibility target was derived, in priority order, from:

1. `fall2026/lecture3` through `lecture6`, the next-term skeleton copied from
   the audited Spring 2026 course.
2. The corresponding narrative in `spring2026/lecture-notes.tex`, which
   records the intended lesson behind each live example.
3. `assignments/assignment2.md`, especially its complete `sum5.s` program.
4. `assemblyGuide.org`, the authoritative syntax, ABI, debugging, and common
   mistakes reference.
5. The non-advanced `assembly-examples` progression.

Old term directories are intentionally not compatibility sources: the course
audit marks them as archival and records known assembly bugs in several of
them. The floating-point and AVX examples are useful future extensions, but
they are not part of the current lecture 3–6 path.

## The teaching sequence

| Course step | Concepts the visualizer must reveal | Representative programs |
|---|---|---|
| Lecture 3 | program entry, registers, source/destination order, wraparound exit status, falling off `.text` | `first.s`, `firstfixed.s`, `firstadd.s`, `firstsub.s` |
| Lecture 4 | data layout, operand width, RIP-relative addresses, effective-address calculation, flags and branches, loops, `write` | `addGlobal*.s`, `addArray*.s`, `cmp1.s`, `sumLoop*.s`, `hello.s` |
| Lecture 5 | blocking input, returned byte counts, jump-based routines versus calls, return addresses, stack growth, frames and locals | `echo.s`, `helloRet.s`, `routine.s`, `fun1.s`, `fun2.s`, `funStack.s` |
| Lecture 6 | separate source modules, ABI preservation, partial registers, integer parsing/printing, recursion, alignment, array traversal | `readInt.s`, `writeInt.s`, `fact.s`, `sumLoopArray.s` |
| Assignment 2 | combining input, output, arrays, loops, and procedures in one whole program | assignment `sum5.s` and student programs |

The design should follow this same order. Each implementation milestone should
finish a recognizable course lesson rather than adding disconnected pieces of
the ISA.

## Core source syntax

### Lexical and symbolic forms

- GNU assembler AT&T operand order: source before destination.
- `#` comments, blank lines, and labels on their own line or before data.
- Global names, dot-local names such as `.Lread_loop`, and numeric local labels
  such as `1:` with `1f`/`1b` references.
- Decimal and hexadecimal integers, negative integers, and character literals
  such as `$'0'` and `$'-'`.
- Immediate (`$value`), register (`%rax`), direct-symbol, and memory operands.
- Equates and current-location expressions, especially
  `hellolen = . - msg`.
- Multiple separately parsed source modules with global and external symbols.

### Sections and directives

Required spellings and aliases:

- `.text`, `.data`, `.rodata`, `.bss`, and `.section <name>`
- `.global`, `.globl`, and `.extern`
- `.byte`, `.word`, `.long`, `.quad`
- `.ascii`, `.asciz`
- `.skip`, `.zero`

`.space` is a low-cost compatibility alias to add with `.skip`. `.float` is
reserved for the later floating-point tier.

### Registers and widths

The machine has the 16 general-purpose 64-bit registers and their 32-, 16-,
and 8-bit views, plus `%rip` and the arithmetic flags. Correct alias behavior
is essential:

- A 32-bit destination write clears the upper half of the 64-bit register.
- An 8- or 16-bit write preserves the untouched upper bits.
- `%rsp` is initialized to a 16-byte-aligned stack address.
- `%rip` is observable but not directly assignable in the course subset.

All machine integers are fixed-width bit vectors. UI display may interpret the
same bits as hexadecimal, signed decimal, unsigned decimal, binary, or ASCII.
The Rust core stores architectural values as fixed-width integers, and the web
adapter must never round-trip them through floating-point JavaScript numbers.

### Addressing

Support the general course form:

```text
disp(base, index, scale)
address = base + index * scale + disp
```

`base` and `index` are optional, `scale` is 1, 2, 4, or 8, and `disp` may be a
constant or symbol. The common cases all need first-class explanations:

- `(%rax)` — pointer dereference
- `-8(%rbp)` — stack local
- `(%rbx,%rcx,8)` — quadword array indexing
- `num(%rip)` — global, RIP-relative
- `lea ...` — calculate the address without reading memory

### Instructions

The following families cover the actual Fall 2026 lecture sources and the
assignment skeleton. A suffix-less spelling should infer its width exactly
where GNU `as` can infer it from a register operand.

| Family | Required spellings/behavior |
|---|---|
| Data movement | `mov`, `movb`, `movl`, `movq`, `movabsq`, `movzbl`, `movzbq`, `lea`, `leaq` |
| Arithmetic | `add`, `addb`, `addl`, `addq`, `sub`, `subq`, `imul`, `divq`, `idivq`, `cqo`, `negq`, `inc`, `incq`, `dec`, `decq` |
| Logic/flags | `xor`, `xorq`, `cmp`, `cmpq`, `testq` |
| Control | `jmp`, `je`, `jne`, `jl`, `jle`, `jg`, `jge`, `jb`, `jbe`, `ja`, `jae`, `jno`, `call`, `ret` |
| Stack | `push`, `pushq`, `pop`, `popq`, `leave` |
| Kernel boundary | `syscall` |

`idivq`, `cqo`, `movzbq`, and the complete signed/unsigned branch pairs occur
in the guide rather than every live source file; including them keeps the
guide's explanations internally usable without materially broadening the
engine.

The observed source files use suffix variants rather freely. Instruction
handlers should therefore be defined once by operation and operand width, not
implemented as unrelated handlers for every spelling.

## Linux and ABI model

The initial environment is a small Linux/System V AMD64 process model:

- Entry is `_start`; there is no implicit `main` or return address.
- Function arguments use `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%r8`, `%r9`.
- Integer return values use `%rax`.
- `%rax`, `%rcx`, `%rdx`, `%rsi`, `%rdi`, `%r8`–`%r11` are caller-saved.
- `%rbx`, `%rbp`, `%r12`–`%r15` are callee-saved.
- `%rsp` must be 16-byte aligned immediately before `call`; `call` pushes an
  8-byte return address.
- The syscall convention uses `%rax` for the number and `%rdi`, `%rsi`,
  `%rdx`, `%r10`, `%r8`, `%r9` for arguments. A returning syscall updates
  `%rcx` and `%r11` as real x86-64 Linux does.

Release 1 implements only the syscalls used by the guide:

| `%rax` | Symbolic operation | Modeled result |
|---:|---|---|
| 0 | `read(fd, buffer, count)` | blocks for input if needed, writes bytes, returns byte count |
| 1 | `write(fd, buffer, count)` | appends exact bytes to stdout/stderr, returns byte count |
| 60 | `exit(status)` | halts and reports both the 64-bit argument and shell-visible low 8 bits |

Terminal input is submitted a line at a time by default, including the newline.
That matches how `readInt` is taught. A later raw/pipe mode can deliberately
reproduce the assignment warning that several lines may arrive in one `read`.

## Semantic details that cannot be approximated

- 64-bit arithmetic wraps modulo 2^64; `fact(21)` therefore overflows.
- The engine maintains at least `CF`, `PF`, `AF`, `ZF`, `SF`, and `OF`, even if
  the first UI only foregrounds the four flags used by course branches.
- `cmp S,D` sets flags from `D-S` and stores no result.
- Signed and unsigned branches use their real flag predicates.
- `inc`/`dec` do not change `CF`.
- `divq` consumes `%rdx:%rax`, produces quotient in `%rax` and remainder in
  `%rdx`, and faults on division by zero or quotient overflow.
- Memory is little-endian and reads/writes honor the instruction width.
- `.rodata` is not writable, `.bss` starts zeroed, and unmapped accesses fault.
- Falling off the last instruction is a fault; there is no implicit exit.
- `call`, `ret`, `push`, `pop`, and `leave` operate through memory and `%rsp`,
  rather than manipulating only a hidden UI call stack.

## Compatibility tiers and explicit non-goals

### Tier 1: course-writing mode

Everything above: handwritten whole programs from lectures 3–6 and assignment
2, including stepping into `readInt` and `writeInt`.

### Tier 2: compiler-reading bridge

Enough read-only/no-op metadata handling to make tiny `gcc -S` output less
hostile: `.file`, `.type`, `.size`, `.ident`, `.align`, `.cfi_*`, and `endbr64`.
Calls through the PLT or into libc require a clearly labeled host stub. This is
useful because lecture 3 briefly reads `test.s`, but it must not delay Tier 1.

### Tier 3: advanced reference examples

Scalar floating point and SIMD (`movss`, `addss`, `cvttss2si`, `vmovups`,
`vaddps`, XMM/YMM/ZMM state, `.float`). These live in the advanced example
collection and the guide's reference appendix, not in the current core arc.

Not planned: decoding ELF binaries, faithfully encoding variable-length x86
instructions, timing, caches, pipelines, privileged instructions, arbitrary
Linux syscalls, or pretending to be a secure sandbox for untrusted binaries.

## Representative acceptance corpus

The engine and UI should be considered course-compatible only when these
behaviors are covered by golden traces and end-to-end tests:

| Fixture | Expected observation |
|---|---|
| `first.s` | moves 60 to `%rax`, then reports that execution fell off `.text` |
| `firstfixed.s` | exits with argument `-1` / `0xffff...ffff`; shell status is 255 |
| `firstadd.s`, `firstsub.s` | make AT&T source/destination order visible; statuses 30 and 10 |
| `addArray3.s` | shows `&num + 1*8`, changes the second quad from 300 to 310 |
| `addArray4.s` | performs 32-bit access to the third `.long`, not an 8-byte access |
| `cmp1.s` | explains why `jge` is taken from the flags and yields shell status 255 |
| `sumLoop.s`, `sumLoopB.s` | terminate with 55 and permit reverse-stepping a loop |
| `hello.s` | writes the exact `.asciz` byte range, with the NUL visible in escaped-byte view |
| `echo.s` | blocks at `read`, writes only the returned number of bytes, then exits |
| `fun1.s`, `fun2.s` | makes the `%rdi` clobber difference explain the 80 versus 40 result |
| `funStack.s` | draws return address, saved `%rbp`, and two local quadwords |
| `readWriteTest.s` | links three modules and can step into or over helpers |
| `fact.s` with input 5 | shows one frame per recursive call and outputs `120` |
| `writeInt.s` | handles 0, a negative value, and `LONG_MIN` correctly |
| assignment `sum5.s` | accepts five line-oriented inputs, fills the array, and outputs 150 for 10–50 |

During the design review, the maintained sources were assembled and linked
with GNU `as`/`ld` without modifying the course repository. The observed shell
statuses included `first.s` = 139 (segmentation fault), `firstfixed.s` = 255,
`firstadd.s` = 30, `firstsub.s` = 10, `addArray3.s` = 54 (310 modulo 256),
`cmp1.s` = 255, `sumLoop*.s` = 55, `fun1.s` = 80, `fun2.s` = 40, and
`funStack.s` = 40. `hello.s` wrote 14 bytes, ending in `0a 00`; linked
`readWriteTest.s`, `fact.s`, and `sumLoopArray.s` produced `123`, the prompt
plus `120`, and `10` respectively. The assignment `sum5.s` was also exercised
through a pseudo-terminal with five separate lines and produced `150`.

## Material notes discovered during review

The fuller Spring 2026 lecture notes clarify two shorthand descriptions in the
Fall 2026 skeleton:

- `lecture3/first.s` is deliberately broken and demonstrates falling off the
  end; the Fall agenda currently calls it "the smallest program that exits."
- `lecture5/helloRet.s` demonstrates the return value of `write` in `%rax`; it
  contains no `call` or `ret`. The actual transition to `call`/`ret` is
  `routine.s` → `fun1.s`.

The visualizer's bundled lesson order and golden behavior should follow the
source plus the narrative notes. The course files themselves remain read-only
to this project.
