# x86-63

`x86-63` is a deliberately small, source-level x86-64 teaching machine for
CS201. It runs GNU/AT&T-syntax assembly one instruction at a time and explains
the resulting register, flag, and process-state changes.

The first student-testable slice is working. It covers the maintained Lecture
3 progression (`first.s`, `firstfixed.s`, `firstadd.s`, and `firstsub.s`) in two
interfaces over one Rust execution core:

- a React/TypeScript browser visualizer using the core through WebAssembly;
- a native Rust binary with a full-screen TUI, line-oriented REPL, and
  scriptable `run`, `check`, and JSONL `trace` modes.

Both interfaces have exact `u64` register behavior, all general-purpose
register aliases, arithmetic flags, Linux `exit`, strict source diagnostics,
structured explanations, reset, and reversible stepping. The browser also has
an in-app guided tutorial.

This is not yet the full lectures 3–6 target described in the design. Memory,
data sections, branches, calls, the stack, and `read`/`write` are the next
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
cargo run -p x86-63-cli -- check course-content/lecture3/firstadd.s
cargo run -p x86-63-cli -- run --example firstfixed
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

Open the local address printed by Vite and choose **Guided tutorial**. A static
production build is created with `npm run web:build`; it has no server-side
runtime dependency.

## Verify the slice

```sh
cargo test --workspace
npm run wasm:test
npm run web:build
```

The native integration tests and the Node/WASM smoke test run the same four
maintained lessons and assert their expected fault/exit outcomes, including
reverse stepping and 64-bit-safe JSON transport.

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
