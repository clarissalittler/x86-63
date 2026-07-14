# x86-63

`x86-63` is a deliberately small, source-level x86-64 teaching machine for
CS201. It runs GNU/AT&T-syntax assembly one instruction at a time and explains
the resulting register, flag, and process-state changes.

The first three student-testable slices are working. They cover the 21 distinct
programs in the maintained Lecture 3–5 progression, spanning register
arithmetic, data, addressing, branches, loops, process I/O, routines, calls,
and stack frames. (Lecture 4's annotated repeat of `firstadd.s` is represented
by the same Lecture 3 lesson.)
Both interfaces run over one Rust execution core:

- a React/TypeScript browser visualizer using the core through WebAssembly;
- a native Rust binary with a full-screen TUI, line-oriented REPL, and
  scriptable `run`, `check`, and JSONL `trace` modes.

Both interfaces have exact `u64` register behavior, all general-purpose
register aliases, little-endian `.data`/`.rodata`/`.bss` memory, a mapped stack,
arithmetic flags, signed and unsigned branches, `call`/`ret`/`push`/`pop`, Linux
`read`/`write`/`exit`, strict source diagnostics, structured explanations,
reset, reversible stepping, and real step-over for calls. Memory and stack
writes, input consumption, branch decisions, and process output are reversible
too. The browser includes guided register, memory, and stack tutorials.

This is not yet the full Lectures 3–6 target described in the design. Lecture
6's multi-module `readInt`/`writeInt` helpers, recursive factorial, and later
instruction families are the next course-shaped slice. Unsupported syntax is
rejected explicitly instead of being guessed or silently repaired.

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
data, and reversible writes, or **Stack tutorial** for calls, return addresses,
and local variables. A static production build is created with
`npm run web:build`; it has no server-side runtime dependency.

## Verify the slice

```sh
cargo test --workspace
./scripts/differential-lecture4.sh
./scripts/differential-lecture5.sh
npm run wasm:test
npm run web:build
```

The native integration tests and Node/WASM smoke test run all 21 maintained
lessons and assert register, memory, stack, branch, input, output, fault, and
exit outcomes, including reverse stepping and 64-bit-safe JSON transport. The
differential scripts assemble the Lecture 4 and Lecture 5 fixtures with GNU
`as`/`ld`, run them, and compare their shell statuses and output bytes with the
teaching machine.

## Documentation

- [Student tutorial](docs/student-tutorial.md) is the guided first lab and
  includes browser, TUI, and REPL instructions.
- [Course subset](docs/course-subset.md) records what the current course
  materials actually teach and establishes the compatibility target.
- [Product and technical design](docs/product-design.md) specifies the planned
  complete experience and architecture.

The longer-term product test remains: a student should be able to paste
`fact.s`, add the course's `readInt.s` and `writeInt.s`, enter `5`, and
understand why it prints `120` by stepping through the recursive stack frames.
