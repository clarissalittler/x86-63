# x86-63

`x86-63` is a deliberately small, source-level x86-64 teaching machine for
CS201. It runs GNU/AT&T-syntax assembly one instruction at a time and explains
the resulting register, flag, and process-state changes.

The first four student-testable slices are working. They cover 25 lessons in
the maintained Lecture 3–6 progression, spanning register arithmetic, data,
addressing, branches, loops, process I/O, linked helper modules, routines,
calls, stack frames, integer conversion, and recursion. (Lecture 4's annotated
repeat of `firstadd.s` is represented by the same Lecture 3 lesson.)
Both interfaces run over one Rust execution core:

- a React/TypeScript browser visualizer using the core through WebAssembly;
- a native Rust binary with a full-screen TUI, line-oriented REPL, and
  scriptable `run`, `check`, and JSONL `trace` modes.

Both interfaces have exact `u64` register behavior, all general-purpose
register aliases, little-endian `.data`/`.rodata`/`.bss` memory, a mapped stack,
arithmetic flags, signed and unsigned branches, `call`/`ret`/`push`/`pop`/
`leave`, zero extension, increment/decrement, signed multiplication, unsigned
division, negation, Linux `read`/`write`/`exit`, multi-module `.extern` linking,
strict source diagnostics, structured explanations, reset, reversible stepping,
and real step-over for calls. Memory and stack writes, input consumption,
branch decisions, process output, and recursive frame chains are reversible
too. The browser includes guided register, memory, stack, and recursion
tutorials.

The maintained Lecture 3–6 compositions now run end to end, including the real
`readInt.s`, `writeInt.s`, and recursive `fact.s`. Assignment 2 integration,
breakpoints, and the remaining guide-level instruction variants are the next
course-shaped work. Unsupported syntax is rejected explicitly instead of being
guessed or silently repaired.

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
cargo run -p x86-63-cli -- trace --example firstsub
```

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

## Verify the slice

```sh
cargo test --workspace
./scripts/differential-lecture4.sh
./scripts/differential-lecture5.sh
./scripts/differential-lecture6.sh
npm run wasm:test
npm run web:build
```

The native integration tests and Node/WASM smoke test run all 25 maintained
lessons and assert register, memory, stack, branch, input, output, fault, and
exit outcomes, including linked modules, recursive frame chains, integer-I/O
edge cases, reverse stepping, and 64-bit-safe JSON transport. The differential
scripts assemble the Lecture 4–6 fixtures with GNU `as`/`ld`, run them, and
compare their shell statuses and output bytes with the teaching machine.

## Documentation

- [Student tutorial](docs/student-tutorial.md) is the guided first lab and
  includes browser, TUI, and REPL instructions.
- [Course subset](docs/course-subset.md) records what the current course
  materials actually teach and establishes the compatibility target.
- [Product and technical design](docs/product-design.md) specifies the planned
  complete experience and architecture.

The next whole-program product test is Assignment 2's `sum5.s`: accept five
separate inputs through the real helpers, fill the array, sum it, and print
`150` for the inputs 10 through 50.
