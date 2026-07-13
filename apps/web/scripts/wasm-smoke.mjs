import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  WasmSession,
  initSync,
  lessons_json
} from "../src/generated/x86_63_wasm.js";

const wasm = readFileSync(
  new URL("../src/generated/x86_63_wasm_bg.wasm", import.meta.url)
);
initSync({ module: wasm });

const lessons = JSON.parse(lessons_json());
assert.deepEqual(
  lessons.map((lesson) => lesson.id),
  ["first", "firstfixed", "firstadd", "firstsub"]
);

const expectedStatus = new Map([
  ["first", { kind: "faulted", code: "fell_off_text" }],
  ["firstfixed", { kind: "exited", shell_status: 255 }],
  ["firstadd", { kind: "exited", shell_status: 30 }],
  ["firstsub", { kind: "exited", shell_status: 10 }]
]);

for (const lesson of lessons) {
  const session = new WasmSession(
    JSON.stringify([{ name: lesson.module_name, source: lesson.source }])
  );
  const initial = JSON.parse(session.view_json());
  assert.equal(initial.protocol_version, 1);
  assert.equal(initial.status.kind, "paused");

  const result = JSON.parse(
    session.execute(JSON.stringify({ Continue: { max_steps: 100 } }))
  );
  const expected = expectedStatus.get(lesson.id);
  assert.equal(result.view.status.kind, expected.kind, lesson.id);
  if (expected.code) assert.equal(result.view.status.code, expected.code, lesson.id);
  if (expected.shell_status !== undefined) {
    assert.equal(result.view.status.shell_status, expected.shell_status, lesson.id);
  }

  // JSON is the public frontend boundary: 64-bit values must remain strings.
  assert.equal(typeof result.view.registers[0].unsigned, "string");
  session.free();
}

const add = lessons.find((lesson) => lesson.id === "firstadd");
const reversible = new WasmSession(
  JSON.stringify([{ name: add.module_name, source: add.source }])
);
reversible.execute(JSON.stringify({ Continue: { max_steps: 100 } }));
const backedUp = JSON.parse(reversible.execute(JSON.stringify("Back")));
assert.equal(backedUp.view.status.kind, "paused");
assert.equal(backedUp.view.next_text.trim(), "syscall");
assert.equal(
  backedUp.view.registers.find((register) => register.name === "rdi").unsigned,
  "30"
);
reversible.free();

let buildError;
try {
  new WasmSession(
    JSON.stringify([
      {
        name: "mistake.s",
        source: ".text\n.global _start\n_start:\n  mov 60,%rax\n"
      }
    ])
  );
} catch (error) {
  buildError = JSON.parse(String(error));
}
assert.equal(buildError.diagnostics[0].code, "E212");
assert.match(buildError.diagnostics[0].help, /\$60/);

console.log("WASM protocol smoke test passed for all four Lecture 3 lessons.");
