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
  lessons.slice(0, 4).map((lesson) => lesson.id),
  ["first", "firstfixed", "firstadd", "firstsub"]
);
assert.equal(lessons.length, 25);

function lessonModules(lesson) {
  return [
    { name: lesson.module_name, source: lesson.source },
    ...lesson.support_modules.map((module) => ({
      name: module.module_name,
      source: module.source
    }))
  ];
}

const expectedStatus = new Map([
  ["first", { kind: "faulted", code: "fell_off_text" }],
  ["firstfixed", { kind: "exited", shell_status: 255 }],
  ["firstadd", { kind: "exited", shell_status: 30 }],
  ["firstsub", { kind: "exited", shell_status: 10 }],
  ["addglobal", { kind: "exited", shell_status: 210 }],
  ["addglobalbetter", { kind: "exited", shell_status: 210 }],
  ["addgloballea", { kind: "exited", shell_status: 210 }],
  ["addarray1", { kind: "exited", shell_status: 210 }],
  ["addarray2", { kind: "exited", shell_status: 210 }],
  ["addarray3", { kind: "exited", shell_status: 54 }],
  ["addarray4", { kind: "exited", shell_status: 160 }],
  ["cmp1", { kind: "exited", shell_status: 255 }],
  ["sumloop", { kind: "exited", shell_status: 55 }],
  ["sumloopb", { kind: "exited", shell_status: 55 }],
  ["hello", { kind: "exited", shell_status: 0 }],
  ["echo", { kind: "exited", shell_status: 0 }],
  ["helloret", { kind: "exited", shell_status: 14 }],
  ["routine", { kind: "exited", shell_status: 40 }],
  ["fun1", { kind: "exited", shell_status: 80 }],
  ["fun2", { kind: "exited", shell_status: 40 }],
  ["funstack", { kind: "exited", shell_status: 40 }],
  ["readwrite", { kind: "exited", shell_status: 0 }],
  ["facttrace", { kind: "exited", shell_status: 120 }],
  ["fact", { kind: "exited", shell_status: 0 }],
  ["sumlooparray", { kind: "exited", shell_status: 0 }]
]);

for (const lesson of lessons) {
  const session = new WasmSession(JSON.stringify(lessonModules(lesson)));
  const initial = JSON.parse(session.view_json());
  assert.equal(initial.protocol_version, 4);
  assert.equal(initial.status.kind, "paused");

  if (["echo", "readwrite", "fact"].includes(lesson.id)) {
    const text = lesson.id === "fact" ? "5" : lesson.id === "readwrite" ? "123" : "browser";
    session.execute(JSON.stringify({ SubmitInput: { text } }));
  }

  const result = JSON.parse(
    session.execute(JSON.stringify({ Continue: { max_steps: 2_000 } }))
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

const hello = lessons.find((lesson) => lesson.id === "hello");
const output = new WasmSession(
  JSON.stringify([{ name: hello.module_name, source: hello.source }])
);
const helloResult = JSON.parse(
  output.execute(JSON.stringify({ Continue: { max_steps: 100 } }))
);
assert.equal(helloResult.view.io.stdout_escaped, "Hello world!\\n\\0");
assert.deepEqual(helloResult.view.io.stdout_bytes.slice(-2), [10, 0]);
output.free();

const echo = lessons.find((lesson) => lesson.id === "echo");
const interactiveInput = new WasmSession(
  JSON.stringify([{ name: echo.module_name, source: echo.source }])
);
const waiting = JSON.parse(
  interactiveInput.execute(JSON.stringify({ Continue: { max_steps: 100 } }))
);
assert.equal(waiting.view.status.kind, "waiting_for_input");
assert.equal(waiting.view.next_text.trim(), "syscall");
interactiveInput.execute(JSON.stringify({ SubmitInput: { text: "hello" } }));
const echoed = JSON.parse(
  interactiveInput.execute(JSON.stringify({ Continue: { max_steps: 100 } }))
);
assert.equal(echoed.view.io.stdin_escaped, "hello\\n");
assert.equal(echoed.view.io.stdin_consumed, 6);
assert.equal(echoed.view.io.stdout_escaped, "hello\\n");
interactiveInput.free();

const funStack = lessons.find((lesson) => lesson.id === "funstack");
const stack = new WasmSession(JSON.stringify(lessonModules(funStack)));
stack.execute(JSON.stringify("Step"));
const call = JSON.parse(stack.execute(JSON.stringify("Step")));
assert.equal(call.view.stack.slots.length, 1);
assert.match(call.view.stack.slots[0].label, /return to funStack\.s:21/);
assert.equal(typeof call.view.stack.slots[0].value, "string");
stack.free();

const factorial = lessons.find((lesson) => lesson.id === "fact");
assert.deepEqual(
  factorial.support_modules.map((module) => module.module_name),
  ["readInt.s", "writeInt.s"]
);
const linkedFactorial = new WasmSession(JSON.stringify(lessonModules(factorial)));
linkedFactorial.execute(JSON.stringify({ SubmitInput: { text: "5" } }));
const factorialResult = JSON.parse(
  linkedFactorial.execute(JSON.stringify({ Continue: { max_steps: 2_000 } }))
);
assert.equal(factorialResult.view.io.stdout_escaped, "Enter a number: 120");
linkedFactorial.free();

const factTrace = lessons.find((lesson) => lesson.id === "facttrace");
const recursion = new WasmSession(JSON.stringify(lessonModules(factTrace)));
let recursiveView = JSON.parse(recursion.view_json());
for (let step = 0; step < 100 && recursiveView.stack.frames.length < 5; step += 1) {
  recursiveView = JSON.parse(recursion.execute(JSON.stringify("Step"))).view;
}
assert.equal(recursiveView.stack.frames.length, 5);
assert.ok(recursiveView.stack.frames.every((frame) => frame.function === "fact"));
assert.ok(recursiveView.stack.frames.every((frame) => frame.aligned_at_call));
recursion.free();

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

console.log("WASM protocol smoke test passed for all 25 Lecture 3–6 lessons.");
