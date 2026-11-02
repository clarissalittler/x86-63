import { useEffect, useMemo, useRef, useState } from "react";
import init, { WasmSession, lessons_json } from "./generated/x86_63_wasm";
import type {
  Assignment,
  CommandResult,
  Diagnostic,
  Lesson,
  MachineStatus,
  MachineView,
  ProgramView,
  ReplResponse,
  StepEvent
} from "./types";

type Command =
  | "Reset"
  | "Step"
  | "Next"
  | "Back"
  | "ClearBreakpoints"
  | { Continue: { max_steps: number } }
  | { SubmitInput: { text: string } }
  | { ToggleBreakpoint: { module: string; line: number } }
  | { SetRegister: { register: string; value: string } };

type ConsoleEntry = { sequence: number; line: string; output: string };

type TutorialStep = {
  title: string;
  instruction: string;
  expected: string;
  check: (view: MachineView) => boolean;
};

type TutorialKind = "registers" | "memory" | "functions" | "recursion";
type EditableModule = { name: string; source: string };
type TeachingMode = "learn" | "inspect";
type NumberFormat = "signed" | "unsigned" | "binary" | "ascii";
type MemoryFormat = "auto" | "byte" | "word" | "long" | "quad" | "text";
type TraceEntry = {
  sequence: number;
  command: Command | { Console: string };
  result: CommandResult;
};
type WorkspacePayload = {
  format: "x86-63-workspace";
  version: number;
  modules: EditableModule[];
};
type ActivePrediction = {
  prompt: string;
  answer: string;
  startDepth: number;
  location: MachineView["next_instruction"];
  revealed: boolean;
  evidence: string | null;
};
type PredictionRecord = ActivePrediction & {
  endDepth: number;
};

const emptyFlags = { cf: false, pf: false, af: false, zf: false, sf: false, of: false };

function lessonModules(lesson: Lesson): EditableModule[] {
  return [
    { name: lesson.module_name, source: lesson.source },
    ...lesson.support_modules.map((module) => ({
      name: module.module_name,
      source: module.source
    }))
  ];
}

export default function App() {
  const session = useRef<WasmSession | null>(null);
  const fileInput = useRef<HTMLInputElement | null>(null);
  const [lessons, setLessons] = useState<Lesson[]>([]);
  const [selectedId, setSelectedId] = useState("firstadd");
  const [modules, setModules] = useState<EditableModule[]>([]);
  const [activeModuleName, setActiveModuleName] = useState("firstadd.s");
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
  const [teachingMode, setTeachingMode] = useState<TeachingMode>("learn");
  const [numberFormat, setNumberFormat] = useState<NumberFormat>("signed");
  const [memoryFormat, setMemoryFormat] = useState<MemoryFormat>("auto");
  const [traceEntries, setTraceEntries] = useState<TraceEntry[]>([]);
  const [predictionDraft, setPredictionDraft] = useState("");
  const [activePrediction, setActivePrediction] = useState<ActivePrediction | null>(null);
  const [predictionJournal, setPredictionJournal] = useState<PredictionRecord[]>([]);
  const [challengeHintCount, setChallengeHintCount] = useState(0);
  const [challengeFeedback, setChallengeFeedback] = useState<string | null>(null);
  const [sourceDirty, setSourceDirty] = useState(false);
  const [assignmentFeedback, setAssignmentFeedback] = useState<string | null>(null);
  const [workspaceFeedback, setWorkspaceFeedback] = useState<string | null>(null);
  const [consoleEntries, setConsoleEntries] = useState<ConsoleEntry[]>([]);

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
          const nextModules = lessonModules(initial);
          setModules(nextModules);
          setActiveModuleName(initial.module_name);
          loadSession(nextModules);
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

  const loadSession = (nextModules: EditableModule[]) => {
    setSourceDirty(false);
    try {
      session.current?.free();
      session.current = new WasmSession(JSON.stringify(nextModules));
      const nextView = JSON.parse(session.current.view_json()) as MachineView;
      setView(nextView);
      setProgram(JSON.parse(session.current.program_json()) as ProgramView);
      setResult(null);
      setDiagnostics([]);
      setStdinDraft("");
      setTraceEntries([]);
      setPredictionDraft("");
      setActivePrediction(null);
      setChallengeFeedback(null);
      setAssignmentFeedback(null);
      setConsoleEntries([]);
      if (nextView.next_instruction) setActiveModuleName(nextView.next_instruction.module);
    } catch (error: unknown) {
      session.current = null;
      setView(null);
      setProgram(null);
      setResult(null);
      setDiagnostics(parseBuildDiagnostics(error));
      setTraceEntries([]);
      setPredictionDraft("");
      setActivePrediction(null);
      setChallengeFeedback(null);
      setAssignmentFeedback(null);
    }
  };

  const applyResult = (command: TraceEntry["command"], next: CommandResult) => {
    setResult(next);
    setView(next.view);
    if (next.view.next_instruction) setActiveModuleName(next.view.next_instruction.module);
    setDiagnostics(next.diagnostics);
    setTutorialFeedback(null);
    setChallengeFeedback(null);
    setAssignmentFeedback(null);
    setTraceEntries((current) => [
      ...current,
      { sequence: current.length + 1, command, result: next }
    ]);
  };

  // Console lines run through the same core REPL as `x86-63 repl`, so a
  // command script behaves identically in the browser and the terminal.
  const runConsoleLines = (lines: string[]) => {
    if (!session.current) return;
    const answered: { line: string; output: string }[] = [];
    try {
      for (const line of lines) {
        const response = JSON.parse(session.current.repl(line)) as ReplResponse;
        answered.push({ line, output: response.output });
        if (response.result) applyResult({ Console: line }, response.result);
        if (response.quit) break;
      }
    } catch (error: unknown) {
      setBootError(String(error));
    }
    setConsoleEntries((current) => [
      ...current,
      ...answered.map((entry, index) => ({ sequence: current.length + index + 1, ...entry }))
    ]);
  };

  const execute = (command: Command) => {
    if (!session.current) return;
    try {
      const next = JSON.parse(session.current.execute(JSON.stringify(command))) as CommandResult;
      setResult(next);
      setView(next.view);
      if (next.view.next_instruction) setActiveModuleName(next.view.next_instruction.module);
      setDiagnostics(next.diagnostics);
      setTutorialFeedback(null);
      setChallengeFeedback(null);
      setAssignmentFeedback(null);
      setTraceEntries((current) => [
        ...current,
        { sequence: current.length + 1, command, result: next }
      ]);
    } catch (error: unknown) {
      setBootError(String(error));
    }
  };

  const chooseLesson = (id: string) => {
    const lesson = lessons.find((candidate) => candidate.id === id);
    if (!lesson) return;
    const nextModules = lessonModules(lesson);
    setSelectedId(id);
    setModules(nextModules);
    setActiveModuleName(lesson.module_name);
    loadSession(nextModules);
    setTutorialOpen(false);
    setTutorialFeedback(null);
    setPredictionJournal([]);
    setChallengeHintCount(0);
    setChallengeFeedback(null);
    setAssignmentFeedback(null);
    setWorkspaceFeedback(null);
  };

  const startTutorial = (kind: TutorialKind = "registers") => {
    const lessonId =
      kind === "memory"
        ? "addarray3"
        : kind === "functions"
          ? "funstack"
          : kind === "recursion"
            ? "facttrace"
            : "firstadd";
    const lesson = lessons.find((candidate) => candidate.id === lessonId);
    if (!lesson) return;
    const nextModules = lessonModules(lesson);
    setSelectedId(lesson.id);
    setModules(nextModules);
    setActiveModuleName(lesson.module_name);
    loadSession(nextModules);
    setTutorialKind(kind);
    setTutorialStep(0);
    setTutorialFeedback(null);
    setTutorialOpen(true);
    setPredictionJournal([]);
    setChallengeHintCount(0);
    setChallengeFeedback(null);
    setAssignmentFeedback(null);
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
  const activeModule = modules.find((module) => module.name === activeModuleName) ?? modules[0];

  const updateActiveModule = (update: (module: EditableModule) => EditableModule) => {
    if (!activeModule) return;
    const previousName = activeModule.name;
    const next = update(activeModule);
    setModules((current) =>
      current.map((module) => (module.name === previousName ? next : module))
    );
    if (next.name !== previousName) setActiveModuleName(next.name);
    setChallengeFeedback(null);
    setAssignmentFeedback(null);
    setSourceDirty(true);
  };
  const changed = useMemo(
    () =>
      new Set(
        (result?.events ?? [])
          .filter((event) => event.kind === "register_write")
          .map((event) => event.canonical ?? "")
      ),
    [result]
  );
  const relevantRegisters = useMemo(() => {
    const relevant = registersInSource(activeModule?.source ?? "");
    relevant.add("rax");
    relevant.add("rdi");
    relevant.add("rsp");
    return relevant;
  }, [activeModule?.source]);
  const visibleRegisters =
    teachingMode === "inspect"
      ? (view?.registers ?? [])
      : (view?.registers ?? []).filter((register) => relevantRegisters.has(register.name));
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

  const predictionPrompt =
    view && view.history_depth > 0 && view.next_text
      ? `What will \`${view.next_text.trim()}\` change, and why?`
      : selectedLesson?.prediction ?? "What do you predict will change?";
  const predictionHasResult = Boolean(
    activePrediction &&
      view &&
      (view.history_depth !== activePrediction.startDepth || view.status.kind !== "paused")
  );
  const observationHidden = Boolean(
    teachingMode === "learn" &&
      activePrediction &&
      !activePrediction.revealed &&
      predictionHasResult
  );

  const commitPrediction = () => {
    const answer = predictionDraft.trim();
    if (!view || !answer) return;
    setActivePrediction({
      prompt: predictionPrompt,
      answer,
      startDepth: view.history_depth,
      location: view.next_instruction,
      revealed: false,
      evidence: null
    });
    setPredictionDraft("");
  };

  const revealPrediction = () => {
    if (!activePrediction || activePrediction.revealed || !view) return;
    const evidence = result?.explanation ?? "The machine state changed without a new explanation.";
    const revealed = { ...activePrediction, revealed: true, evidence };
    setActivePrediction(revealed);
    setPredictionJournal((current) => [
      ...current,
      { ...revealed, endDepth: view.history_depth, evidence }
    ]);
  };

  const startNewPrediction = () => {
    setActivePrediction(null);
    setPredictionDraft("");
  };

  const checkChallenge = () => {
    if (!selectedLesson?.challenge) return;
    if (sourceDirty) {
      setChallengeFeedback("Assemble your edited source before checking the repair.");
      return;
    }
    const solved = challengePassed(selectedLesson.id, view, traceEntries);
    if (solved) {
      setChallengeFeedback(`Solved. ${selectedLesson.challenge.success}`);
      return;
    }
    const firstError = diagnostics.find((diagnostic) => diagnostic.severity === "error");
    if (firstError) {
      setChallengeFeedback(`Not yet. The assembler still reports ${firstError.code}: ${firstError.message}`);
    } else if (view?.history_depth === 0 && view.status.kind === "paused") {
      setChallengeFeedback("The repair assembles. Run it, then check the observable result.");
    } else {
      setChallengeFeedback(`Not yet. ${selectedLesson.challenge.success}`);
    }
  };

  const runAssignmentSample = () => {
    const assignment = selectedLesson?.assignment;
    if (!assignment || modules.length === 0) return;
    setAssignmentFeedback(null);
    setWorkspaceFeedback(null);
    try {
      session.current?.free();
      session.current = null;
      const nextSession = new WasmSession(JSON.stringify(modules));
      session.current = nextSession;
      const nextProgram = JSON.parse(nextSession.program_json()) as ProgramView;
      const transitions: TraceEntry[] = [];
      const run = (command: Command) => {
        const commandResult = JSON.parse(
          nextSession.execute(JSON.stringify(command))
        ) as CommandResult;
        transitions.push({
          sequence: transitions.length + 1,
          command,
          result: commandResult
        });
        return commandResult;
      };

      let finalResult: CommandResult | null = null;
      let stopReason: string | null = null;
      for (const input of assignment.sample_inputs) {
        const waiting = run({ Continue: { max_steps: 2_000 } });
        finalResult = waiting;
        if (waiting.view.status.kind !== "waiting_for_input") {
          stopReason = `Sample stopped before input ${input}: expected a read, but the machine is ${waiting.view.status.kind}.`;
          break;
        }
        finalResult = run({ SubmitInput: { text: input } });
      }
      if (!stopReason && finalResult?.view.status.kind === "paused") {
        finalResult = run({ Continue: { max_steps: 2_000 } });
      }
      if (!finalResult) return;

      const allDiagnostics = transitions.flatMap((entry) => entry.result.diagnostics);
      setProgram(nextProgram);
      setView(finalResult.view);
      setResult(finalResult);
      setActiveModuleName(selectedLesson.module_name);
      setDiagnostics(allDiagnostics);
      setTraceEntries(transitions);
      setSourceDirty(false);
      setStdinDraft("");
      setPredictionDraft("");
      setActivePrediction(null);
      if (stopReason) {
        setAssignmentFeedback(stopReason);
      } else if (assignmentChecks(finalResult.view, transitions, nextProgram, assignment).every(Boolean)) {
        setAssignmentFeedback(`Sample passed. ${assignment.success}`);
      } else if (finalResult.view.status.kind === "faulted") {
        setAssignmentFeedback(
          `Sample faulted (${finalResult.view.status.code}): ${finalResult.view.status.message}`
        );
      } else {
        setAssignmentFeedback(
          "The sample completed, but one or more rubric checks still need attention. Use the checklist and trace receipts below."
        );
      }
    } catch (error: unknown) {
      session.current?.free();
      session.current = null;
      setView(null);
      setProgram(null);
      setResult(null);
      setTraceEntries([]);
      setDiagnostics(parseBuildDiagnostics(error));
      setSourceDirty(false);
      setAssignmentFeedback("The sample could not start because the current workspace does not assemble.");
    }
  };

  const addModule = () => {
    const name = uniqueModuleName(modules);
    setModules((current) => [...current, { name, source: "# New linked module\n.text\n" }]);
    setActiveModuleName(name);
    setSourceDirty(true);
    setWorkspaceFeedback(`${name} added · assemble when the module is ready.`);
    setAssignmentFeedback(null);
  };

  const removeActiveModule = () => {
    if (!activeModule || modules.length <= 1) return;
    const nextModules = modules.filter((module) => module.name !== activeModule.name);
    setModules(nextModules);
    setActiveModuleName(nextModules[0]?.name ?? "");
    setSourceDirty(true);
    setWorkspaceFeedback(`${activeModule.name} removed · assemble to relink the workspace.`);
    setAssignmentFeedback(null);
  };

  const importModules = async (fileList: FileList | null) => {
    const files = Array.from(fileList ?? []);
    if (files.length === 0) return;
    try {
      let nextModules = [...modules];
      let firstImported: string | null = null;
      for (const file of files) {
        const text = await file.text();
        if (file.name.toLowerCase().endsWith(".json")) {
          const payload = JSON.parse(text) as Partial<WorkspacePayload>;
          if (
            payload.format !== "x86-63-workspace" ||
            payload.version !== 1 ||
            !validModules(payload.modules)
          ) {
            throw new Error(`${file.name} is not an x86-63 workspace export`);
          }
          nextModules = payload.modules;
          firstImported = nextModules[0]?.name ?? null;
          continue;
        }
        if (!file.name.toLowerCase().endsWith(".s")) {
          throw new Error(`${file.name} is not an assembly module or workspace JSON file`);
        }
        const imported = { name: file.name, source: text };
        const existing = nextModules.findIndex((module) => module.name === imported.name);
        if (existing >= 0) nextModules[existing] = imported;
        else nextModules.push(imported);
        firstImported ??= imported.name;
      }
      if (nextModules.length === 0) throw new Error("a workspace must contain at least one module");
      setModules(nextModules);
      setActiveModuleName(firstImported ?? nextModules[0]!.name);
      loadSession(nextModules);
      setWorkspaceFeedback(
        `Imported ${files.length} local file${files.length === 1 ? "" : "s"}; nothing was uploaded.`
      );
      setAssignmentFeedback(null);
    } catch (error: unknown) {
      setWorkspaceFeedback(`Import failed: ${String(error)}`);
    }
  };

  const exportWorkspace = () => {
    downloadJson(
      {
        format: "x86-63-workspace",
        version: 1,
        exported_at: new Date().toISOString(),
        modules
      },
      `x86-63-${selectedLesson?.id ?? "workspace"}-workspace.json`
    );
    setWorkspaceFeedback("Workspace exported with every linked source module.");
  };

  const exportTrace = () => {
    const lessonSummary = selectedLesson
      ? {
          id: selectedLesson.id,
          title: selectedLesson.title,
          lecture: selectedLesson.lecture,
          challenge: selectedLesson.challenge
        }
      : null;
    const payload = {
      format: "x86-63-teaching-trace",
      version: 1,
      exported_at: new Date().toISOString(),
      lesson: lessonSummary,
      teaching_mode: teachingMode,
      modules,
      build_diagnostics: diagnostics,
      predictions: predictionJournal,
      transitions: traceEntries
    };
    downloadJson(payload, `x86-63-${selectedLesson?.id ?? "trace"}.json`);
  };

  if (bootError) {
    return <main className="fatal">Could not start x86-63: {bootError}</main>;
  }

  return (
    <main className={`app-shell ${teachingMode}-mode`}>
      <header className="masthead">
        <div>
          <p className="eyebrow">CS201 teaching machine</p>
          <h1>x86-63</h1>
        </div>
        <div className="mode-and-tagline">
          <p className="tagline">One instruction at a time, with receipts.</p>
          <div className="mode-toggle" role="group" aria-label="Teaching density">
            <button
              className={teachingMode === "learn" ? "selected" : ""}
              aria-pressed={teachingMode === "learn"}
              onClick={() => setTeachingMode("learn")}
            >
              Learn
              <small>focused state</small>
            </button>
            <button
              className={teachingMode === "inspect" ? "selected" : ""}
              aria-pressed={teachingMode === "inspect"}
              onClick={() => setTeachingMode("inspect")}
            >
              Inspect
              <small>full machine</small>
            </button>
          </div>
        </div>
      </header>

      <section className="lesson-bar" aria-label="Lesson and execution controls">
        <label>
          Example
          <select value={selectedId} onChange={(event) => chooseLesson(event.target.value)}>
            {lessonGroups(lessons).map((group) => (
                <optgroup label={group.label} key={group.key}>
                  {group.lessons.map((lesson) => (
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
          <button
            className="tutorial-trigger recursion-tutorial-trigger"
            onClick={() => startTutorial("recursion")}
            disabled={!lessons.length}
          >
            Recursion tutorial
          </button>
          <button onClick={() => loadSession(modules)} disabled={!lessons.length}>
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
          <button
            onClick={() => execute("ClearBreakpoints")}
            disabled={!view?.breakpoints.length}
          >
            Clear breaks{view?.breakpoints.length ? ` (${view.breakpoints.length})` : ""}
          </button>
          <button onClick={exportTrace} disabled={!modules.length}>
            Export trace
          </button>
        </div>
      </section>

      {selectedLesson?.challenge && (
        <ChallengePanel
          lesson={selectedLesson}
          hintCount={challengeHintCount}
          feedback={challengeFeedback}
          onCheck={checkChallenge}
          onHint={() =>
            setChallengeHintCount((current) =>
              Math.min(selectedLesson.challenge?.hints.length ?? 0, current + 1)
            )
          }
          onRestart={() => chooseLesson(selectedLesson.id)}
        />
      )}

      {selectedLesson?.assignment && (
        <AssignmentPanel
          lesson={selectedLesson}
          checks={assignmentChecks(
            view,
            traceEntries,
            program,
            selectedLesson.assignment
          )}
          feedback={assignmentFeedback}
          onRunSample={runAssignmentSample}
        />
      )}

      {selectedLesson && view && teachingMode === "learn" && !tutorialOpen && (
        <PredictionPanel
          prompt={predictionPrompt}
          draft={predictionDraft}
          active={activePrediction}
          hasResult={predictionHasResult}
          onDraft={setPredictionDraft}
          onCommit={commitPrediction}
          onReveal={revealPrediction}
          onNew={startNewPrediction}
        />
      )}

      {selectedLesson && teachingMode === "inspect" && (
        <section className="prediction inspect-prompt">
          <strong>{lessonCollectionLabel(selectedLesson)} · Before you step:</strong>{" "}
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
                : tutorialKind === "recursion"
                  ? "Recursive frame chains"
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
            <span className={`source-state ${sourceDirty ? "dirty" : session.current ? "" : "blocked"}`}>
              {sourceDirty
                ? "edited · assemble to run"
                : session.current
                  ? "assembled"
                  : "build blocked"}
            </span>
            {modules.length > 1 && (
              <select
                className="module-select"
                aria-label="Source module"
                value={activeModule?.name ?? ""}
                onChange={(event) => setActiveModuleName(event.target.value)}
              >
                {modules.map((module) => (
                  <option value={module.name} key={module.name}>{module.name}</option>
                ))}
              </select>
            )}
            <input
              aria-label="Module name"
              value={activeModule?.name ?? ""}
              onChange={(event) =>
                updateActiveModule((module) => ({ ...module, name: event.target.value }))
              }
            />
          </div>
          <div className="workspace-actions">
            <button onClick={addModule}>New module</button>
            <button onClick={removeActiveModule} disabled={modules.length <= 1}>Remove module</button>
            <button onClick={() => fileInput.current?.click()}>Import .s / workspace</button>
            <button onClick={exportWorkspace} disabled={!modules.length}>Export workspace</button>
            <input
              ref={fileInput}
              className="visually-hidden"
              type="file"
              accept=".s,.json,text/plain,application/json"
              multiple
              onChange={(event) => {
                void importModules(event.target.files);
                event.target.value = "";
              }}
            />
            {workspaceFeedback && <small aria-live="polite">{workspaceFeedback}</small>}
          </div>
          <textarea
            className="source-editor"
            aria-label="Assembly source"
            spellCheck={false}
            value={activeModule?.source ?? ""}
            onChange={(event) =>
              updateActiveModule((module) => ({ ...module, source: event.target.value }))
            }
          />
          <SourceListing
            program={program}
            view={view}
            moduleName={activeModule?.name ?? null}
            onToggle={(module, line) =>
              execute({ ToggleBreakpoint: { module, line } })
            }
          />
        </div>

        <div className="machine-column">
          {observationHidden ? (
            <section className="panel prediction-curtain" aria-live="polite">
              <p className="eyebrow">Prediction recorded</p>
              <h2>The resulting machine state is being held back.</h2>
              <p>Commit to what you expected, then reveal the register, flag, memory, and stack evidence together.</p>
              <button className="primary" onClick={revealPrediction}>Reveal the result</button>
            </section>
          ) : (
            <>
              <section className="panel registers-panel">
                <div className="panel-heading">
                  <div>
                    <h2>Registers</h2>
                    {teachingMode === "learn" && (
                      <small className="focus-note">
                        showing {visibleRegisters.length} used or course-role registers
                      </small>
                    )}
                  </div>
                  <label className="panel-control">
                    Interpret as
                    <select
                      aria-label="Register interpretation"
                      value={numberFormat}
                      onChange={(event) => setNumberFormat(event.target.value as NumberFormat)}
                    >
                      <option value="signed">signed</option>
                      <option value="unsigned">unsigned</option>
                      <option value="binary">binary</option>
                      <option value="ascii">ASCII bytes</option>
                    </select>
                  </label>
                  <StatusBadge status={view?.status} />
                </div>
                <div className="register-grid">
                  {visibleRegisters.map((register) => (
                    <div className={`register ${changed.has(register.name) ? "changed" : ""}`} key={register.name}>
                      <strong>%{register.name}</strong>
                      <span className="register-role">{registerRole(register.name)}</span>
                      <code>{register.hex}</code>
                      <small>{formatRegister(register, numberFormat)}</small>
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
                <MemoryPanel
                  memory={view.memory}
                  changed={changedMemory}
                  format={memoryFormat}
                  numberFormat={numberFormat}
                  onFormat={setMemoryFormat}
                />
              )}

              {view && <StackPanel stack={view.stack} changed={changedStack} />}

              <section className="panel explanation-panel" aria-live="polite">
                <p className="eyebrow">This step</p>
                <h2>{result?.explanation ?? "Ready at _start."}</h2>
                <EventSummary
                  events={result?.events ?? []}
                  compact={teachingMode === "learn"}
                />
              </section>
            </>
          )}

          {view && (
            <ProcessIoPanel
              io={view.io}
              value={stdinDraft}
              onChange={setStdinDraft}
              onSubmit={submitInput}
              halted={view.status.kind === "exited" || view.status.kind === "faulted"}
              hideOutput={observationHidden}
            />
          )}

          {view && (
            <DebuggerConsole
              entries={consoleEntries}
              onRun={runConsoleLines}
              onClear={() => setConsoleEntries([])}
            />
          )}

          {diagnostics.length > 0 && <Diagnostics diagnostics={diagnostics} />}
        </div>
      </section>
    </main>
  );
}

function ChallengePanel({
  lesson,
  hintCount,
  feedback,
  onCheck,
  onHint,
  onRestart
}: {
  lesson: Lesson;
  hintCount: number;
  feedback: string | null;
  onCheck: () => void;
  onHint: () => void;
  onRestart: () => void;
}) {
  const challenge = lesson.challenge;
  if (!challenge) return null;
  return (
    <section className="challenge panel" aria-label="Repair challenge">
      <div className="challenge-copy">
        <p className="eyebrow">Repair lab · evidence before hints</p>
        <h2>{challenge.goal}</h2>
        <p><strong>Done when:</strong> {challenge.success}</p>
        {hintCount > 0 && (
          <ol className="challenge-hints">
            {challenge.hints.slice(0, hintCount).map((hint, index) => (
              <li key={hint}>{index === challenge.hints.length - 1 ? `Direct repair: ${hint}` : hint}</li>
            ))}
          </ol>
        )}
        {feedback && (
          <p
            className={`challenge-feedback ${feedback.startsWith("Solved") ? "solved" : ""}`}
            aria-live="polite"
          >
            {feedback}
          </p>
        )}
      </div>
      <div className="challenge-controls">
        <button className="primary" onClick={onCheck}>Check my repair</button>
        <button onClick={onHint} disabled={hintCount >= challenge.hints.length}>
          {hintCount === 0 ? "Show a hint" : "Show another hint"}
        </button>
        <button onClick={onRestart}>Restore starter</button>
      </div>
    </section>
  );
}

function AssignmentPanel({
  lesson,
  checks,
  feedback,
  onRunSample
}: {
  lesson: Lesson;
  checks: boolean[];
  feedback: string | null;
  onRunSample: () => void;
}) {
  const assignment = lesson.assignment;
  if (!assignment) return null;
  const complete = checks.length > 0 && checks.every(Boolean);
  return (
    <section className="assignment panel" aria-label="Assignment 2 test bench">
      <div className="assignment-copy">
        <p className="eyebrow">Assignment 2 · repeatable test bench</p>
        <h2>{assignment.goal}</h2>
        <ol className="assignment-checklist">
          {assignment.requirements.map((requirement, index) => (
            <li className={checks[index] ? "passed" : ""} key={requirement}>
              <span aria-hidden="true">{checks[index] ? "✓" : "○"}</span>
              {requirement}
            </li>
          ))}
        </ol>
        <p className="assignment-sample">
          <strong>Sample terminal lines:</strong> {assignment.sample_inputs.join(" · ")}
        </p>
        {feedback && (
          <p
            className={`assignment-feedback ${complete ? "passed" : ""}`}
            aria-live="polite"
          >
            {feedback}
          </p>
        )}
      </div>
      <div className="assignment-controls">
        <button className="primary" onClick={onRunSample}>
          Run five-line sample
        </button>
        <small>Rebuilds the current modules, then submits one line at each blocked read.</small>
      </div>
    </section>
  );
}

function PredictionPanel({
  prompt,
  draft,
  active,
  hasResult,
  onDraft,
  onCommit,
  onReveal,
  onNew
}: {
  prompt: string;
  draft: string;
  active: ActivePrediction | null;
  hasResult: boolean;
  onDraft: (value: string) => void;
  onCommit: () => void;
  onReveal: () => void;
  onNew: () => void;
}) {
  if (!active) {
    return (
      <form
        className="prediction prediction-journal"
        onSubmit={(event) => {
          event.preventDefault();
          onCommit();
        }}
      >
        <div>
          <strong>Predict before stepping</strong>
          <p>{prompt}</p>
        </div>
        <textarea
          aria-label="Your prediction"
          rows={2}
          value={draft}
          onChange={(event) => onDraft(event.target.value)}
          placeholder="Write what will change and why…"
        />
        <button type="submit" disabled={!draft.trim()}>Record prediction</button>
      </form>
    );
  }

  if (!active.revealed) {
    return (
      <section className="prediction prediction-journal prediction-locked">
        <div>
          <strong>Prediction sealed</strong>
          <p>{active.prompt}</p>
          <small>Your {active.answer.split(/\s+/).length}-word answer is hidden until you reveal the evidence.</small>
        </div>
        <button className="primary" onClick={onReveal} disabled={!hasResult}>
          {hasResult ? "Reveal result" : "Step, Next, or Run first"}
        </button>
      </section>
    );
  }

  return (
    <section className="prediction prediction-journal prediction-revealed">
      <div>
        <strong>Compare prediction with evidence</strong>
        <p><b>Your prediction:</b> {active.answer}</p>
        <p><b>Machine receipt:</b> {active.evidence ?? "No receipt was produced."}</p>
      </div>
      <button onClick={onNew}>Predict the next instruction</button>
    </section>
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

function SourceListing({
  program,
  view,
  moduleName,
  onToggle
}: {
  program: ProgramView | null;
  view: MachineView | null;
  moduleName: string | null;
  onToggle: (module: string, line: number) => void;
}) {
  const module = program?.modules.find((candidate) => candidate.name === moduleName) ?? program?.modules[0];
  if (!module) return null;
  return (
    <ol className="source-listing" aria-label="Assembled source with current instruction">
      {module.source.split("\n").map((line, index) => {
        const lineNumber = index + 1;
        const current = view?.next_instruction?.module === module.name && view.next_instruction.line === lineNumber;
        const executable = program?.instructions.some(
          (instruction) =>
            instruction.location.module === module.name && instruction.location.line === lineNumber
        );
        const breakpoint = view?.breakpoints.some(
          (location) => location.module === module.name && location.line === lineNumber
        );
        return (
          <li
            className={`${current ? "current" : ""} ${breakpoint ? "has-breakpoint" : ""}`}
            key={`${lineNumber}-${line}`}
          >
            <button
              className="breakpoint-toggle"
              aria-label={`${breakpoint ? "Remove" : "Set"} breakpoint at ${module.name} line ${lineNumber}`}
              aria-pressed={Boolean(breakpoint)}
              disabled={!executable}
              title={executable ? "Toggle breakpoint" : "No instruction on this line"}
              onClick={() => onToggle(module.name, lineNumber)}
            >
              {breakpoint ? "●" : "○"}
            </button>
            <code>{line || " "}</code>
          </li>
        );
      })}
    </ol>
  );
}

function EventSummary({ events, compact = false }: { events: StepEvent[]; compact?: boolean }) {
  const useful = events.filter((event) =>
    [
      "register_write",
      "effective_address",
      "memory_read",
      "memory_write",
      "arithmetic",
      "division",
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
      "fault",
      "breakpoint_changed",
      "breakpoint_hit"
    ].includes(event.kind)
  );
  if (useful.length === 0) return null;
  const shown = compact && useful.length > 16
    ? [...useful.slice(0, 4), ...useful.slice(-12)]
    : useful;
  const hiddenCount = useful.length - shown.length;
  return (
    <ul className="event-list">
      {hiddenCount > 0 && (
        <li className="event-omission">
          {hiddenCount} middle receipts hidden in Learn mode; Inspect shows the complete transition.
        </li>
      )}
      {shown.map((event, index) => (
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
    case "division":
      return `${event.signed ? "idiv" : "div"}: ${event.dividend_high}:${event.dividend_low} ÷ ${event.divisor} → quotient ${event.quotient}, remainder ${event.remainder}`;
    case "effective_address":
      return `${event.expression} resolves to ${event.address}${event.symbol ? ` (${event.symbol})` : ""}`;
    case "memory_read":
      return `read ${event.width} bits at ${event.address}: ${event.value}`;
    case "memory_write":
      return `wrote ${event.width} bits at ${event.address}: ${event.before} → ${event.after}`;
    case "breakpoint_changed":
      return `breakpoint ${event.enabled ? "set" : "removed"} at ${event.location?.module}:${event.location?.line}`;
    case "breakpoint_hit":
      return `stopped before breakpoint at ${event.location?.module}:${event.location?.line}`;
    case "compare":
      return `cmp: ${event.destination} − ${event.source} = ${event.result}`;
    case "branch":
      return `${event.condition} → ${event.target}: ${event.predicate}; ${event.taken ? "taken" : "not taken"}`;
    case "call":
      return `call ${event.target}: %rsp ${event.stack_pointer_before} was ${event.aligned_before ? "16-byte aligned" : "misaligned"}; pushed return address ${event.return_address}`;
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
  changed,
  format,
  numberFormat,
  onFormat
}: {
  memory: MachineView["memory"];
  changed: Set<number>;
  format: MemoryFormat;
  numberFormat: NumberFormat;
  onFormat: (format: MemoryFormat) => void;
}) {
  return (
    <section className="panel memory-panel">
      <div className="panel-heading">
        <h2>Memory</h2>
        <label className="panel-control">
          Group as
          <select
            aria-label="Memory grouping"
            value={format}
            onChange={(event) => onFormat(event.target.value as MemoryFormat)}
          >
            <option value="auto">source directives</option>
            <option value="byte">bytes</option>
            <option value="word">words · 2B</option>
            <option value="long">longs · 4B</option>
            <option value="quad">quads · 8B</option>
            <option value="text">text rows</option>
          </select>
        </label>
        <span className="memory-size">mapped data · {memory.bytes.length} bytes</span>
      </div>
      <div className="memory-symbols">
        {memory.symbols.map((symbol) => {
          const textMode = format === "text";
          const compactByteBuffer =
            format === "auto" && symbol.element_width === 1 && symbol.size > 32;
          const requestedWidth = ({ byte: 1, word: 2, long: 4, quad: 8 } as const)[
            format as "byte" | "word" | "long" | "quad"
          ];
          const displayWidth = textMode
            ? 16
            : compactByteBuffer
              ? 8
              : format === "auto"
                ? symbol.element_width
                : requestedWidth;
          const viewSize =
            format === "auto" || textMode
              ? symbol.size
              : Math.min(
                  memory.bytes.length - symbol.offset,
                  Math.max(symbol.size, displayWidth)
                );
          const crossesSymbolBoundary = viewSize > symbol.size;
          const bytes = memory.bytes.slice(symbol.offset, symbol.offset + viewSize);
          const elements = chunk(bytes, displayWidth);
          const elementCount = Math.ceil(symbol.size / symbol.element_width);
          return (
            <section className="memory-symbol" key={symbol.name}>
              <header>
                <strong>{symbol.name}</strong>
                <code>{symbol.address}</code>
                <small>
                  {symbol.section} · {directiveName(symbol.element_width)} × {elementCount}
                  {crossesSymbolBoundary ? " · view includes neighboring bytes" : ""}
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
                        {format === "auto" && !compactByteBuffer ? `[${index}]` : `[+${byteOffset}]`} {addressAt(memory.base, offset)}
                      </small>
                      <code>{element.map(hexByte).join(" ")}</code>
                      <strong>
                        {textMode || compactByteBuffer
                          ? `text “${renderBytes(element)}”`
                          : formatMemoryValue(element, numberFormat)}
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
          %rsp mod 16 = {stack.rsp_mod_16} · {stack.aligned_for_call ? "ready for call" : "not call-aligned"}
        </span>
      </div>
      {stack.frames.length > 0 && (
        <div className="frame-chain" aria-label="Active stack frames">
          <h3>Frame chain · newest first</h3>
          {stack.frames.map((frame) => (
            <article className="stack-frame" key={`${frame.depth}-${frame.rbp}`}>
              <strong>#{frame.depth} · {frame.function ?? "unknown function"}</strong>
              <code>%rbp {frame.rbp}</code>
              <small>
                returns to {frame.return_location
                  ? `${frame.return_location.module}:${frame.return_location.line}`
                  : frame.return_address}
                {frame.aligned_at_call ? " · aligned call" : " · misaligned call"}
              </small>
            </article>
          ))}
        </div>
      )}
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

function scriptLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("#"));
}

function DebuggerConsole({
  entries,
  onRun,
  onClear
}: {
  entries: ConsoleEntry[];
  onRun: (lines: string[]) => void;
  onClear: () => void;
}) {
  const [draft, setDraft] = useState("");
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const scriptInput = useRef<HTMLInputElement | null>(null);
  const log = useRef<HTMLDivElement | null>(null);
  const history = entries.map((entry) => entry.line);

  useEffect(() => {
    log.current?.scrollTo({ top: log.current.scrollHeight });
  }, [entries.length]);

  const recall = (direction: -1 | 1) => {
    if (history.length === 0) return;
    const start = historyIndex ?? history.length;
    const next = Math.min(Math.max(start + direction, 0), history.length);
    setHistoryIndex(next === history.length ? null : next);
    setDraft(next === history.length ? "" : (history[next] ?? ""));
  };

  return (
    <section className="panel console-panel" aria-label="Debugger console">
      <div className="panel-heading">
        <h2>Debugger console</h2>
        <span className="memory-size">same commands as x86-63 repl · type help</span>
      </div>
      {entries.length > 0 && (
        <div className="console-log" ref={log} role="log" aria-live="polite">
          {entries.map((entry) => (
            <div className="console-entry" key={entry.sequence}>
              <code className="console-command">(x86-63) {entry.line}</code>
              {entry.output && <pre>{entry.output}</pre>}
            </div>
          ))}
        </div>
      )}
      <form
        className="console-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!draft.trim()) return;
          onRun([draft]);
          setDraft("");
          setHistoryIndex(null);
        }}
      >
        <label htmlFor="console-line">(x86-63)</label>
        <input
          id="console-line"
          value={draft}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "ArrowUp") {
              event.preventDefault();
              recall(-1);
            } else if (event.key === "ArrowDown") {
              event.preventDefault();
              recall(1);
            }
          }}
          placeholder="p/x $rax · x/8xb &buff · break 12 · bt"
        />
        <button type="submit">Run</button>
      </form>
      <div className="console-actions">
        <button type="button" onClick={() => scriptInput.current?.click()}>
          Run script…
        </button>
        <button type="button" onClick={onClear} disabled={entries.length === 0}>
          Clear log
        </button>
        <input
          ref={scriptInput}
          type="file"
          accept=".gdb,.txt,text/plain"
          hidden
          onChange={async (event) => {
            const file = event.target.files?.[0];
            event.target.value = "";
            if (file) onRun(scriptLines(await file.text()));
          }}
        />
      </div>
    </section>
  );
}

function ProcessIoPanel({
  io,
  value,
  onChange,
  onSubmit,
  halted,
  hideOutput
}: {
  io: MachineView["io"];
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  halted: boolean;
  hideOutput: boolean;
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
      {hideOutput && (io.stdout_bytes.length > 0 || io.stderr_bytes.length > 0) && (
        <p className="output-held">Process output is held until you reveal the prediction evidence.</p>
      )}
      {!hideOutput && io.stdout_bytes.length > 0 && (
        <div className="output-stream">
          <strong>stdout</strong>
          <pre>{renderBytes(io.stdout_bytes)}</pre>
          <code>{io.stdout_escaped}</code>
        </div>
      )}
      {!hideOutput && io.stderr_bytes.length > 0 && (
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
  const value = littleEndianUnsigned(bytes);
  const bits = BigInt(bytes.length * 8);
  const sign = 1n << (bits - 1n);
  return (value & sign ? value - (1n << bits) : value).toString();
}

function littleEndianUnsigned(bytes: number[]): bigint {
  let value = 0n;
  bytes.forEach((byte, index) => {
    value |= BigInt(byte) << BigInt(index * 8);
  });
  return value;
}

function formatMemoryValue(bytes: number[], format: NumberFormat): string {
  const value = littleEndianUnsigned(bytes);
  switch (format) {
    case "unsigned":
      return `unsigned ${value}`;
    case "binary":
      return `binary 0b${groupBits(value.toString(2).padStart(bytes.length * 8, "0"))}`;
    case "ascii":
      return `ASCII “${renderAsciiCells(bytes)}”`;
    case "signed":
      return `signed ${littleEndianSigned(bytes)}`;
  }
}

function formatRegister(
  register: MachineView["registers"][number],
  format: NumberFormat
): string {
  const value = BigInt(register.unsigned);
  switch (format) {
    case "unsigned":
      return `unsigned ${register.unsigned}`;
    case "binary":
      return `binary 0b${groupBits(value.toString(2).padStart(64, "0"))}`;
    case "ascii": {
      const bytes = Array.from({ length: 8 }, (_, index) =>
        Number((value >> BigInt(index * 8)) & 0xffn)
      );
      return `ASCII low→high “${renderAsciiCells(bytes)}”`;
    }
    case "signed":
      return `signed ${register.signed}`;
  }
}

function groupBits(bits: string): string {
  return bits.replace(/(.{8})(?=.)/g, "$1_");
}

function renderAsciiCells(bytes: number[]): string {
  return bytes
    .map((byte) => {
      if (byte === 0) return "␀";
      if (byte === 10) return "↵";
      if (byte === 13) return "␍";
      if (byte === 9) return "⇥";
      return byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : "·";
    })
    .join("");
}

function registerRole(name: string): string {
  return (
    {
      rax: "return value · syscall number",
      rdi: "argument 1 · fd or exit status",
      rsi: "argument 2 · buffer address",
      rdx: "argument 3 · byte count or remainder",
      rcx: "argument 4 · caller-saved",
      r8: "argument 5 · caller-saved",
      r9: "argument 6 · caller-saved",
      r10: "caller-saved",
      r11: "caller-saved",
      rbx: "callee-saved",
      rbp: "frame pointer · callee-saved",
      rsp: "stack pointer",
      r12: "callee-saved",
      r13: "callee-saved",
      r14: "callee-saved",
      r15: "callee-saved"
    } as Record<string, string>
  )[name] ?? "general-purpose register";
}

function registersInSource(source: string): Set<string> {
  const registers = new Set<string>();
  for (const match of source.matchAll(/%([a-z][a-z0-9]*)/gi)) {
    const canonical = canonicalRegister(match[1] ?? "");
    if (canonical) registers.add(canonical);
  }
  return registers;
}

function canonicalRegister(alias: string): string | null {
  const name = alias.toLowerCase();
  const numbered = /^r(8|9|1[0-5])(?:d|w|b)?$/.exec(name);
  if (numbered) return `r${numbered[1]}`;
  const aliases: Record<string, string> = {
    rax: "rax", eax: "rax", ax: "rax", al: "rax",
    rbx: "rbx", ebx: "rbx", bx: "rbx", bl: "rbx",
    rcx: "rcx", ecx: "rcx", cx: "rcx", cl: "rcx",
    rdx: "rdx", edx: "rdx", dx: "rdx", dl: "rdx",
    rsi: "rsi", esi: "rsi", si: "rsi", sil: "rsi",
    rdi: "rdi", edi: "rdi", di: "rdi", dil: "rdi",
    rbp: "rbp", ebp: "rbp", bp: "rbp", bpl: "rbp",
    rsp: "rsp", esp: "rsp", sp: "rsp", spl: "rsp"
  };
  return aliases[name] ?? null;
}

function lessonGroups(lessons: Lesson[]) {
  const lectureNumbers = Array.from(
    new Set(
      lessons
        .filter((lesson) => lesson.collection === "lecture")
        .map((lesson) => lesson.lecture)
    )
  ).sort((left, right) => left - right);
  const groups = lectureNumbers.map((lecture) => ({
    key: `lecture-${lecture}`,
    label: `Lecture ${lecture}`,
    lessons: lessons.filter(
      (lesson) => lesson.collection === "lecture" && lesson.lecture === lecture
    )
  }));
  const assignments = lessons.filter((lesson) => lesson.collection === "assignment");
  if (assignments.length > 0) {
    groups.push({ key: "assignments", label: "Assignment 2", lessons: assignments });
  }
  const guide = lessons.filter((lesson) => lesson.collection === "guide");
  if (guide.length > 0) {
    groups.push({ key: "guide", label: "Assembly guide", lessons: guide });
  }
  const practice = lessons.filter((lesson) => lesson.collection === "practice");
  if (practice.length > 0) {
    groups.push({ key: "practice", label: "Practice labs", lessons: practice });
  }
  return groups;
}

function lessonCollectionLabel(lesson: Lesson): string {
  if (lesson.collection === "assignment") return "Assignment 2";
  if (lesson.collection === "practice") return "Practice lab";
  if (lesson.collection === "guide") return "Assembly guide";
  return `Lecture ${lesson.lecture}`;
}

function assignmentChecks(
  view: MachineView | null,
  traceEntries: TraceEntry[],
  program: ProgramView | null,
  assignment: Assignment
): boolean[] {
  const events = traceEntries.flatMap((entry) => entry.result.events);
  const submitted = events.filter((event) => event.kind === "input_submitted").length;
  const reads = events.filter((event) => event.kind === "input_read").length;
  const inputPassed =
    submitted === assignment.sample_inputs.length && reads === assignment.sample_inputs.length;

  const nums = view?.memory.symbols.find((symbol) => symbol.name === "nums");
  const arrayPassed = Boolean(
    view &&
      nums &&
      nums.element_width === 8 &&
      nums.size >= assignment.sample_inputs.length * 8 &&
      assignment.sample_inputs.every(
        (expected, index) => memoryUnsigned(view, "nums", index) === expected
      )
  );

  const calls = events.filter((event) => event.kind === "call");
  const calledTargets = new Set(calls.map((event) => event.target));
  const proceduresPassed = Boolean(
    program?.labels.read_five !== undefined &&
      program.labels.sum_array !== undefined &&
      calledTargets.has("read_five") &&
      calledTargets.has("sum_array") &&
      calledTargets.has("readInt") &&
      calledTargets.has("writeInt") &&
      calls.length > 0 &&
      calls.every((event) => event.aligned_before === true)
  );

  const outputPassed = Boolean(
    view?.status.kind === "exited" &&
      view.status.shell_status === 0 &&
      view.io.stdout_escaped === assignment.expected_output
  );
  return [inputPassed, arrayPassed, proceduresPassed, outputPassed];
}

function uniqueModuleName(modules: EditableModule[]): string {
  const names = new Set(modules.map((module) => module.name));
  for (let index = 1; ; index += 1) {
    const candidate = `module${index}.s`;
    if (!names.has(candidate)) return candidate;
  }
}

function validModules(value: unknown): value is EditableModule[] {
  if (!Array.isArray(value) || value.length === 0) return false;
  const modulesAreValid = value.every(
    (module) =>
      typeof module === "object" &&
      module !== null &&
      typeof (module as EditableModule).name === "string" &&
      (module as EditableModule).name.trim().length > 0 &&
      typeof (module as EditableModule).source === "string"
  );
  if (!modulesAreValid) return false;
  return new Set(value.map((module) => (module as EditableModule).name)).size === value.length;
}

function downloadJson(payload: unknown, filename: string) {
  const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], {
    type: "application/json"
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function challengePassed(
  id: string,
  view: MachineView | null,
  traceEntries: TraceEntry[]
): boolean {
  if (!view || view.status.kind !== "exited") return false;
  switch (id) {
    case "missingimmediate":
    case "reversedoperands":
      return view.status.shell_status === 0 && registerUnsigned(view, "rax") === "60";
    case "wrongwidth":
      return registerUnsigned(view, "rdi") === "255";
    case "callersavedloop":
      return view.io.stdout_escaped === "321" && view.status.shell_status === 0;
    case "misalignedcall": {
      const calls = traceEntries.flatMap((entry) =>
        entry.result.events.filter((event) => event.kind === "call")
      );
      return (
        view.io.stdout_escaped === "42" &&
        view.status.shell_status === 0 &&
        calls.length > 0 &&
        calls.every((event) => event.aligned_before === true)
      );
    }
    default:
      return false;
  }
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
  const hasErrors = diagnostics.some((diagnostic) => diagnostic.severity === "error");
  return (
    <section className={`panel diagnostics ${hasErrors ? "has-errors" : "warnings-only"}`} aria-live="assertive">
      <p className="eyebrow">{hasErrors ? "Assembler diagnostics" : "Teaching diagnostic"}</p>
      {diagnostics.map((diagnostic) => (
        <article
          className={`diagnostic-${diagnostic.severity}`}
          key={`${diagnostic.code}-${diagnostic.location?.module ?? "machine"}-${diagnostic.location?.line ?? 0}`}
        >
          <strong>
            {diagnostic.code}
            {diagnostic.location
              ? ` at ${diagnostic.location.module}:${diagnostic.location.line}`
              : ""}
          </strong>
          <p>{diagnostic.message}</p>
          {diagnostic.help && <small>Teaching hint: {diagnostic.help}</small>}
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
  if (kind === "recursion") return recursionTutorialSteps;
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

const recursionTutorialSteps: TutorialStep[] = [
  {
    title: "Start with a fixed argument",
    instruction:
      "This tracing harness keeps the input at 5 so the frame chain is the only moving part. Find `_start`; do not step yet.",
    expected: "Line 25 should be next, %rdi should be 0, and no frames should be active.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 0 &&
      view.next_instruction?.line === 25 &&
      view.stack.frames.length === 0
  },
  {
    title: "Place n in the argument register",
    instruction: "Click Step once. The first call to fact will receive n=5 in %rdi.",
    expected: "%rdi should be 5 and the call on line 26 should be next.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 1 &&
      registerUnsigned(view, "rdi") === "5" &&
      view.next_instruction?.line === 26
  },
  {
    title: "Enter fact",
    instruction:
      "Click Step on `call fact`. The return address appears immediately; the frame itself gets an %rbp in the next two instructions.",
    expected: "Line 7 should be next with one return-address slot and %rsp mod 16 equal to 8.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 2 &&
      view.next_instruction?.line === 7 &&
      view.stack.slots.length === 1 &&
      view.stack.rsp_mod_16 === 8
  },
  {
    title: "Build the first frame",
    instruction:
      "Click Step three times for push, the %rbp anchor, and 16 bytes of locals. Watch the alignment return to call-ready.",
    expected: "One fact frame should be active, line 10 should be next, and %rsp should be 16-byte aligned.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 5 &&
      view.next_instruction?.line === 10 &&
      view.stack.frames.length === 1 &&
      view.stack.frames[0]?.function === "fact" &&
      view.stack.aligned_for_call
  },
  {
    title: "Descend one recursive level",
    instruction:
      "Click Step eight times: cmp, jle, save n, dec, call, push, anchor %rbp, reserve locals. The new frame becomes #0.",
    expected: "Two fact frames should be linked, the new call should have n=4, and line 10 should be next again.",
    check: (view) =>
      view.status.kind === "paused" &&
      view.history_depth === 13 &&
      view.next_instruction?.line === 10 &&
      registerUnsigned(view, "rdi") === "4" &&
      view.stack.frames.length === 2 &&
      view.stack.frames.every((frame) => frame.function === "fact")
  },
  {
    title: "Let the recursion unwind",
    instruction:
      "Click Run. The machine reaches n=1, returns 1, multiplies by each saved local on the way out, and tears down every frame.",
    expected: "The program should exit with 120, %rdi should contain 120, and the frame chain should be empty.",
    check: (view) =>
      view.status.kind === "exited" &&
      view.status.shell_status === 120 &&
      registerUnsigned(view, "rdi") === "120" &&
      view.stack.frames.length === 0 &&
      view.stack.slots.length === 0
  }
];
