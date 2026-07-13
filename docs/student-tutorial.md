# Your first x86-63 traces

The register lab takes about 15 minutes; the memory lab takes another 15. You
will predict and run CS201 assembly, reverse both register and memory changes,
investigate a broken program, and use an assembler diagnostic to repair a
mistake.

The current prototype teaches the maintained Lecture 3–4 subset: register and
memory forms of `mov`/`add`/`sub`, `lea`, `cmp`, `xor`, conditional jumps,
loops, `.data` directives, and Linux `write`/`exit` through `syscall`. Calls,
the stack, and the course's integer I/O helpers are not in this build yet.

## Learning goals

By the end, you should be able to explain:

- why GNU/AT&T operands are read source first, destination second;
- which register changes in `add %rbx,%rcx`;
- why `_start` cannot simply fall off the end of its instructions;
- why an exit argument of `-1` appears to a shell as status `255`;
- how reverse stepping differs from merely moving a source highlight;
- how `base + index × scale` becomes a concrete memory address;
- how `.long` and `.quad` determine element width and stride; and
- how `cmp` supplies the flags consumed by a later conditional jump.

## Start in the browser

Open the URL your instructor gave you. If you are running the project locally,
follow the browser setup in the repository README and open the address printed
by `npm run web:dev`.

Choose **Guided tutorial**. It loads **Addition and AT&T operand order** and
starts a seven-checkpoint register walkthrough. **Memory tutorial** loads
**Base + index × scale** and starts a separate eight-checkpoint walkthrough.
Keep the tutorial card open while you use the execution controls.

The screen has two source views:

- The dark upper box is editable. Changes there do nothing until you choose
  **Assemble**.
- The numbered lower box is the assembled program. Its yellow row is the
  instruction that will execute next.

The right side shows every general-purpose register in hexadecimal and signed
decimal, the six arithmetic flags, symbolic memory and exact bytes, process
output, execution status, history depth, and an explanation of the most recent
transition.

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
output panel renders the text and also preserves its escaped byte form:

```text
Hello world!\n\0
```

The terminating NUL is visible because `.asciz` emitted it and the equate
`hellolen = . - msg` includes every byte through the current location. The
terminal would make that last byte easy to miss; the teaching machine does not.

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
| `n` | Next; currently identical to Step because this slice has no calls |
| `b` or left arrow | Reverse one instruction |
| `c` | Continue to exit or fault |
| `r` | Reset to `_start` |
| `m` | Toggle the right pane between registers and memory/output |
| `q` or `Esc` | Leave the TUI |

For a line-oriented interface, use:

```sh
cargo run -p x86-63-cli -- repl --example firstadd
```

Enter `step` or the GDB-style alias `si`, then try `regs`, `memory`, `output`,
`why`, `back`, `run`, `reset`, `help`, and `quit`. The browser buttons, TUI
keys, and REPL commands all call the same execution engine.

## Feedback for this prototype

When reporting your test, include:

1. Which interface you used: browser, TUI, or REPL.
2. The example and action immediately before the problem.
3. What you expected and what actually happened.
4. Whether **This step** helped you explain the state change.
5. One place where you hesitated, even if the tool behaved correctly.

A useful one-line report looks like:

> Browser, firstsub, third Step: I expected `%rbx` to change because I read the
> operands left-to-right; the explanation corrected me, but I did not notice
> the changed-register highlight at first.
