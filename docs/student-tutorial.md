# Your first x86-63 trace

This tutorial takes about 15 minutes. You will predict and run the first
assembly sequence from CS201, reverse an instruction, investigate a broken
program, and use an assembler diagnostic to repair a mistake.

The current prototype teaches the Lecture 3 subset: `mov`, `add`, `sub`, and
Linux `exit` through `syscall`. Later course features such as memory, jumps,
loops, calls, and terminal I/O are not in this build yet.

## Learning goals

By the end, you should be able to explain:

- why GNU/AT&T operands are read source first, destination second;
- which register changes in `add %rbx,%rcx`;
- why `_start` cannot simply fall off the end of its instructions;
- why an exit argument of `-1` appears to a shell as status `255`; and
- how reverse stepping differs from merely moving a source highlight.

## Start in the browser

Open the URL your instructor gave you. If you are running the project locally,
follow the browser setup in the repository README and open the address printed
by `npm run web:dev`.

Choose **Guided tutorial**. It loads **Addition and AT&T operand order** and
starts a seven-checkpoint walkthrough. Keep the tutorial card open while you
use the execution controls.

The screen has two source views:

- The dark upper box is editable. Changes there do nothing until you choose
  **Assemble**.
- The numbered lower box is the assembled program. Its yellow row is the
  instruction that will execute next.

The right side shows every general-purpose register in hexadecimal and signed
decimal, the six arithmetic flags, execution status, history depth, and an
explanation of the most recent transition.

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
| `q` or `Esc` | Leave the TUI |

For a line-oriented interface, use:

```sh
cargo run -p x86-63-cli -- repl --example firstadd
```

Enter `step` or the GDB-style alias `si`, then try `regs`, `why`, `back`,
`run`, `reset`, `help`, and `quit`. The browser buttons, TUI keys, and REPL
commands all call the same execution engine.

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
