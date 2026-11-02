# Command scripts

Each `<lesson-id>.gdb` file is a REPL command script for the bundled lesson of
the same id. The files serve three purposes:

- students and instructors can replay them with
  `x86-63 repl --example <lesson-id> -x tests/scripts/<lesson-id>.gdb`;
- `x86-63 trace --example <lesson-id> --commands <script>` emits one JSON
  response per line;
- `npm run parity:test` replays every script through both the native binary and
  the WebAssembly build and requires byte-identical JSON for every line.

Blank lines and lines starting with `#` are skipped.
