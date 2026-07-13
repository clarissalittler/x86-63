export type Lesson = {
  id: string;
  title: string;
  lecture: number;
  summary: string;
  prediction: string;
  module_name: string;
  source: string;
};

export type SourceLocation = {
  module: string;
  line: number;
  column: number;
};

export type Diagnostic = {
  severity: "error" | "warning";
  code: string;
  message: string;
  help: string | null;
  location: SourceLocation | null;
};

export type MachineStatus =
  | { kind: "paused" }
  | { kind: "exited"; raw_hex: string; signed: string; shell_status: number }
  | { kind: "faulted"; code: string; message: string };

export type RegisterView = {
  name: string;
  hex: string;
  signed: string;
  unsigned: string;
};

export type FlagsView = {
  cf: boolean;
  pf: boolean;
  af: boolean;
  zf: boolean;
  sf: boolean;
  of: boolean;
};

export type MachineView = {
  protocol_version: number;
  status: MachineStatus;
  next_instruction: SourceLocation | null;
  next_text: string | null;
  registers: RegisterView[];
  flags: FlagsView;
  history_depth: number;
};

export type StepEvent = {
  kind: string;
  register?: string;
  canonical?: string;
  before?: string;
  after?: string;
  operation?: string;
  left?: string;
  right?: string;
  result?: string;
  code?: string;
  message?: string;
  shell_status?: number;
};

export type CommandResult = {
  protocol_version: number;
  steps_executed: number;
  view: MachineView;
  events: StepEvent[];
  diagnostics: Diagnostic[];
  explanation: string;
};

export type ProgramView = {
  modules: { name: string; source: string }[];
  instructions: { index: number; location: SourceLocation; text: string }[];
  labels: Record<string, number>;
  entry: number;
};
