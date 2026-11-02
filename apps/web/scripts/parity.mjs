// Replays every tests/scripts/<lesson>.gdb command script through the native
// binary (`x86-63 trace --commands`) and through the WebAssembly build, and
// requires byte-identical JSON responses for every line.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";

import { WasmSession, initSync, lessons_json } from "../src/generated/x86_63_wasm.js";

const root = new URL("../../../", import.meta.url);
const native = new URL("target/debug/x86-63", root).pathname;
const scriptsDirectory = new URL("tests/scripts/", root);

initSync({
  module: readFileSync(new URL("../src/generated/x86_63_wasm_bg.wasm", import.meta.url))
});
const lessons = new Map(JSON.parse(lessons_json()).map((lesson) => [lesson.id, lesson]));

function scriptLines(text) {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("#"));
}

const scripts = readdirSync(scriptsDirectory).filter((name) => name.endsWith(".gdb")).sort();
assert.ok(scripts.length > 0, "no command scripts found");

let lineCount = 0;
let eventCount = 0;
for (const name of scripts) {
  const id = name.replace(/\.gdb$/, "");
  const lesson = lessons.get(id);
  assert.ok(lesson, `${name} does not name a bundled lesson`);
  const path = new URL(name, scriptsDirectory).pathname;

  const nativeLines = execFileSync(native, ["trace", "--example", id, "--commands", path], {
    encoding: "utf8"
  })
    .split("\n")
    .filter((line) => line.length > 0);

  const session = new WasmSession(
    JSON.stringify([
      { name: lesson.module_name, source: lesson.source },
      ...lesson.support_modules.map((module) => ({
        name: module.module_name,
        source: module.source
      }))
    ])
  );
  const lines = scriptLines(readFileSync(path, "utf8"));
  assert.equal(nativeLines.length, lines.length, `${id}: native emitted a different line count`);
  lines.forEach((line, index) => {
    const response = session.repl(line);
    const wasmLine = `{"line":${JSON.stringify(line)},"response":${response}}`;
    if (wasmLine !== nativeLines[index]) {
      assert.fail(
        `${id} line ${index + 1} (${line}) differs:\n native ${nativeLines[index]}\n wasm   ${wasmLine}`
      );
    }
    eventCount += JSON.parse(response).result?.events.length ?? 0;
  });
  lineCount += lines.length;
  session.free();
}

console.log(
  `Native/WASM parity: ${scripts.length} scripts, ${lineCount} commands, and ${eventCount} events are byte-identical.`
);
