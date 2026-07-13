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
  memory: {
    base: string;
    bytes: number[];
    symbols: SymbolView[];
  };
  io: {
    stdout_bytes: number[];
    stdout_escaped: string;
    stderr_bytes: number[];
    stderr_escaped: string;
  };
  history_depth: number;
};

export type SymbolView = {
  name: string;
  address: string;
  offset: number;
  size: number;
  element_width: number;
  section: string;
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
  address?: string;
  value?: string;
  symbol?: string | null;
  expression?: string;
  destination?: string;
  source?: string;
  predicate?: string;
  condition?: string;
  target?: string;
  taken?: boolean;
  fd?: number;
  bytes?: number[];
  escaped?: string;
  width?: number;
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
  symbols: SymbolView[];
  constants: Record<string, string>;
  data_base: string;
  data_size: number;
  entry: number;
};
