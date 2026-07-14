use serde::{Deserialize, Serialize};

use crate::{Diagnostic, Flags, MachineStatus, SourceLocation, SymbolView};

// Version 4 adds linked lesson modules, recursive frame projections, alignment
// receipts, and the multiply/divide operations used by Lecture 6 helpers.
pub const PROTOCOL_VERSION: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Reset,
    Step,
    Next,
    Back,
    Continue { max_steps: usize },
    SubmitInput { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub protocol_version: u32,
    pub steps_executed: usize,
    pub view: MachineView,
    pub events: Vec<StepEvent>,
    pub diagnostics: Vec<Diagnostic>,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineView {
    pub protocol_version: u32,
    pub status: MachineStatus,
    pub next_instruction: Option<SourceLocation>,
    pub next_text: Option<String>,
    pub registers: Vec<RegisterView>,
    pub flags: FlagsView,
    pub memory: MemoryView,
    pub stack: StackView,
    pub io: IoView,
    pub history_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryView {
    pub base: String,
    pub bytes: Vec<u8>,
    pub symbols: Vec<SymbolView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoView {
    pub stdin_bytes: Vec<u8>,
    pub stdin_escaped: String,
    pub stdin_consumed: usize,
    pub stdout_bytes: Vec<u8>,
    pub stdout_escaped: String,
    pub stderr_bytes: Vec<u8>,
    pub stderr_escaped: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackView {
    pub base: String,
    pub top: String,
    pub rsp: String,
    pub rbp: String,
    pub rsp_mod_16: u8,
    pub aligned_for_call: bool,
    pub bytes: Vec<u8>,
    pub slots: Vec<StackSlotView>,
    pub frames: Vec<StackFrameView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackSlotView {
    pub address: String,
    pub value: String,
    pub signed: String,
    pub offset_from_rbp: Option<i64>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackFrameView {
    pub depth: usize,
    pub function: Option<String>,
    pub rbp: String,
    pub saved_rbp: String,
    pub return_address: String,
    pub return_location: Option<SourceLocation>,
    pub aligned_at_call: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterView {
    pub name: String,
    pub hex: String,
    pub signed: String,
    pub unsigned: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagsView {
    pub cf: bool,
    pub pf: bool,
    pub af: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
}

impl From<Flags> for FlagsView {
    fn from(flags: Flags) -> Self {
        Self {
            cf: flags.cf,
            pf: flags.pf,
            af: flags.af,
            zf: flags.zf,
            sf: flags.sf,
            of: flags.of,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepEvent {
    Instruction {
        location: SourceLocation,
        text: String,
    },
    RegisterRead {
        register: String,
        value: String,
        width: u8,
    },
    RegisterWrite {
        register: String,
        canonical: String,
        before: String,
        after: String,
        width: u8,
    },
    EffectiveAddress {
        expression: String,
        address: String,
        symbol: Option<String>,
    },
    MemoryRead {
        address: String,
        value: String,
        width: usize,
        symbol: Option<String>,
    },
    MemoryWrite {
        address: String,
        before: String,
        after: String,
        width: usize,
        symbol: Option<String>,
    },
    Arithmetic {
        operation: String,
        left: String,
        right: String,
        result: String,
        width: u8,
    },
    Division {
        dividend_high: String,
        dividend_low: String,
        divisor: String,
        quotient: String,
        remainder: String,
        width: u8,
    },
    FlagsChanged {
        before: FlagsView,
        after: FlagsView,
    },
    Compare {
        destination: String,
        source: String,
        result: String,
        width: u8,
    },
    Branch {
        condition: String,
        predicate: String,
        target: String,
        taken: bool,
    },
    Call {
        target: String,
        return_address: String,
        return_location: Option<SourceLocation>,
        stack_pointer_before: String,
        aligned_before: bool,
    },
    Return {
        return_address: String,
        return_location: Option<SourceLocation>,
    },
    StackPush {
        value: String,
        stack_pointer: String,
    },
    StackPop {
        value: String,
        stack_pointer: String,
    },
    Syscall {
        number: String,
        name: Option<String>,
    },
    Exit {
        raw_hex: String,
        signed: String,
        shell_status: u8,
    },
    Output {
        fd: u64,
        bytes: Vec<u8>,
        escaped: String,
    },
    InputRequested {
        fd: u64,
        address: String,
        count: usize,
    },
    InputSubmitted {
        bytes: Vec<u8>,
        escaped: String,
    },
    InputRead {
        fd: u64,
        bytes: Vec<u8>,
        escaped: String,
    },
    Fault {
        code: String,
        message: String,
    },
    Reversed {
        location: SourceLocation,
    },
    Reset,
}

pub(crate) fn hex64(value: u64) -> String {
    format!("0x{value:016x}")
}

pub(crate) fn escaped_bytes(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),
            b'\\' => escaped.push_str("\\\\"),
            0 => escaped.push_str("\\0"),
            0x20..=0x7e => escaped.push(char::from(byte)),
            _ => escaped.push_str(&format!("\\x{byte:02x}")),
        }
    }
    escaped
}
