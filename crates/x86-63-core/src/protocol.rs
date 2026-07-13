use serde::{Deserialize, Serialize};

use crate::{Diagnostic, Flags, MachineStatus, SourceLocation, SymbolView};

// Version 2 adds data memory, process I/O, effective-address receipts, and
// branch/output events to every machine view and command result.
pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Reset,
    Step,
    Next,
    Back,
    Continue { max_steps: usize },
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
    pub stdout_bytes: Vec<u8>,
    pub stdout_escaped: String,
    pub stderr_bytes: Vec<u8>,
    pub stderr_escaped: String,
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
