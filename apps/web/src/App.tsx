import { useEffect, useMemo, useRef, useState } from "react";
import init, { WasmSession, lessons_json } from "./generated/x86_63_wasm";
import type {
  CommandResult,
  Diagnostic,
  Lesson,
  MachineStatus,
  MachineView,
  ProgramView,
  StepEvent
} from "./types";

type Command =
  | "Reset"
  | "Step"
  | "Next"
  | "Back"
  | { Continue: { max_steps: number } }
  | { SubmitInput: { text: string } };

type TutorialStep = {
  title: string;
  instruction: string;
  expected: string;
  check: (view: MachineView) => boolean;
};

type TutorialKind = "registers" | "memory" | "functions";

const emptyFlags = { cf: false, pf: false, af: false, zf: false, sf: false, of: false };

export default function App() {
  const session = useRef<WasmSession | null>(null);
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [selectedId, setSelectedId] = useState("firstadd");
  const [moduleName, setModuleName] = useState("firstadd.s");
  const [source, setSource] = useState("");
  const [view, setView] = useState<MachineView | null>(null);
  const [program, setProgram] = useState<ProgramView | null>(null);
  const [result, setResult] = useState<CommandResult | null>(null);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [bootError, setBootError] = useState<string | null>(null);
  const [tutorialOpen, setTutorialOpen] = useState(false);
  const [tutorialKind, setTutorialKind] = useState<TutorialKind>("registers");
  const [tutorialStep, setTutorialStep] = useState(0);
  const [tutorialFeedback, setTutorialFeedback] = useState<string | null>(null);
  const [stdinDraft, setStdinDraft] = useState("");

  useEffect(() => {
    let active = true;
    void init()
      .then(() => {
        if (!active) return;
        const loaded = JSON.parse(lessons_json()) as Lesson[];
        setLessons(loaded);
        const initial = loaded.find((lesson) => lesson.id === "firstadd") ?? loaded[0];
        if (initial) {
          setSelectedId(initial.id);
          setModuleName(initial.module_name);
          setSource(initial.source);
          loadSession(initial.module_name, initial.source);
        }
      })
      .catch((error: unknown) => {
        if (active) setBootError(String(error));
      });
    return () => {
      active = false;
      session.current?.free();
      session.current = null;
    };
  }, []);

  const loadSession = (name: string, nextSource: string) => {
    try {
      session.current?.free();
      session.current = new WasmSession(JSON.stringify([{ name, source: nextSource }]));
      setView(JSON.parse(session.current.view_json()) as MachineView);
      setProgram(JSON.parse(session.current.program_json()) as ProgramView);
      setResult(null);
      setDiagnostics([]);
      setStdinDraft("");
    } catch (error: unknown) {
      session.current = null;
      setView(null);
      setProgram(null);
      setResult(null);
      setDiagnostics(parseBuildDiagnostics(error));
    }
  };

  const execute = (command: Command) => {
    if (!session.current) return;
    try {
      const next = JSON.parse(session.current.execute(JSON.stringify(command))) as CommandResult;
      setResult(next);
      setView(next.view);
      setDiagnostics(next.diagnostics);
      setTutorialFeedback(null);
    } catch (error: unknown) {
      setBootError(String(error));
    }
  };

  const chooseLesson = (id: string) => {
    const lesson = lessons.find((candidate) => candidate.id === id);
    if (!lesson) return;
    setSelectedId(id);
    setModuleName(lesson.module_name);
    setSource(lesson.source);
    loadSession(lesson.module_name, lesson.source);
    setTutorialOpen(false);
    setTutorialFeedback(null);
  };

  const startTutorial = (kind: TutorialKind = "registers") => {
    const lessonId = kind === "memory" ? "addarray3" : kind === "functions" ? "funstack" : "firstadd";
    const lesson = lessons.find((candidate) => candidate.id === lessonId);
    if (!lesson) return;
    setSelectedId(lesson.id);
    setModuleName(lesson.module_name);
    setSource(lesson.source);
    loadSession(lesson.module_name, lesson.source);
    setTutorialKind(kind);
    setTutorialStep(0);
    setTutorialFeedback(null);
    setTutorialOpen(true);
  };

  const checkTutorial = () => {
    if (!view) return;
    const steps = tutorialSteps(tutorialKind);
    const step = steps[tutorialStep] ?? steps[0]!;
    setTutorialFeedback(
      step.check(view)
        ? `Looks right. ${step.expected}`
        : `Not yet. ${step.expected} If you got ahead, choose Start over.`
    );
  };

  const selectedLesson = lessons.find((lesson) => lesson.id === selectedId);
  const changed = useMemo(
    () =>
      new Set(
        (result?.events ?? [])
          .filter((event) => event.kind === "register_write")
          .map((event) => event.canonical ?? "")
      ),
    [result]
  );
  const changedMemory = useMemo(() => {
    const offsets = new Set<number>();
    if (!view) return offsets;
    for (const event of result?.events ?? []) {
      if (event.kind !== "memory_write" || !event.address || !event.width) continue;
      const relative = BigInt(event.address) - BigInt(view.memory.base);
      if (relative < 0n || relative >= BigInt(view.memory.bytes.length)) continue;
      const start = Number(relative);
      for (let offset = start; offset < start + event.width / 8; offset += 1) {
        offsets.add(offset);
      }
    }
    return offsets;
  }, [result, view]);
  const changedStack = useMemo(
    () =>
      new Set(
        (result?.events ?? [])
          .filter((event) => event.kind === "memory_write" && event.address)
          .map((event) => event.address as string)
      ),
    [result]
  );

  const submitInput = () => {
    execute({ SubmitInput: { text: stdinDraft } });
    setStdinDraft("");
  };

  if (bootError) {
    return <main className="fatal">Could not start x86-63: {bootError}</main>;
  }

  return (
    <main className="app-shell">
      <header className="masthead">
        <div>
          <p className="eyebrow">CS201 teaching machine</p>
          <h1>x86-63</h1>
        </div>
        <p className="tagline">One instruction at a time, with receipts.</p>
      </header>

      <section className="lesson-bar" aria-label="Lesson and execution controls">
        <label>
          Example
          <select value={selectedId} onChange={(event) => chooseLesson(event.target.value)}>
            {Array.from(new Set(lessons.map((lesson) => lesson.lecture)))
              .sort((left, right) => left - right)
              .map((lecture) => (
                <optgroup label={`Lecture ${lecture}`} key={lecture}>
                  {lessons
                    .filter((lesson) => lesson.lecture === lecture)
                    .map((lesson) => (
                      <option key={lesson.id} value={lesson.id}>
                        {lesson.title}
                      </option>
                    ))}
                </optgroup>
              ))}
          </select>
        </label>
        <div className="control-group">
          <button
            className="tutorial-trigger"
            onClick={() => startTutorial("registers")}
            disabled={!lessons.length}
          >
            Guided tutorial
          </button>
          <button
            className="tutorial-trigger memory-tutorial-trigger"
            onClick={() => startTutorial("memory")}
            disabled={!lessons.length}
          >
            Memory tutorial
          </button>
          <button
            className="tutorial-trigger functions-tutorial-trigger"
            onClick={() => startTutorial("functions")}
            disabled={!lessons.length}
          >
            Stack tutorial
          </button>
          <button onClick={() => loadSession(moduleName, source)} disabled={!lessons.length}>
            Assemble
          </button>
          <button onClick={() => execute("Reset")} disabled={!session.current}>
            Reset
          </button>
          <button
            onClick={() => execute("Back")}
            disabled={
              !session.current ||
              (!view?.history_depth && view?.status.kind !== "waiting_for_input")
            }
          >
            ← Back
          </button>
          <button className="primary" onClick={() => execute("Step")} disabled={!canStep(view)}>
            Step
          </button>
          <button onClick={() => execute("Next")} disabled={!canStep(view)}>
            Next
          </button>
          <button
            onClick={() => execute({ Continue: { max_steps: 10_000 } })}
            disabled={!canStep(view)}
          >
            Run
          </button>
        </div>
      </section>

      {selectedLesson && (
        <section className="prediction">
          <strong>Lecture {selectedLesson.lecture} · Before you step:</strong>{" "}
          {selectedLesson.prediction}
        </section>
      )}

      {tutorialOpen && (
        <Tutorial
          title={
            tutorialKind === "memory"
              ? "Memory and addressing"
              : tutorialKind === "functions"
                ? "Calls and stack frames"
                : "Registers and arithmetic"
          }
          steps={tutorialSteps(tutorialKind)}
          step={tutorialStep}
          feedback={tutorialFeedback}
          onCheck={checkTutorial}
          onMove={(offset) => {
            const length = tutorialSteps(tutorialKind).length;
            setTutorialStep((current) =>
              Math.max(0, Math.min(length - 1, current + offset))
            );
            setTutorialFeedback(null);
          }}
          onRestart={() => startTutorial(tutorialKind)}
          onClose={() => setTutorialOpen(false)}
        />
      )}

      <section className="workspace">
        <div className="source-column panel">
          <div className="panel-heading">
            <h2>Source</h2>
            <input
              aria-label="Module name"
              value={moduleName}
              onChange={(event) => setModuleName(event.target.value)}
            />
          </div>
          <textarea
            className="source-editor"
            aria-label="Assembly source"
            spellCheck={false}
            value={source}
            onChange={(event) => setSource(event.target.value)}
          />
          <SourceListing program={program} view={view} />
        </div>

        <div className="machine-column">
          <section className="panel registers-panel">
            <div className="panel-heading">
              <h2>Registers</h2>
              <StatusBadge status={view?.status} />
            </div>
            <div className="register-grid">
              {(view?.registers ?? []).map((register) => (
                <div className={`register ${changed.has(register.name) ? "changed" : ""}`} key={register.name}>
                  <strong>%{register.name}</strong>
                  <code>{register.hex}</code>
                  <small>{register.signed}</small>
                </div>
              ))}
            </div>
          </section>

          <section className="panel flags-panel">
            <div>
              <h2>Flags</h2>
              <div className="flags">
                {Object.entries(view?.flags ?? emptyFlags).map(([name, value]) => (
                  <span key={name} className={value ? "set" : ""}>
                    {name.toUpperCase()} {value ? 1 : 0}
                  </span>
                ))}
              </div>
            </div>
            <div className="history">history: {view?.history_depth ?? 0}</div>
          </section>

          {view && view.memory.bytes.length > 0 && (
            <MemoryPanel memory={view.memory} changed={changedMemory} />
          )}

          {view && <StackPanel stack={view.stack} changed={changedStack} />}

          {view && (
            <ProcessIoPanel
              io={view.io}
              value={stdinDraft}
              onChange={setStdinDraft}
              onSubmit={submitInput}
              halted={view.status.kind === "exited" || view.status.kind === "faulted"}
            />
          )}

          <section className="panel explanation-panel" aria-live="polite">
            <p className="eyebrow">This step</p>
            <h2>{result?.explanation ?? "Ready at _start."}</h2>
            <EventSummary events={result?.events ?? []} />
          </section>

          {diagnostics.length > 0 && <Diagnostics diagnostics={diagnostics} />}
        </div>
      </section>
    </main>
  );
}

function Tutorial({
  title,
  steps,
  step,
  feedback,
  onCheck,
  onMove,
  onRestart,
  onClose
}: {
  title: string;
  steps: TutorialStep[];
  step: number;
  feedback: string | null;
  onCheck: () => void;
  onMove: (offset: number) => void;
  onRestart: () => void;
  onClose: () => void;
}) {
  const current = steps[step] ?? steps[0]!;
  return (
    <section className="tutorial panel" aria-label="Guided tutorial">
      <div className="tutorial-copy">
        <p className="eyebrow">
          Guided tutorial · {title} · {step + 1} of {steps.length}
        </p>
        <h2>{current.title}</h2>
        <p>{current.instruction}</p>
        {feedback && <p className="tutorial-feedback" aria-live="polite">{feedback}</p>}
      </div>
      <div className="tutorial-controls">
        <button onClick={() => onMove(-1)} disabled={step === 0}>Previous</button>
        <button onClick={onCheck}>Check my screen</button>
        {step < steps.length - 1 ? (
          <button className="primary" onClick={() => onMove(1)}>Next instruction</button>
        ) : (
          <button className="primary" onClick={onClose}>Finish</button>
        )}
        <button onClick={onRestart}>Start over</button>
      </div>
    </section>
  );
}

function SourceListing({ program, view }: { program: ProgramView | null; view: MachineView | null }) {
  const module = program?.modules[0];
  if (!module) return null;
  return (
    <ol className="source-listing" aria-label="Assembled source with current instruction">
      {module.source.split("\n").map((line, index) => {
        const lineNumber = index + 1;
        const current = view?.next_instruction?.module === module.name && view.next_instruction.line === lineNumber;
        return (
          <li className={current ? "current" : ""} key={`${lineNumber}-${line}`}>
            <code>{line || " "}</code>
          </li>
        );
      })}
    </ol>
  );
}

function EventSummary({ events }: { events: StepEvent[] }) {
  const useful = events.filter((event) =>
    [
      "register_write",
      "effective_address",
      "memory_read",
      "memory_write",
      "arithmetic",
      "compare",
      "branch",
      "call",
      "return",
      "stack_push",
      "stack_pop",
      "input_requested",
      "input_submitted",
      "input_read",
      "output",
      "exit",
      "fault"
    ].includes(event.kind)
  );
  if (useful.length === 0) return null;
  return (
    <ul className="event-list">
      {useful.map((event, index) => (
        <li key={`${event.kind}-${index}`}>{eventText(event)}</li>
      ))}
    </ul>
  );
}

function eventText(event: StepEvent): string {
  switch (event.kind) {
    case "register_write":
      return `%${event.register}: ${event.before} → ${event.after}`;
    case "arithmetic":
      return `${event.operation}: ${event.left} and ${event.right} produced ${event.result}`;
    case "effective_address":
      return `${event.expression} resolves to ${event.address}${event.symbol ? ` (${event.symbol})` : ""}`;
    case "memory_read":
      return `read ${event.width} bits at ${event.address}: ${event.value}`;
    case "memory_write":
      return `wrote ${event.width} bits at ${event.address}: ${event.before} → ${event.after}`;
    case "compare":
      return `cmp: ${event.destination} − ${event.source} = ${event.result}`;
    case "branch":
      return `${event.condition} → ${event.target}: ${event.predicate}; ${event.taken ? "taken" : "not taken"}`;
    case "call":
      return `call ${event.target}: pushed return address ${event.return_address}`;
    case "return":
      return `ret: popped ${event.return_address}${event.return_location ? ` → ${event.return_location.module}:${event.return_location.line}` : ""}`;
    case "stack_push":
      return `push ${event.value}; %rsp is now ${event.stack_pointer}`;
    case "stack_pop":
      return `pop ${event.value}; %rsp is now ${event.stack_pointer}`;
    case "input_requested":
      return `read is waiting for up to ${event.count} bytes of stdin at ${event.address}`;
    case "input_submitted":
      return `submitted stdin line: ${event.escaped}`;
    case "input_read":
      return `read consumed: ${event.escaped}`;
    case "output":
      return `write(fd=${event.fd}): ${event.escaped}`;
    case "exit":
      return `The shell-visible status is ${event.shell_status}.`;
    case "fault":
      return `${event.code}: ${event.message}`;
    default:
      return event.kind;
  }
}

function MemoryPanel({
  memory,
  changed
}: {
  memory: MachineView["memory"];
  changed: Set<number>;
}) {
  return (
    <section className="panel memory-panel">
      <div className="panel-heading">
        <h2>Memory</h2>
        <span className="memory-size">mapped data · {memory.bytes.length} bytes</span>
      </div>
      <div className="memory-symbols">
        {memory.symbols.map((symbol) => {
          const bytes = memory.bytes.slice(symbol.offset, symbol.offset + symbol.size);
          const compactByteBuffer = symbol.element_width === 1 && symbol.size > 32;
          const displayWidth = compactByteBuffer ? 8 : symbol.element_width;
          const elements = chunk(bytes, displayWidth);
          const elementCount = Math.ceil(symbol.size / symbol.element_width);
          return (
            <section className="memory-symbol" key={symbol.name}>
              <header>
                <strong>{symbol.name}</strong>
                <code>{symbol.address}</code>
                <small>
                  {symbol.section} · {directiveName(symbol.element_width)} × {elementCount}
                </small>
              </header>
              <div className="memory-elements">
                {elements.map((element, index) => {
                  const byteOffset = index * displayWidth;
                  const offset = symbol.offset + byteOffset;
                  const isChanged = element.some((_, byte) => changed.has(offset + byte));
                  return (
                    <div className={`memory-element ${isChanged ? "changed" : ""}`} key={offset}>
                      <small>
                        {compactByteBuffer ? `[+${byteOffset}]` : `[${index}]`} {addressAt(memory.base, offset)}
                      </small>
                      <code>{element.map(hexByte).join(" ")}</code>
                      <strong>
                        {compactByteBuffer ? `text “${renderBytes(element)}”` : littleEndianSigned(element)}
                      </strong>
                    </div>
                  );
                })}
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}

function StackPanel({
  stack,
  changed
}: {
  stack: MachineView["stack"];
  changed: Set<string>;
}) {
  return (
    <section className="panel stack-panel">
      <div className="panel-heading">
        <h2>Stack</h2>
        <span className="memory-size">
          %rsp {stack.rsp} · %rbp {stack.rbp}
        </span>
      </div>
      {stack.slots.length === 0 ? (
        <p className="empty-state">The stack is empty at its initial aligned top, {stack.top}.</p>
      ) : (
        <div className="stack-slots">
          {stack.slots.map((slot) => (
            <div
              className={`stack-slot ${changed.has(slot.address) ? "changed" : ""}`}
              key={slot.address}
            >
              <code>{slot.address}</code>
              <strong>{slot.value}</strong>
              <small>{slot.label ?? `signed ${slot.signed}`}</small>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function ProcessIoPanel({
  io,
  value,
  onChange,
  onSubmit,
  halted
}: {
  io: MachineView["io"];
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  halted: boolean;
}) {
  return (
    <section className="panel output-panel">
      <div className="panel-heading">
        <h2>Process I/O</h2>
        <span className="memory-size">input is submitted one line at a time</span>
      </div>
      <form
        className="stdin-form"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <label htmlFor="stdin-line">stdin line</label>
        <input
          id="stdin-line"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          disabled={halted}
          placeholder="for example: hello"
        />
        <button type="submit" disabled={halted}>Submit line</button>
      </form>
      {io.stdin_bytes.length > 0 && (
        <div className="output-stream input-stream">
          <strong>stdin · {io.stdin_consumed}/{io.stdin_bytes.length} bytes consumed</strong>
          <pre>{renderBytes(io.stdin_bytes)}</pre>
          <code>{io.stdin_escaped}</code>
        </div>
      )}
      {io.stdout_bytes.length > 0 && (
        <div className="output-stream">
          <strong>stdout</strong>
          <pre>{renderBytes(io.stdout_bytes)}</pre>
          <code>{io.stdout_escaped}</code>
        </div>
      )}
      {io.stderr_bytes.length > 0 && (
        <div className="output-stream">
          <strong>stderr</strong>
          <pre>{renderBytes(io.stderr_bytes)}</pre>
          <code>{io.stderr_escaped}</code>
        </div>
      )}
    </section>
  );
}

function chunk(values: number[], width: number): number[][] {
  const result: number[][] = [];
  for (let index = 0; index < values.length; index += width) {
    result.push(values.slice(index, index + width));
  }
  return result;
}

function directiveName(width: number): string {
  return ({ 1: ".byte", 2: ".word", 4: ".long", 8: ".quad" } as Record<number, string>)[width] ?? `${width}B`;
}

function addressAt(base: string, offset: number): string {
  return `0x${(BigInt(base) + BigInt(offset)).toString(16).padStart(16, "0")}`;
}

function hexByte(value: number): string {
  return value.toString(16).padStart(2, "0");
}

function littleEndianSigned(bytes: number[]): string {
  let value = 0n;
  bytes.forEach((byte, index) => {
    value |= BigInt(byte) << BigInt(index * 8);
  });
  const bits = BigInt(bytes.length * 8);
  const sign = 1n << (bits - 1n);
  return (value & sign ? value - (1n << bits) : value).toString();
}

function renderBytes(bytes: number[]): string {
  return bytes
    .map((byte) => {
      if (byte === 0) return "␀";
      if (byte === 10) return "\n";
      if (byte === 13) return "\r";
      if (byte === 9) return "\t";
      return byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : `�`;
    })
    .join("");
}

function Diagnostics({ diagnostics }: { diagnostics: Diagnostic[] }) {
  return (
    <section className="panel diagnostics" aria-live="assertive">
      <p className="eyebrow">Assembler diagnostics</p>
      {diagnostics.map((diagnostic) => (
        <article key={`${diagnostic.code}-${diagnostic.location?.line ?? 0}`}>
          <strong>
            {diagnostic.code}
            {diagnostic.location ? ` at line ${diagnostic.location.line}` : ""}
          </strong>
          <p>{diagnostic.message}</p>
          {diagnostic.help && <small>Try: {diagnostic.help}</small>}
        </article>
      ))}
    </section>
  );
}

function StatusBadge({ status }: { status: MachineStatus | undefined }) {
  if (!status) return <span className="status">loading</span>;
  if (status.kind === "paused") return <span className="status paused">paused</span>;
  if (status.kind === "waiting_for_input") {
    return <span className="status waiting">waiting for stdin</span>;
  }
  if (status.kind === "exited") {
    return <span className="status exited">exited · shell {status.shell_status}</span>;
  }
  return <span className="status faulted">faulted · {status.code}</span>;
}

function canStep(view: MachineView | null): boolean {
  return view?.status.kind === "paused";
}

function parseBuildDiagnostics(error: unknown): Diagnostic[] {
  try {
    const parsed = JSON.parse(String(error)) as { diagnostics?: Diagnostic[] };
    if (parsed.diagnostics) return parsed.diagnostics;
  } catch {
    // The fallback below preserves unexpected adapter errors for the student.
  }
  return [
    {
      severity: "error",
      code: "E-WASM",
      message: String(error),
      help: null,
      location: null
    }
  ];
}

function registerUnsigned(view: MachineView, name: string): string | undefined {
  return view.registers.find((register) => register.name === name)?.unsigned;
}

function memoryUnsigned(view: MachineView, symbolName: string, index: number): string | undefined {
  const symbol = view.memory.symbols.find((candidate) => candidate.name === symbolName);
  if (!symbol) return undefined;
  const offset = symbol.offset + index * symbol.element_width;
  const bytes = view.memory.bytes.slice(offset, offset + symbol.element_width);
  if (bytes.length !== symbol.element_width) return undefined;
  let value = 0n;
  bytes.forEach((byte, byteIndex) => {
    value |= BigInt(byte) << BigInt(byteIndex * 8);
  });
  return value.toString();
}

function stackUnsigned(view: MachineView, offsetFromRbp: number): string | undefined {
  const slot = view.stack.slots.find((candidate) => candidate.offset_from_rbp === offsetFromRbp);
  return slot ? BigInt(slot.value).toString() : undefined;
}

function tutorialSteps(kind: TutorialKind): TutorialStep[] {
  if (kind === "memory") return memoryTutorialSteps;
  if (kind === "functions") return functionTutorialSteps;
  return registerTutorialSteps;
}

const registerTutorialSteps: TutorialStep[] = [
  {
    title: "Find the next instruction",
    instruction:
      "The yellow source row is what Step will execute—not what just ran. Find `mov $10,%rbx`, then check your screen.",
    expected: "The machine should be paused at line 5 with history 0.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 0 &&
      view.next_instruction?.line === 5
  },
  {
    title: "Move an immediate into %rbx",
    instruction:
      "Click Step once. Watch %rbx change, read the explanation, and notice that the yellow row advances.",
    expected: "%rbx should be 10, history should be 1, and line 6 should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 1 &&
      registerUnsigned(view, "rbx") === "10" &&
      view.next_instruction?.line === 6
  },
  {
    title: "Load the destination value",
    instruction:
      "Click Step again. In AT&T syntax, `$20` is the source and `%rcx` is the destination.",
    expected: "%rcx should be 20 while %rbx remains 10.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      registerUnsigned(view, "rbx") === "10" &&
      registerUnsigned(view, "rcx") === "20"
  },
  {
    title: "Predict, then add",
    instruction:
      "Before clicking Step, predict which register changes for `add %rbx,%rcx`. Then step and compare with the explanation.",
    expected: "%rcx should be 30, %rbx should remain 10, and PF should be 1.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 3 &&
      registerUnsigned(view, "rbx") === "10" &&
      registerUnsigned(view, "rcx") === "30" &&
      view.flags.pf
  },
  {
    title: "Reverse one real transition",
    instruction:
      "Click Back. Reverse stepping restores machine state; it does not merely move the yellow highlight.",
    expected: "%rcx should be 20 again and history should return to 2.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      registerUnsigned(view, "rcx") === "20"
  },
  {
    title: "Replay deterministically",
    instruction:
      "Click Step once more. The same inputs produce the same register value and flags.",
    expected: "%rcx should be 30 again and history should be 3.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 3 &&
      registerUnsigned(view, "rcx") === "30"
  },
  {
    title: "Run to the system call",
    instruction:
      "Click Run. The remaining moves prepare Linux `exit`, then `syscall` halts the teaching machine.",
    expected: "The badge should say exited · shell 30, with %rdi containing 30.",
    check: (view) =>
      view.status.kind === "exited" &&
      view.status.shell_status === 30 &&
      registerUnsigned(view, "rdi") === "30"
  }
];

const memoryTutorialSteps: TutorialStep[] = [
  {
    title: "Read the data before the code",
    instruction:
      "Find `num` in the Memory panel. The `.quad` directive made four eight-byte little-endian elements. Do not step yet.",
    expected: "Memory should contain 200, 300, 400, and 500; line 13 should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 0 &&
      view.next_instruction?.line === 13 &&
      ["200", "300", "400", "500"].every(
        (value, index) => memoryUnsigned(view, "num", index) === value
      )
  },
  {
    title: "Load an address, not an element",
    instruction:
      "Click Step for `lea num(%rip),%rbx`. Compare %rbx with the address printed beside `num` in Memory.",
    expected: "%rbx should contain 4194304 (0x400000), while num[0] remains 200.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 1 &&
      registerUnsigned(view, "rbx") === "4194304" &&
      memoryUnsigned(view, "num", 0) === "200"
  },
  {
    title: "Choose an index",
    instruction:
      "Step once more. `%rcx = 1` will become the index in `(%rbx,%rcx,8)`.",
    expected: "%rcx should be 1 and line 17, the memory add, should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      registerUnsigned(view, "rcx") === "1" &&
      view.next_instruction?.line === 17
  },
  {
    title: "Evaluate base + index × scale",
    instruction:
      "Predict the address first: 0x400000 + 1×8. Then Step. The highlighted element and event receipt should agree.",
    expected: "num[1] should change from 300 to 310; the other three elements should be unchanged.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 3 &&
      ["200", "310", "400", "500"].every(
        (value, index) => memoryUnsigned(view, "num", index) === value
      )
  },
  {
    title: "Undo the memory write",
    instruction:
      "Click Back. Reverse execution restores the bytes themselves, not only the source highlight.",
    expected: "num[1] should be 300 again and history should return to 2.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      memoryUnsigned(view, "num", 1) === "300"
  },
  {
    title: "Replay the same write",
    instruction:
      "Click Step again. The effective address should resolve identically and num[1] should return to 310.",
    expected: "num[1] should be 310 and history should be 3.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 3 &&
      memoryUnsigned(view, "num", 1) === "310"
  },
  {
    title: "Dereference the address",
    instruction:
      "Step over `movq (%rbx,%rcx,8),%rdi`. Parentheses mean read the value stored at the calculated address.",
    expected: "%rdi should contain 310 and memory should still contain 310.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 4 &&
      registerUnsigned(view, "rdi") === "310" &&
      memoryUnsigned(view, "num", 1) === "310"
  },
  {
    title: "Observe the shell boundary",
    instruction:
      "Click Run. Linux receives the full 64-bit value 310, while the shell reports only its low eight bits.",
    expected: "The machine should exit with shell status 54 because 310 mod 256 = 54.",
    check: (view) =>
      view.status.kind === "exited" &&
      view.status.shell_status === 54 &&
      registerUnsigned(view, "rdi") === "310" &&
      memoryUnsigned(view, "num", 1) === "310"
  }
];

const functionTutorialSteps: TutorialStep[] = [
  {
    title: "Begin at the caller",
    instruction:
      "Find `_start` and the highlighted `mov $10,%rdi`. The function body appears earlier in the file, but execution begins at `_start`.",
    expected: "Line 19 should be next, the stack should be empty, and %rsp should equal the stack top.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 0 &&
      view.next_instruction?.line === 19 &&
      view.stack.slots.length === 0 &&
      view.stack.rsp === view.stack.top
  },
  {
    title: "Place the argument",
    instruction: "Click Step. The System V calling convention puts this first integer argument in %rdi.",
    expected: "%rdi should contain 10 and the call on line 20 should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 1 &&
      registerUnsigned(view, "rdi") === "10" &&
      view.next_instruction?.line === 20
  },
  {
    title: "Step into call",
    instruction:
      "Click Step—not Next—on `call fun`. Watch %rsp move down eight bytes and inspect the new return-address slot.",
    expected: "Execution should enter line 6 with one stack slot labeled as the return to line 21.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      view.next_instruction?.line === 6 &&
      view.stack.slots.length === 1 &&
      (view.stack.slots[0]?.label?.includes("return to funStack.s:21") ?? false)
  },
  {
    title: "Save the caller's frame pointer",
    instruction: "Step over `push %rbp`. This is a real eight-byte memory write below the return address.",
    expected: "There should be two active stack slots and line 7 should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 3 &&
      view.next_instruction?.line === 7 &&
      view.stack.slots.length === 2
  },
  {
    title: "Anchor the new frame",
    instruction:
      "Step over `mov %rsp,%rbp`. %rbp now stays fixed while %rsp can move to reserve locals.",
    expected: "%rbp and %rsp should match, and the saved caller %rbp slot should be labeled.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 4 &&
      registerUnsigned(view, "rbp") === registerUnsigned(view, "rsp") &&
      view.stack.slots.some((slot) => slot.label?.includes("saved caller %rbp"))
  },
  {
    title: "Reserve two local quadwords",
    instruction:
      "Step over `sub $16,%rsp`. The stack grows downward, so two new zeroed slots appear at -16(%rbp) and -8(%rbp).",
    expected: "Four slots should be active, including offsets -16 and -8 from %rbp.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 5 &&
      view.next_instruction?.line === 9 &&
      view.stack.slots.length === 4 &&
      stackUnsigned(view, -16) === "0" &&
      stackUnsigned(view, -8) === "0"
  },
  {
    title: "Write the locals",
    instruction:
      "Click Step three times. The first local becomes 2×10 and the second receives the constant 20.",
    expected: "Both -16(%rbp) and -8(%rbp) should contain 20, with line 12 next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 8 &&
      view.next_instruction?.line === 12 &&
      stackUnsigned(view, -16) === "20" &&
      stackUnsigned(view, -8) === "20"
  },
  {
    title: "Reverse a local write",
    instruction: "Click Back once. Reverse execution restores the actual eight stack bytes at -16(%rbp).",
    expected: "The -16 local should return to 0 while the -8 local remains 20.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 7 &&
      stackUnsigned(view, -16) === "0" &&
      stackUnsigned(view, -8) === "20"
  },
  {
    title: "Tear the frame down",
    instruction:
      "Step once to replay the write, then click Run. The epilogue restores %rsp and %rbp; ret consumes the return address.",
    expected: "The program should exit with 40 and the active stack should be empty again.",
    check: (view) =>
      view.status.kind === "exited" &&
      view.status.shell_status === 40 &&
      registerUnsigned(view, "rdi") === "40" &&
      view.stack.slots.length === 0 &&
      view.stack.rsp === view.stack.top
  }
];
