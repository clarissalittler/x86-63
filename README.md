# x86-63

`x86-63` is a deliberately small, source-level x86-64 teaching machine for
CS201. It runs GNU/AT&T-syntax assembly one instruction at a time and explains
the resulting register, flag, and process-state changes.

The course-shaped teaching slices now cover 25 maintained lessons in the
Lecture 3–6 progression, the complete Assignment 2 `sum5` workspace, nine
complete programs from the CS201 assembly guide, and five repair labs,
spanning register arithmetic, data,
addressing, branches, loops, process I/O, linked helper modules, routines,
calls, stack frames, integer conversion, and recursion. (Lecture 4's annotated
repeat of `firstadd.s` is represented by the same Lecture 3 lesson.)
Both interfaces run over one Rust execution core:

- a React/TypeScript browser visualizer using the core through WebAssembly;
- a native Rust binary with a full-screen TUI, line-oriented REPL, and
  scriptable `run`, `exec`, `check`, and JSONL `trace` modes.

Both interfaces have exact `u64` register behavior, all general-purpose
register aliases, little-endian `.data`/`.rodata`/`.bss` memory, a mapped stack,
arithmetic flags, signed and unsigned branches, `call`/`ret`/`push`/`pop`/
`leave`, zero extension, increment/decrement, signed multiplication, signed
and unsigned division with `cqo`/`cltq` sign extension, negation, every
condition-code spelling of the conditional jumps (`jz`, `jnz`, `js`, `jc`, …),
numeric local labels (`1:` with `1f`/`1b`), parenthesized equate and immediate
expressions such as `(. - arr) / 8`, Linux `read`/`write`/`exit`, multi-module
`.extern` linking,
strict source diagnostics, structured explanations, reset, reversible stepping,
and real step-over for calls. Memory and stack writes, input consumption,
branch decisions, process output, and recursive frame chains are reversible
too. The browser includes guided register, memory, stack, and recursion
tutorials; focused **Learn** and full-state **Inspect** modes; signed,
unsigned, binary, and ASCII interpretations; regroupable memory; prediction
journaling; progressive challenge hints; and downloadable execution traces.
Every surface accepts the guide's gdb vocabulary through one shared command
layer: `info registers`, `p/x $rax`, `x/8xb &buff`, `x/s &msg`, `x/4gx $rsp`,
`x/3i $pc`, `bt`, `info frame`, `break LABEL|LINE|*ADDR`, and a reversible
`set $reg = EXPR`, in the REPL, after `:` in the TUI, and in the browser's
**Debugger console**. Command scripts replay with `repl -x FILE`, `source
FILE`, or the console's **Run script…**.
Students can set source breakpoints from the assembled gutter, manage linked
modules locally, export/import complete workspaces, and run Assignment 2's
five separate input interactions against an evidence-backed rubric checklist.

The maintained Lecture 3–6 compositions, Assignment 2, and every complete
program in the assembly guide now run end to end, including the real
`readInt.s`, `writeInt.s`, recursive `fact.s`, and `sum5.s`; this completes the
Tier 1 course-writing subset. The remaining work is cross-surface classroom
polish (Slice 5 of the design). Unsupported syntax is rejected explicitly
instead of being guessed or silently repaired.

## Try the native version

With Rust 1.85 or newer:

```sh
cargo run -p x86-63-cli -- tui --example firstadd
```

The default command opens the same TUI. The plain REPL is useful over a basic
terminal or with a screen reader:

```sh
cargo run -p x86-63-cli -- repl --example firstadd
```

Other useful commands:

```sh
cargo run -p x86-63-cli -- examples
cargo run -p x86-63-cli -- check course-content/lecture4/addArray3.s
cargo run -p x86-63-cli -- run --example hello
cargo run -p x86-63-cli -- run --example echo --stdin hello
cargo run -p x86-63-cli -- tui --example funstack
cargo run -p x86-63-cli -- tui --example fact --stdin 5
cargo run -p x86-63-cli -- run --example readwrite --stdin 123
cargo run -p x86-63-cli -- repl --example sum5
cargo run -p x86-63-cli -- trace --example firstsub
cargo run -p x86-63-cli -- exec --example guidesumn
cargo run -p x86-63-cli -- repl --example fact -x tests/scripts/fact.gdb --batch
cargo run -p x86-63-cli -- trace --example fact --commands tests/scripts/fact.gdb
```

`exec` behaves like the assembled executable: program output goes straight to
stdout/stderr, each blocked `read` takes one line from stdin (as a terminal
would deliver it), and the process exits with the program's shell status.

## Try the browser version

The interactive visualizer is published at
[clarissalittler.github.io/x86-63](https://clarissalittler.github.io/x86-63/).
Every push to `main` tests and redeploys the site through GitHub Pages.

The web build needs Node/npm, the Rust WASM target, and the matching
`wasm-bindgen` CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126 --locked
npm install
npm run web:dev
```

Open the local address printed by Vite. Choose **Guided tutorial** for registers
and arithmetic, **Memory tutorial** for `lea`, scaled indexing, little-endian
data, and reversible writes, **Stack tutorial** for calls, return addresses and
local variables, or **Recursion tutorial** for a live `%rbp` frame chain.
Linked lessons expose a source-module chooser and automatically follow control
flow into `readInt.s` and `writeInt.s`. A static production build is created
with `npm run web:build`; it has no server-side runtime dependency.

Click the breakpoint circle beside any executable assembled line, then use
**Run** to stop before it. **Reset** preserves breakpoints; assembling a new
workspace starts a fresh debugger session. The Source workspace controls can
add/remove modules, import local `.s` files or an exported workspace JSON, and
export all linked source modules. Files remain in the browser and are never
uploaded.

Use **Learn** to show only registers used by the active module plus essential
course-role registers. Record a prediction before stepping and the browser can
hold back the resulting machine panes until the student chooses to reveal the
evidence. Use **Inspect** for the complete architectural state. The **Practice
labs** group contains five deliberately broken programs covering immediates,
operand order, access width, caller-saved registers, and stack alignment.
The **Assembly guide** group holds the guide's numeric-local-label example,
its seven exercises, and a short `cqo`/`idivq` program for the guide's
division detour.
The separate **Assignment 2** group bundles `sum5.s` with both I/O helpers.
Its **Run five-line sample** action waits for each blocked read before submitting
10, 20, 30, 40, and 50, then checks the array, procedure calls, output, stack,
and exit evidence.

## Verify the slice

```sh
cargo test --workspace
./scripts/differential-lecture4.sh
./scripts/differential-lecture5.sh
./scripts/differential-lecture6.sh
./scripts/differential-guide.sh
npm run wasm:test
npm run parity:test
npm run web:build
```

The native integration tests and Node/WASM smoke test run all 25 maintained
lessons, Assignment 2, the nine guide programs, and five repair labs,
asserting register, memory, stack, branch, input, output, fault, and
exit outcomes, including linked modules, recursive frame chains, integer-I/O
edge cases, reverse stepping, and 64-bit-safe JSON transport. The differential
scripts assemble the Lecture 4–6 and guide fixtures with GNU `as`/`ld`, run
them, and compare their shell statuses and output bytes with the teaching
machine. `parity:test` replays every command script in `tests/scripts/`
through the native binary and the WebAssembly build and requires
byte-identical JSON responses, events included, for every line.

## Documentation

- [Student tutorial](docs/student-tutorial.md) is the guided first lab and
  includes browser, TUI, and REPL instructions.
- [Course subset](docs/course-subset.md) records what the current course
  materials actually teach and establishes the compatibility target.
- [Product and technical design](docs/product-design.md) specifies the planned
  complete experience and architecture.

The Assignment 2 product test accepts five separate inputs through the real
helpers, fills the array with 10 through 50, calls the summing and output
procedures, and prints `150` followed by a newline.
