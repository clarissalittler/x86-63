# Product and technical design

## Product thesis

`x86-63` should occupy the space between tracing a program on paper and using
GDB. Paper is excellent for slowing down but poor at checking a whole program;
GDB is accurate but exposes raw state without explaining it. This tool should
execute strict, deterministic semantics while narrating the state transition
in the language already used in CS201.

The primary audience is a student encountering assembly for the first time.
The secondary audience is the instructor projecting an example live. A third
use is assignment debugging, but the tool should teach rather than merely tell
students what line to change.

The central interaction is:

1. Load or paste one or more `.s` modules.
2. Parse/link them and show precise, source-local diagnostics.
3. Start at `_start` with a visible initial machine state.
4. Predict what the highlighted instruction will do.
5. Step, observe a small diff, and read a generated explanation.
6. Step backward, set a breakpoint, or inspect state with familiar GDB-like
   commands.

## Two first-class surfaces, one machine

Release 1 should ship both of these interfaces:

1. **Browser visualizer.** A static site gives students a URL with no toolchain
   setup, works well on a classroom projector, and can keep the source,
   registers, memory, stack, explanation, and I/O visible together.
2. **Native terminal REPL/TUI.** A local binary works over SSH on the course
   server, feels continuous with GDB, supports keyboard-driven instruction,
   and remains useful without a browser. The same binary should also have a
   plain line-oriented REPL and headless commands for screen readers, scripts,
   and CI.

Neither is a companion or reduced version of the other. They share one Rust
core containing the lexer, parser, linker, machine, command interpreter,
history, diagnostics, and structured explanation events. The browser consumes
that core through a thin WebAssembly adapter; the terminal binary links it
natively. UI code may choose a layout, but it may not calculate flags, resolve
addresses, emulate an instruction, or invent a different command behavior.

This changes the language recommendation from TypeScript-only to:

- **Rust core:** exact `u64`/`i64` bit behavior, a strong typed IR, one native
  artifact for the terminal, and the same code compiled to WebAssembly.
- **Browser:** React, TypeScript, Vite, and CodeMirror 6. TypeScript renders
  core-owned state projections and events; it does not implement semantics.
- **Terminal:** Ratatui and Crossterm for the full-screen interface, with the
  same binary exposing plain `repl`, `run`, `check`, and `trace` modes.

The x86 subset is small enough that raw performance is not the reason to use
Rust/WASM. The reason is semantic identity across a static web app and a native
binary. The cost is a WebAssembly boundary, so that boundary must be coarse:
keep a session inside WASM and return a compact state projection plus
`StepDelta` events per command rather than serializing the entire machine and
memory after every step.

Proposed repository shape:

```text
Cargo.toml                     Rust workspace
crates/
  x86-63-core/                parser, linker, machine, commands, trace
  x86-63-wasm/                wasm-bindgen session adapter
  x86-63-cli/                 TUI, plain REPL, run/check/trace commands
  x86-63-course/              lesson manifests and shared example loading
apps/
  web/                        React/TypeScript browser visualizer
course-content/               versioned `.s` modules and lesson metadata
tests/
  fixtures/                   small compatibility programs
  differential/               GNU as/ld comparisons on x86-64 Linux
docs/
```

The core crate should not depend on Ratatui, React concepts, browser APIs, or a
host terminal. The WASM and terminal crates are adapters around its public
command/event API.

## Main visual layout

Desktop/projector layout:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Example ▾  Modules  Assemble   Reset    ◀ Back  Step  Next  Run   speed ▾ │
├──────────────────────────────────────────┬─────────────────────────────────┤
│ SOURCE                                   │ REGISTERS             FLAGS     │
│  12  mov $10,%rcx                       │ rax  0x...003c         ZF 0      │
│  13  cmp $10,%rcx                       │ rbx  0x...0037         SF 0      │
│▶ 14  jle loopStart                      │ rcx  0x...000b  +1     OF 0      │
│  15                                   │ rdi  0x...0000         CF 0      │
│                                          ├─────────────────────────────────┤
│                                          │ STACK / MEMORY / SYMBOLS        │
│                                          │ ...                             │
├──────────────────────────────────────────┼─────────────────────────────────┤
│ REPL                                     │ THIS STEP                       │
│ (x86-63) p/d $rcx                        │ `jle` is not taken.              │
│ $1 = 11                                  │ Signed ≤ requires ZF=1 or       │
│                                          │ SF≠OF; both are false.           │
├──────────────────────────────────────────┴─────────────────────────────────┤
│ STDIN / STDOUT / DIAGNOSTICS                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

On a narrow screen, Source, Machine, Memory/Stack, and Console become tabs.
The first release may optimize for laptop/projector use, but color, focus
order, and every change indicator must remain accessible without relying only
on red/green.

The terminal TUI mirrors the same information architecture rather than trying
to imitate the browser pixel-for-pixel:

```text
┌ Source: sumLoop.s ─────────────────┬ Registers / Flags ───────────────────┐
│  12  cmp $10,%rcx                 │ rax  0x000000000000003c   ZF 0       │
│▶ 13  jle loopStart                │ rcx  0x000000000000000b   SF 0       │
│  14  mov %rbx,%rdi                │ rsp  0x00007fffffffe000   OF 0       │
├ Stack / Memory ────────────────────┼ This step / I/O ─────────────────────┤
│ rsp → ...                          │ jle not taken: ZF=0 and SF=OF        │
│                                    │ stdout:                              │
├────────────────────────────────────┴──────────────────────────────────────┤
│ (x86-63) p/d $rcx                                                      │
└──────────────────────────────────────────────────────────────────────────┘
```

The TUI needs a single-pane fallback for small terminals and a `--no-tui`
line-oriented mode that emits the same explanations without cursor control or
color. The native command should behave sensibly over SSH and respect
`NO_COLOR`.

## What each pane teaches

### Source

- Highlight the next instruction, not merely the last executed line.
- Highlight source and destination operands separately on hover/focus.
- Preserve comments because the course examples use them as lesson text.
- Multiple module tabs are essential for `readInt`/`writeInt`.
- A gutter holds current location, breakpoints, and parse/runtime diagnostics.
- "Step into source module" must switch tabs without disorienting the learner.
- The browser edits source inline. The TUI keeps its source pane read-only,
  offers `reload` after changes in the student's normal editor, and may launch
  `$EDITOR` as a convenience. Scratch `asm` remains available in either
  interface; release 1 does not need to grow a second terminal text editor.

### Registers

- Default to registers used by the current program plus `%rsp`, `%rbp`, and
  `%rip`; offer an all-register view.
- Flash changed bits/bytes, not just the whole row. A write to `%edi` should
  visibly clear the upper half of `%rdi`; a write to `%dil` should not.
- Show hex and signed decimal together by default. Unsigned, binary, and ASCII
  are toggles, not separate state.
- Label contextual roles such as "1st argument," "return value," "stack
  pointer," and "callee-saved."

### Flags and branches

- Show `ZF`, `SF`, `OF`, and `CF` prominently; expose `PF` and `AF` in the full
  view.
- After `cmp`, render the conceptual subtraction with concrete operands.
- Before/after a conditional jump, render its Boolean predicate and which
  terms are true. This is more useful than a generic "branch taken" badge.
- Explicitly label a branch as signed or unsigned.

### Memory

- Organize memory by `.rodata`, `.data`, `.bss`, and stack, with symbolic labels
  before raw addresses.
- Provide bytes, words, longs, quads, signed/unsigned integers, and escaped
  string views over the same data.
- On a memory operand, animate the effective-address equation with live values:
  `&nums + %rbx * 8 = 0x...`.
- Highlight the precise bytes read or written and their little-endian order.
- Use directive metadata to offer helpful `nums[3]` labels without treating
  those inferred types as architectural facts.

### Stack

- Draw the stack downward, matching the course language.
- Keep `%rsp` and `%rbp` arrows visible.
- Annotate architectural entries (return addresses, saved registers, bytes at
  local offsets) and separately show a derived/shadow call-frame outline.
- Show `%rsp mod 16` continuously and explain alignment before each `call`.
- Recursion should display repeated frames with the same source function but
  different saved `n` values.

### I/O

- A `read` with no pending input enters an explicit `waiting for stdin` state;
  it does not fake an empty read or freeze the UI.
- Submitted terminal input shows its bytes, including newline.
- `write` shows symbolic arguments and previews the exact byte range first.
- The stdout view has rendered and escaped-byte modes so the NUL written by the
  course's `.asciz` hello example is visible.
- On exit, show both `%rdi` as a 64-bit value and what `echo $?` would show.

### "This step" explanation

Explanations are generated from execution events, never reimplemented in UI
components. A useful explanation has four compact parts:

1. **Decode:** `add (%r15,%rcx,8),%rax` is a 64-bit addition.
2. **Read:** address = `arr + 2*8`; memory there is `3`; `%rax` is `3`.
3. **Write:** `%rax = 3 + 3 = 6` (hex and signed views available).
4. **Flags/notes:** list changed flags and any relevant width/ABI observation.

This event stream is also the basis of reverse stepping, tests, and a future
"predict the result" exercise mode.

## REPL design

Use the GDB vocabulary already introduced in `assemblyGuide.org` wherever it
fits, while accepting friendlier long aliases. This makes the visualizer a
bridge into the real debugger rather than a parallel command language.

| Commands | Meaning |
|---|---|
| `start`, `run`, `reset` | initialize/reinitialize execution |
| `modules`, `reload` | list modules or re-read native file-backed modules |
| `si`, `step` | execute one instruction, entering calls |
| `ni`, `next` | execute through a call as one source-level action |
| `back`, `reverse-stepi` | undo one deterministic transition |
| `c`, `continue` | run to breakpoint, input wait, exit, fault, or step limit |
| `break <label|line>` | set breakpoint by symbol or source line |
| `info registers` | show machine registers |
| `p/x`, `p/d`, `p/t <expr>` | inspect expression in hex/decimal/binary |
| `x/8xb &buff`, `x/4gx $rsp`, `x/s &msg` | GDB-like memory examination |
| `stdin <escaped text>` | enqueue bytes for a blocked/future read |
| `set $rax = 10` | explicitly mutate state, recorded in history |
| `why` | focus the explanation for the current/previous transition |
| `asm <instruction>` | execute one instruction in scratch mode |
| `help [topic]` | command and course-subset help |

Browser controls, TUI keys, and textual REPL commands all lower to the same
core `Command` enum. A click on **Step**, the TUI step key, and entering `si`
must produce the same transition and event stream. Command parsing belongs in
shared Rust code so aliases and errors cannot drift across surfaces.

Scratch mode is a valuable lecture tool: start with a blank initialized
machine and enter `asm mov $10,%rbx`, then `asm add %rbx,%rbx`. It should use
the same parser and semantics as program mode, not a second evaluator.

The native binary should expose these outer modes in addition to the in-session
REPL commands:

```text
x86-63 tui   program.s [helper.s ...]     # default interactive full screen
x86-63 repl  program.s [helper.s ...]     # line-oriented interactive mode
x86-63 run   program.s [helper.s ...]     # run to exit/input/fault
x86-63 check program.s [helper.s ...]     # parse/link/lint only
x86-63 trace --format jsonl program.s     # stable machine-readable events
```

`run`, `check`, and `trace` make the native surface useful in assignment
workflows and give the browser adapter an independent integration oracle.

## Strict execution plus teaching diagnostics

The engine should execute valid source strictly. Teaching assistance is a
separate diagnostics layer; it must never silently repair a program.

High-value parse/link diagnostics:

- immediate missing `$`, register missing `%`, or likely reversed operands;
- impossible memory-to-memory move;
- operand widths that cannot be reconciled;
- undefined or multiply defined symbols;
- unsupported instruction with the supported course alternative/tier;
- unresolved `.extern` with an action to add the course helper module.

High-value runtime explanations/warnings:

- `.long` data accessed as a quad, with the exact extra bytes highlighted;
- byte/word write leaving stale high bits, with `movzbl` suggested when apt;
- access outside mapped memory or write to `.rodata`;
- wrong conventional fd for `read`/`write`;
- stack misalignment at `call`;
- a function returning with a changed callee-saved register;
- `ret` to an invalid address or an unbalanced stack;
- signed branch used where values look like a length/index (worded as a prompt,
  not an assertion of intent);
- integer overflow, including the first overflowing factorial step;
- falling off `.text` without exit.

Warnings need provenance and confidence. "`%rbx` changed across this call" is
a fact; "you probably intended an unsigned branch" is an inference and should
say so.

Two modes can tune density without changing semantics:

- **Learn:** explanations open, contextual ABI labels, high-confidence hints.
- **Inspect:** compact diffs, all registers/flags, hints collapsed.

A later **Predict** mode can ask the student to enter the destination value or
whether a branch is taken before revealing the transition.

## Core architecture

```text
source modules
     │
     ▼
lexer → parser → source AST → validator/linker → Program IR + source map
                                                     │
                                                     ▼
Command ───────────────────────────────────────→ transition(machine, command)
                                                     │
                                  ┌──────────────────┴──────────────────┐
                                  ▼                                     ▼
                            next MachineState                    StepDelta/events
                                                                         │
                              ┌──────────────────────────────────────────┴─────────┐
                              ▼                                                    ▼
                    native session adapter                              WASM session adapter
                    ┌──────────┼─────────┐                                      │
                    ▼          ▼         ▼                                      ▼
                   TUI       REPL   run/check/trace                         browser UI
```

The architectural seam is `Session::execute(Command) -> CommandResult`, where
the result contains status, a reversible delta/event list, diagnostics, and the
small state projection needed for rendering. Both adapters also expose
read-only queries for paged memory, symbols, source spans, and disassembly-like
instruction listings. A renderer must not need private machine internals.

Structured protocol values crossing WASM use an explicitly versioned schema.
Architectural integers are encoded as fixed-width hexadecimal strings or
byte arrays at the JavaScript boundary, never as JSON numbers. A JSONL trace
from the native binary uses the same schema, which makes cross-surface parity
testable and gives future tools a stable integration format.

### Program representation

Use a typed IR; do not execute strings. Each instruction retains module,
line/column spans, original spelling, resolved operand widths, and resolved
symbol references. Data declarations retain byte ranges and optional display
metadata such as element width and string provenance.

Assign deterministic virtual regions for text, read-only data, writable data,
BSS, and stack. These are teaching-machine addresses, not actual ELF layout.
RIP-relative symbolic operands resolve through the virtual linker and behave
consistently. The UI must label addresses as virtual and should foreground
symbols/source lines rather than imply that pseudo instruction addresses are
real machine encodings.

The linker must support multiple modules, local versus global symbols,
externals, equates, and a chosen entry point. `readInt.s` and `writeInt.s`
should be ordinary bundled source modules so students can step into their real
implementation; opaque host intrinsics would undermine lecture 6.

### Machine state

Conceptually:

```rust
struct MachineState {
    registers: RegisterFile, // canonical u64 storage + alias accessors
    rip: InstructionRef,
    flags: Flags,
    memory: PagedMemory,
    io: IoState,
    status: MachineStatus,
    // Derived UI/ABI metadata; never used to implement ret semantics.
    shadow_frames: Vec<ShadowFrame>,
}
```

Memory can begin as sparse pages or immutable initial image plus an overlay.
Programs are tiny, so clarity matters more than micro-optimization.

### Transitions and history

Every instruction produces a reversible `StepDelta`:

- instruction before/after;
- register writes with old/new canonical bits and alias used;
- flag writes including undefined flags;
- ordered memory reads and writes with effective-address derivations;
- control-flow decision;
- stack/call-frame events;
- input consumed and output appended;
- diagnostics, exit, wait, or fault event.

Undo applies old values from the delta in reverse order. This is less memory
than cloning every state and provides exactly the facts needed by the UI. User
mutations (`set`, adding stdin) are transitions too, so a history remains
deterministic.

`continue` must have a configurable instruction limit and stop on repeated
state/PC heuristics with a friendly "possible infinite loop" prompt. The WASM
adapter runs bounded chunks in a Web Worker so it cannot lock the browser main
thread. The native adapter uses the same chunk/stop contract and permits a
prompt Ctrl-C cancellation without leaving the session inconsistent.

### Instruction implementation

Centralize these primitives:

- read/write a register alias at width;
- read/write little-endian memory at width;
- resolve an effective address while emitting derivation events;
- wrapping add/subtract and flag calculation;
- condition-code predicates;
- push/pop of 64-bit values;
- fault creation.

Instruction definitions should compose those primitives. This avoids subtle
drift between, for example, `subq`, `cmpq`, and `jge`, and it makes unit tests
small enough to audit against Intel/AMD semantics.

## Verification strategy

Use five layers:

1. **Unit semantics:** edge cases for aliases, overflow/carry flags, branch
   predicates, endian memory, division, and stack primitives.
2. **Parser/linker fixtures:** every syntax form in the course corpus, with
   golden diagnostics for malformed forms.
3. **Golden traces:** the representative programs in `course-subset.md`,
   asserting meaningful intermediate diffs as well as final output.
4. **Differential Linux tests:** assemble/link deterministic supported programs
   with GNU `as`/`ld`, run them, and compare stdout and shell status. Small
   generated arithmetic cases can compare final values via an exit/write
   harness. Differential tests run only where x86-64 Linux and binutils exist.
5. **Adapter parity:** replay the same versioned command script through the
   native session and the WASM session, then assert identical diagnostics,
   status projections, and trace events. Browser and TUI smoke tests assert
   that their controls dispatch the expected shared commands.

The source interpreter will not share real instruction addresses with an ELF,
so differential tests compare architectural observations in scope, not byte
encoding or absolute addresses.

## Delivery as course-shaped vertical slices

### Slice 0: executable semantics charter

- Set up the Rust workspace, core types, versioned command/event schema, and
  compiling native/WASM adapter shells.
- Implement exact register aliases, widths, flags, and sparse memory.
- Encode the acceptance fixtures and expected outcomes before the UI grows.
- Deliverable: headless tests for machine primitives plus a native/WASM parity
  smoke test for a no-op session command.

### Slice 1: Lecture 3 — first arithmetic

- Parse `.text`, `.global`, labels, immediates, registers, `mov`, `add`, `sub`,
  `syscall 60`.
- Minimal browser and terminal shells with source, register diff, explanation,
  reset, step, and back. The native binary also exposes the plain REPL.
- Run `first.s`, `firstfixed.s`, `firstadd.s`, `firstsub.s`.
- This is the smallest useful prototype and the first cross-surface parity
  gate: the same command script must yield the same trace in native and WASM.

### Slice 2: Lecture 4 — memory and control flow

- Data/rodata/BSS directives, equates, full addressing, widths, `lea`.
- Arithmetic flags, comparisons, conditional/unconditional branches.
- Memory, flags, and effective-address views in both browser and TUI.
- `write` syscall and output byte view.
- Run every hand-written lecture 4 file.

### Slice 3: Lecture 5 — I/O and the stack

- Blocking `read`, terminal input, exact byte counts.
- `call`, `ret`, push/pop/leave, breakpoints, step-over.
- Stack-frame visualization and alignment checks in both interfaces, with
  equivalent text in plain REPL/trace output.
- Run every hand-written lecture 5 file.

### Slice 4: Lecture 6 and the assignment

- Multiple modules, global/extern resolution, helper module chooser.
- Remaining helper instructions: zero extension, multiply/divide, negate,
  signed/unsigned branches.
- Recursive frame UX and callee-saved checks across browser, TUI, and plain
  REPL.
- Run `readInt`, `writeInt`, `fact`, `sumLoopArray`, and assignment `sum5`.

### Slice 5: cross-surface classroom polish

- Complete the shared GDB-compatible command set and examples/lesson picker.
- Browser: URL-shareable source, Learn/Inspect density, keyboard and screen
  reader pass, stable static deployment, instructor projector mode.
- Native: responsive SSH behavior, single-pane/small-terminal mode,
  `NO_COLOR`, shell completion, and distributable binaries.
- Both: import/export the same module bundle, replay the same command scripts,
  and render every structured explanation event.
- Differential and adapter-parity gates across the complete Tier 1 corpus.

## Release 1 acceptance criteria

Release 1 is done when:

- Every handwritten Fall 2026 lecture 3–6 assembly file except compiler output
  either executes correctly or produces an intentional, precise fault.
- The assignment skeleton links with the real helper source and accepts five
  separate terminal inputs in both browser and native sessions.
- Step, step-over, continue, breakpoint, reset, and reverse-step agree across
  browser controls, TUI keys, and REPL commands.
- Every transition can identify exactly which registers, flags, memory bytes,
  stack entries, and I/O bytes changed.
- Invalid/unsupported syntax never silently changes meaning.
- Continue cannot freeze the browser or make the TUI unresponsive to cancel.
- `--no-tui` exposes every execution state and diagnostic without requiring
  cursor control, mouse input, or color.
- Native and WASM adapters produce byte-for-byte-equivalent versioned event
  traces for the shared parity corpus.
- Core semantic and golden tests pass, and deterministic examples match GNU
  `as`/`ld` for stdout and exit status on x86-64 Linux.

## Confirmed decisions and choices that remain

Confirmed:

1. The browser visualizer and native terminal REPL/TUI are both first-class
   release surfaces.
2. They share one semantic implementation and one command/event protocol.
3. Given that requirement, a Rust core compiled natively and to WebAssembly is
   the recommended architecture; TypeScript remains a browser presentation
   language rather than a second emulator.
4. No backend is required for release 1.

Two content/distribution choices can be made during scaffolding without
changing the machine design:

1. **Bundled course content:** either copy a deliberately small, versioned set
   of examples into this repository or load them from a sibling CS201 checkout.
   A copied/versioned set makes browser and binary releases reproducible; a
   read-only sync/check script can detect drift from the course repository.
2. **Browser share links:** source-in-URL is convenient but assignment code can
   be large. A compressed URL with an explicit size limit is preferable to
   introducing a server/database.

The recommended default is a small versioned lesson corpus, a read-only drift
checker, compressed size-limited browser links, and no backend.
