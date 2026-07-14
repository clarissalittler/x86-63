# Your first x86-63 traces

The register lab takes about 15 minutes, the memory lab another 15, the
input-and-stack lab about 20, and the linked-functions/recursion lab another
20. You will predict and run CS201 assembly, reverse register, memory, stack,
and input changes, investigate a broken program, and use an assembler
diagnostic to repair a mistake.

The current prototype teaches the maintained Lecture 3–6 subset: register and
memory forms of `mov`/`add`/`sub`, `lea`, `cmp`, `xor`, conditional jumps,
loops, `.data`/`.rodata`/`.bss`, `call`/`ret`/`push`/`pop`, and Linux
`read`/`write`/`exit` through `syscall`. It also links `.extern` references
across modules and runs the course's `readInt`, `writeInt`, and recursive
factorial using zero extension, multiply/divide, `test`, `neg`, and `leave`.

## Learning goals

By the end, you should be able to explain:

- why GNU/AT&T operands are read source first, destination second;
- which register changes in `add %rbx,%rcx`;
- why `_start` cannot simply fall off the end of its instructions;
- why an exit argument of `-1` appears to a shell as status `255`;
- how reverse stepping differs from merely moving a source highlight;
- how `base + index × scale` becomes a concrete memory address;
- how `.long` and `.quad` determine element width and stride; and
- how `cmp` supplies the flags consumed by a later conditional jump;
- why `read` can block and why its return value determines a later `write`;
- how `call` stores a return address and `ret` retrieves it;
- why the stack grows toward lower addresses and how `%rbp` names local slots;
  and
- when **Step** and **Next** behave differently at a function call;
- how `.extern` calls transfer control into another source module;
- how `movzbl`, repeated multiply-by-10, and a byte loop parse ASCII digits;
- how quotient and remainder from `divq` produce decimal output digits;
- why `%rsp mod 16` should be zero immediately before a course call; and
- how saved `%rbp` values form a reversible chain of recursive frames.

## Start in the browser

Open the URL your instructor gave you. If you are running the project locally,
follow the browser setup in the repository README and open the address printed
by `npm run web:dev`.

Choose **Guided tutorial**. It loads **Addition and AT&T operand order** and
starts a seven-checkpoint register walkthrough. **Memory tutorial** loads
**Base + index × scale** and starts a separate eight-checkpoint walkthrough.
**Stack tutorial** loads **Stack frame and local variables** and walks through
a call, its frame setup, two locals, teardown, and return. **Recursion
tutorial** fixes factorial's input at 5 and shows repeated `fact` frames growing
and unwinding. Keep the tutorial card open while you use the execution
controls.

The screen has two source views:

- The dark upper box is editable. Changes there do nothing until you choose
  **Assemble**.
- The numbered lower box is the assembled program. Its yellow row is the
  instruction that will execute next.

Linked Lecture 6 lessons add a **Source module** chooser. Step into a call and
the chooser follows execution into `readInt.s` or `writeInt.s`; choose another
module manually when you want to compare definitions before stepping.

The right side shows every general-purpose register in hexadecimal and signed
decimal, the six arithmetic flags, symbolic memory and exact bytes, the live
stack, process input/output, execution status, history depth, and an explanation
of the most recent transition.

## Trace addition

Do not choose **Run** yet. Assembly becomes much less mysterious when you slow
it down enough to make a prediction before each step.

### 1. Load `%rbx`

The initial yellow row is:

```asm
mov $10,%rbx
```

The `$` says that `10` is an immediate value. The `%` says that `rbx` is a
register. Predict the destination, then choose **Step**.

Check that:

- `%rbx` is highlighted and contains `0x000000000000000a`, signed `10`;
- the history depth is `1`;
- the yellow source row advances; and
- the explanation says that `mov` did not change the flags.

### 2. Load `%rcx`

Choose **Step** once more. `%rcx` should now contain signed `20`, while `%rbx`
still contains `10`.

### 3. Read arithmetic right-to-left

The next instruction is:

```asm
add %rbx,%rcx
```

AT&T syntax writes the source first and destination second. Before stepping,
complete this equation on paper:

```text
%rcx = old %rcx + %rbx = ___ + ___ = ___
```

Now choose **Step**. The result is `%rcx = 20 + 10 = 30`; `%rbx` remains `10`.
The parity flag (`PF`) is `1` because the low result byte has an even number of
set bits. The generated explanation should show the concrete calculation.

### 4. Reverse, then replay

Choose **Back** once. `%rcx` returns to `20`, the old flags return, the yellow
row returns to `add`, and history depth decreases to `2`. This is a real state
restoration.

Choose **Step** again. `%rcx` deterministically becomes `30` with the same
flags.

### 5. Finish the process

Choose **Run**. The remaining instructions copy `30` into `%rdi`, put syscall
number `60` in `%rax`, and invoke `syscall`. The status badge should say:

```text
exited · shell 30
```

Choose **Back** after exit. The machine becomes paused at `syscall`; choose
**Step** to perform the exit again. Reverse history includes terminal states,
not only arithmetic instructions.

## Compare subtraction

Select **Subtraction reads right-to-left**. Step through the two moves, then
pause at:

```asm
sub %rbx,%rcx
```

Predict whether `%rcx` becomes `10` or `-10`. The operation is:

```text
destination - source = old %rcx - %rbx = 20 - 10 = 10
```

Step once to test the prediction, then choose **Run**. The shell-visible exit
status is `10`.

## Investigate two exits that are not the same

Select **The deliberately incomplete program**. It contains only:

```asm
mov $60,%rax
```

`60` is the Linux syscall number for `exit`, but putting a number in a register
does not invoke the kernel. Choose **Step**. `%rax` changes, then the machine
faults because execution fell off the end of `.text`. `_start` has no implicit
return or exit.

Now select **Exit, fixed** and choose **Run**. `%rdi` contains the 64-bit bit
pattern for signed `-1`, while the status badge reports shell status `255`. A
shell observes only the low eight bits of an exit argument:

```text
-1 as 64 bits = 0xffffffffffffffff
low 8 bits     = 0xff = 255
```

## Trace an effective address and a memory write

Choose **Memory tutorial**. It loads **Base + index × scale**. Before stepping,
the Memory panel shows the four `.quad` values named by `num`:

```text
num[0] = 200   num[1] = 300   num[2] = 400   num[3] = 500
```

Each value occupies eight bytes and is displayed both as little-endian bytes
and as a signed decimal value.

### 1. Distinguish an address from its contents

The first instruction is:

```asm
lea num(%rip),%rbx
```

`lea` calculates an address; it does not read the bytes at that address. Step
once. `%rbx` becomes `0x0000000000400000`, the address printed next to `num`,
while `num[0]` remains `200`.

Step over `mov $1,%rcx`. The next memory operand is:

```asm
(%rbx,%rcx,8)
```

Evaluate it before stepping:

```text
base + index × scale = 0x400000 + 1 × 8 = 0x400008
```

That is the address of `num[1]`, because each `.quad` is eight bytes wide.

### 2. Change bytes, then reverse them

Step over:

```asm
addq $10,(%rbx,%rcx,8)
```

The second element is highlighted and changes from `300` to `310`. The event
receipt shows the effective-address calculation, memory read, and memory write
separately.

Choose **Back**. The eight original bytes return and `num[1]` is `300` again.
Choose **Step** to replay the write. This demonstrates that memory is part of
the reversible machine state.

Step once more over:

```asm
movq (%rbx,%rcx,8),%rdi
```

The parentheses dereference the calculated address, so `%rdi` receives the
stored value `310`. Choose **Run**. The process passes all 64 bits of `310` to
Linux, while the status badge shows `54` because a shell observes `310 mod
256`.

## Compare element widths

Select **.long changes width and stride**. Here `num` contains four `.long`
elements, so each element is four bytes rather than eight:

```text
num[0] = 50   num[1] = 100   num[2] = 150   num[3] = 200
```

After `lea`, the program adds eight to the pointer. Predict the selected index:

```text
8-byte displacement ÷ 4-byte elements = index 2
```

Continue through `addl $10,(%rbx)`. `num[2]` becomes `160`; `num[1]` remains
`100`. The suffix and declaration jointly make the width visible instead of
letting an accidental eight-byte interpretation pass unnoticed.

## Read a conditional branch receipt

Select **cmp sets flags; jge reads them**. Step through the three moves, then
step over:

```asm
cmp %rbx,%rcx
```

AT&T order means the comparison computes flags as if it had evaluated
`%rcx - %rbx`, or `20 - 10`. It does not store that result in either register.

The next instruction is `jge greater`. Predict whether it is taken, then Step.
The receipt names the signed predicate—`SF == OF`—and substitutes the concrete
flag values. Because the branch skips `mov $1,%rdi`, `%rdi` remains `-1` and
the eventual shell status is `255`. Choose **Back** to return to the branch and
see that control flow is reversible as well.

## Inspect exact process output

Select **Hello, bytes: the write syscall** and choose **Run**. The Process
I/O panel renders the text and also preserves its escaped byte form:

```text
Hello world!\n\0
```

The terminating NUL is visible because `.asciz` emitted it and the equate
`hellolen = . - msg` includes every byte through the current location. The
terminal would make that last byte easy to miss; the teaching machine does not.

Now select **write returns its byte count** and choose **Run**. This program
copies `%rax` after `write` into the exit argument. Its shell status is `14`,
the exact number of bytes written by `"Hello world!\n\0"`. A syscall number in
`%rax` is an input before `syscall`; the kernel's return value replaces it.

## Submit input and watch `read` block

Select **Echo one input line**. The `.bss` symbol `buff` reserves 128 zeroed
bytes without putting an initialized string in the executable.

Choose **Run** without submitting input. Execution pauses on the `syscall` at
line 14 and the status says **waiting for stdin**. The attempted `read` is not
added to history because no machine transition has completed; the yellow row
still names the same syscall.

Type `hello` in the **stdin line** field and choose **Submit line**. x86-63
queues `hello\n`, just as pressing Enter in a terminal supplies a line. Choose
**Step** once and check all three consequences:

- the first six bytes of `buff` are `68 65 6c 6c 6f 0a`;
- `%rax` is `6`, the number of bytes actually read; and
- Process I/O says that all 6 queued stdin bytes were consumed.

Choose **Back** immediately. The buffer returns to zeros, the input cursor
returns to the start of the queued line, and execution returns to the `read`
syscall. Step again, then choose **Run**. The program copies the return count
from `%rax` to `%rdx`, so `write` emits exactly `hello\n`, not all 128 bytes in
the buffer.

This is the key relationship:

```text
read return value in %rax → write count in %rdx
```

## Trace a call and a real stack frame

Choose **Stack tutorial**. It loads **Stack frame and local variables**. The
program computes `20 + 2 × 10`, but the point of this trace is where its
temporary values live.

Step over `mov $10,%rdi`, then step over `call fun`. `call` does two things as
one instruction: it subtracts eight from `%rsp` and stores the address of the
instruction after the call at that new stack address. The Stack panel labels
that value as the return address, and the yellow source row jumps to `fun`.

Continue one instruction at a time:

1. `push %rbp` moves `%rsp` down another eight bytes and saves the caller's
   frame pointer.
2. `mov %rsp,%rbp` makes that address the stable anchor for this frame.
3. `sub $16,%rsp` reserves two eight-byte local slots. The stack grows toward
   lower addresses, so subtraction allocates space.
4. `mov %rdi,-8(%rbp)` stores `10` in the first local.
5. `add %rdi,-8(%rbp)` changes that local to `20`.
6. `movq $20,-16(%rbp)` stores `20` in the second local.

The address formulas are literal:

```text
first local  = %rbp - 8
second local = %rbp - 16
saved %rbp   = %rbp
return value = %rbp + 8
```

Choose **Back** after writing either local. Its previous eight bytes return,
which demonstrates that stack memory uses the same reversible state model as
`.data` and `.bss`.

Replay the write and continue. `mov %rbp,%rsp` discards both locals at once,
`pop %rbp` restores the saved frame pointer, and `ret` pops the stored return
address into control flow. The caller then sees `%rax = 40` and exits with
status `40`.

### Compare Step with Next

Select **A routine that changes its argument**. Step once so the yellow row is
on `call rdadder`, then choose **Next**. Next runs the call, its body, and its
`ret`, stopping at the instruction after the call. `%rax` and `%rdi` are both
`40`, and the stack is empty again. Reset and use **Step** at the same call to
enter the routine and inspect its return address instead.

Run this example to completion; it exits `80` because the routine doubles
`%rdi` in place, so its second call receives `40`. Then run **A routine that
preserves its argument**. It exits `40` because the routine uses `%r9` as its
working register and leaves `%rdi = 20` for the second call. The instruction
set does not decide which registers a routine may destroy—programmers need an
agreed calling convention.

## Follow a call into another source module

Select **Link readInt and writeInt**. The main module declares two names without
defining them:

```asm
.extern readInt
.extern writeInt
```

Use the **Source module** chooser to confirm that the definitions live in
`readInt.s` and `writeInt.s`. Assemble all three modules together; `.extern`
does not create a function, but it tells the source reader that another module
supplies the label.

Return to `readWriteTest.s` and Step on `call readInt`. The yellow row moves to
`readInt.s`, and the return-address slot says it will return to
`readWriteTest.s:8`. Choose **Run** without queued input. Execution stops at the
`read` syscall in `readInt.s`, not in the caller.

Submit `123` and Step the syscall. Check that:

- the shared `.bss` buffer begins with `31 32 33 0a`;
- `%rax = 4`, because the newline is part of the terminal line; and
- `readInt` saves that count, subtracts one, and asks `parseInt` to process
  exactly three digits.

At `call parseInt`, Step to enter the helper. Each loop iteration zero-extends
one byte with `movzbl`, subtracts ASCII `'0'`, multiplies the accumulator by
10, and adds the new digit:

| Input byte | Digit | Old accumulator | `old × 10 + digit` |
|---|---:|---:|---:|
| `'1'` (`0x31`) | 1 | 0 | 1 |
| `'2'` (`0x32`) | 2 | 1 | 12 |
| `'3'` (`0x33`) | 3 | 12 | 123 |

When `readInt` returns, `%rax` carries `123` across the module boundary. Reset,
submit `123` before execution, Step to the first call, and choose **Next**.
Next runs both `readInt` and its nested `parseInt` call, then stops back in
`readWriteTest.s` with `%rax = 123` and a balanced stack.

Step the move into `%rdi`, then Step into `writeInt`. Its 32-byte stack buffer
is filled backward. Each `divq` divides the unsigned value in `%rdx:%rax` by
10 and returns both pieces:

| Dividend | Quotient in `%rax` | Remainder in `%rdx` | Stored byte |
|---:|---:|---:|---|
| 123 | 12 | 3 | `'3'` |
| 12 | 1 | 2 | `'2'` |
| 1 | 0 | 1 | `'1'` |

Moving `%rsi` toward lower addresses makes those reverse-order remainders form
the forward string `123`. `write` emits it, `leave` performs
`mov %rbp,%rsp` plus `pop %rbp`, and `ret` returns to the main module.

## Watch recursive frames grow and unwind

Choose **Recursion tutorial**. It loads the compact **Factorial frame tracing
lab**, which uses the same recursive function as the maintained course program
but fixes `%rdi = 5` so input parsing does not obscure the stack trace.

After the first `call fact`, the Stack panel reports `%rsp mod 16 = 8`: `call`
has just pushed one eight-byte return address. Step through `push %rbp`, the
move into `%rbp`, and `sub $16,%rsp`. The first frame appears and the stack is
call-ready again:

```text
before call:       %rsp mod 16 = 0
after call:        %rsp mod 16 = 8
after push %rbp:   %rsp mod 16 = 0
after 16B locals:  %rsp mod 16 = 0
```

The function saves its current argument at `-8(%rbp)`, decrements `%rdi`, and
calls itself. Build the second frame as directed by the tutorial. The frame
chain is newest first; every card names `fact`, its `%rbp`, and the source line
to which it will return.

Repeat the eight-instruction descent if you want to expose all five frames.
Their preserved locals explain the later multiplications:

| Frame argument | Saved local | Value returned upward |
|---:|---:|---:|
| 1 | base case | 1 |
| 2 | 2 | 2 |
| 3 | 3 | 6 |
| 4 | 4 | 24 |
| 5 | 5 | 120 |

Choose **Back** after a frame's `mov %rsp,%rbp`. The newest frame disappears
because reverse execution restores the previous `%rbp`; replaying the move
reconstructs the same chain. Choose **Run** from any paused point. Each `leave`
removes one frame, each `ret` follows its stored address, and the harness exits
with status `120` and an empty stack.

Finally select **Recursive factorial across three modules**, submit `5`, and
Run. This is the maintained program rather than the compact harness. It prints
exactly:

```text
Enter a number: 120
```

When you Step rather than Run, the Source module chooser follows the whole path:
`fact.s → readInt.s → fact.s → writeInt.s → fact.s`.

## Make and repair a source mistake

Return to **Addition and AT&T operand order**. In the dark editor, change:

```asm
mov $10,%rbx
```

to:

```asm
mov 10,%rbx
```

Choose **Assemble**. Execution controls become unavailable and diagnostic
`E212` points to line 5: the immediate value is missing `$`. The diagnostic
suggests `$10`; it does not silently fix your program.

Restore the `$`, choose **Assemble**, and confirm that the machine is paused at
the first instruction with history `0`.

## Write one tiny program

Replace the editor contents with this program and choose **Assemble**:

```asm
.section .text
.global _start

_start:
    mov $7,%rax
    add $5,%rax
    mov %rax,%rdi
    mov $60,%rax
    syscall
```

Before running, predict the final value copied to `%rdi` and the shell status.
Then step through it. Both should be `12`.

Try one variation of your own: change one immediate, replace `add` with `sub`,
or use a different destination register. If the source is outside this
prototype's subset, x86-63 should produce an explicit diagnostic.

## Native terminal version

From the repository root, open the full-screen visualizer with:

```sh
cargo run -p x86-63-cli -- tui --example firstadd
```

The keys are shown at the bottom of the screen:

| Key | Action |
|---|---|
| `s`, `Enter`, or right arrow | Step one instruction |
| `n` | Next; step over a call, otherwise step one instruction |
| `b` or left arrow | Reverse one instruction |
| `c` | Continue to exit or fault |
| `i` | Enter one stdin line; Enter submits it and Esc cancels |
| `r` | Reset to `_start` |
| `m` | Cycle the right pane through registers, memory/I/O, and stack |
| `q` or `Esc` | Leave the TUI |

The TUI automatically opens its input editor when a `read` blocks. To start
the echo example with a line already queued, use the scriptable runner:

```sh
cargo run -p x86-63-cli -- run --example echo --stdin hello
cargo run -p x86-63-cli -- run --example fact --stdin 5
cargo run -p x86-63-cli -- tui --example facttrace
```

Bundled Lecture 6 examples automatically load their helper modules. For your
own files, pass every module as a positional argument:

```sh
cargo run -p x86-63-cli -- run fact.s readInt.s writeInt.s --stdin 5
```

The TUI Stack pane prints `%rsp mod 16` and the active frame chain above the
raw slots. Use `m` to return to it whenever a recursive `call` or `leave`
switches the detail pane.

For a line-oriented interface, use:

```sh
cargo run -p x86-63-cli -- repl --example firstadd
```

Enter `step` or the GDB-style alias `si`, then try `next`, `regs`, `memory`,
`stack`, `output`, `input hello`, `why`, `back`, `run`, `reset`, `help`, and
`quit`. The browser buttons, TUI keys, and REPL commands all call the same
execution engine.

## Feedback for this prototype

When reporting your test, include:

1. Which interface you used: browser, TUI, or REPL.
2. The example and action immediately before the problem.
3. What you expected and what actually happened.
4. Whether **This step** and the Memory or Stack panel helped you explain the
   state change.
5. One place where you hesitated, even if the tool behaved correctly.

A useful one-line report looks like:

> Browser, firstsub, third Step: I expected `%rbx` to change because I read the
> operands left-to-right; the explanation corrected me, but I did not notice
> the changed-register highlight at first.
