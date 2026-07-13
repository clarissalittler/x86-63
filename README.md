# x86-63

`x86-63` is a deliberately small, source-level x86-64 teaching machine for
CS201. It runs GNU/AT&T-syntax assembly one instruction at a time and explains
the resulting register, flag, and process-state changes.

The first two student-testable slices are working. They cover the 15 distinct
programs in the maintained Lecture 3–4 progression, spanning register
arithmetic, data, addressing, branches, loops, and process output. (Lecture 4's
annotated repeat of `firstadd.s` is represented by the same Lecture 3 lesson.)
Both interfaces run over one Rust execution core:

- a React/TypeScript browser visualizer using the core through WebAssembly;
- a native Rust binary with a full-screen TUI, line-oriented REPL, and
  scriptable `run`, `check`, and JSONL `trace` modes.

Both interfaces have exact `u64` register behavior, all general-purpose
register aliases, little-endian data memory, arithmetic flags, signed and
unsigned branches, Linux `write`/`exit`, strict source diagnostics, structured
explanations, reset, and reversible stepping. Memory writes, branch decisions,
and process output are reversible too. The browser includes guided register and
memory tutorials.

This is not yet the full Lectures 3–6 target described in the design. Calls,
the stack, integer I/O helpers, and the later instruction families are the next
course-shaped slices. Unsupported syntax is rejected explicitly instead of
being guessed or silently repaired.

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
cargo run -p x86-63-cli -- trace --example firstsub
```

## Try the browser version

The web build needs Node/npm, the Rust WASM target, and the matching
`wasm-bindgen` CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126 --locked
npm install
npm run web:dev
```

Open the local address printed by Vite. Choose **Guided tutorial** for registers
and arithmetic, or **Memory tutorial** for `lea`, scaled indexing,
little-endian data, and reversible writes. A static production build is created
with `npm run web:build`; it has no server-side runtime dependency.

## Verify the slice

```sh
cargo test --workspace
./scripts/differential-lecture4.sh
npm run wasm:test
npm run web:build
```

The native integration tests and Node/WASM smoke test run all 15 maintained
lessons and assert register, memory, branch, output, fault, and exit outcomes,
including reverse stepping and 64-bit-safe JSON transport. The differential
script assembles and runs all 11 new Lecture 4 fixtures with GNU `as`/`ld`,
then compares their shell statuses and output bytes with the teaching machine.

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
