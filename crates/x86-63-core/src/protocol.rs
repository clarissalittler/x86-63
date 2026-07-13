use serde::{Deserialize, Serialize};

use crate::{Diagnostic, Flags, MachineStatus, SourceLocation};

pub const PROTOCOL_VERSION: u32 = 1;

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
    pub history_depth: usize,
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
    Syscall {
        number: String,
        name: Option<String>,
    },
    Exit {
        raw_hex: String,
        signed: String,
        shell_status: u8,
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
