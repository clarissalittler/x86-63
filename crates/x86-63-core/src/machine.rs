use serde::{Deserialize, Serialize};

use crate::program::{Instruction, Program, SourceLocation};
use crate::protocol::{FlagsView, StepEvent, hex64};

const STACK_START: u64 = 0x0000_7fff_ffff_e000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flags {
    pub cf: bool,
    pub pf: bool,
    pub af: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MachineStatus {
    Paused,
    Exited {
        raw_hex: String,
        signed: String,
        shell_status: u8,
    },
    Faulted {
        code: String,
        message: String,
    },
}

impl MachineStatus {
    pub fn can_step(&self) -> bool {
        matches!(self, Self::Paused)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CanonicalRegister {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    Rbp,
    Rsp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl CanonicalRegister {
    pub(crate) const ALL: [Self; 16] = [
        Self::Rax,
        Self::Rbx,
        Self::Rcx,
        Self::Rdx,
        Self::Rsi,
        Self::Rdi,
        Self::Rbp,
        Self::Rsp,
        Self::R8,
        Self::R9,
        Self::R10,
        Self::R11,
        Self::R12,
        Self::R13,
        Self::R14,
        Self::R15,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Rax => 0,
            Self::Rbx => 1,
            Self::Rcx => 2,
            Self::Rdx => 3,
            Self::Rsi => 4,
            Self::Rdi => 5,
            Self::Rbp => 6,
            Self::Rsp => 7,
            Self::R8 => 8,
            Self::R9 => 9,
            Self::R10 => 10,
            Self::R11 => 11,
            Self::R12 => 12,
            Self::R13 => 13,
            Self::R14 => 14,
            Self::R15 => 15,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rax => "rax",
            Self::Rbx => "rbx",
            Self::Rcx => "rcx",
            Self::Rdx => "rdx",
            Self::Rsi => "rsi",
            Self::Rdi => "rdi",
            Self::Rbp => "rbp",
            Self::Rsp => "rsp",
            Self::R8 => "r8",
            Self::R9 => "r9",
            Self::R10 => "r10",
            Self::R11 => "r11",
            Self::R12 => "r12",
            Self::R13 => "r13",
            Self::R14 => "r14",
            Self::R15 => "r15",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegisterRef {
    canonical: CanonicalRegister,
    width: u8,
    name: &'static str,
}

impl RegisterRef {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        let legacy = [
            (CanonicalRegister::Rax, ["rax", "eax", "ax", "al"]),
            (CanonicalRegister::Rbx, ["rbx", "ebx", "bx", "bl"]),
            (CanonicalRegister::Rcx, ["rcx", "ecx", "cx", "cl"]),
            (CanonicalRegister::Rdx, ["rdx", "edx", "dx", "dl"]),
        ];
        for (canonical, names) in legacy {
            if let Some(index) = names.iter().position(|candidate| *candidate == name) {
                return Some(Self {
                    canonical,
                    width: [64, 32, 16, 8][index],
                    name: names[index],
                });
            }
        }

        let middle = [
            (CanonicalRegister::Rsi, ["rsi", "esi", "si", "sil"]),
            (CanonicalRegister::Rdi, ["rdi", "edi", "di", "dil"]),
            (CanonicalRegister::Rbp, ["rbp", "ebp", "bp", "bpl"]),
            (CanonicalRegister::Rsp, ["rsp", "esp", "sp", "spl"]),
        ];
        for (canonical, names) in middle {
            if let Some(index) = names.iter().position(|candidate| *candidate == name) {
                return Some(Self {
                    canonical,
                    width: [64, 32, 16, 8][index],
                    name: names[index],
                });
            }
        }

        for (number, canonical) in [
            (8, CanonicalRegister::R8),
            (9, CanonicalRegister::R9),
            (10, CanonicalRegister::R10),
            (11, CanonicalRegister::R11),
            (12, CanonicalRegister::R12),
            (13, CanonicalRegister::R13),
            (14, CanonicalRegister::R14),
            (15, CanonicalRegister::R15),
        ] {
            for (suffix, width) in [("", 64), ("d", 32), ("w", 16), ("b", 8)] {
                let candidate = format!("r{number}{suffix}");
                if name == candidate {
                    let static_name = match (number, suffix) {
                        (8, "") => "r8",
                        (8, "d") => "r8d",
                        (8, "w") => "r8w",
                        (8, "b") => "r8b",
                        (9, "") => "r9",
                        (9, "d") => "r9d",
                        (9, "w") => "r9w",
                        (9, "b") => "r9b",
                        (10, "") => "r10",
                        (10, "d") => "r10d",
                        (10, "w") => "r10w",
                        (10, "b") => "r10b",
                        (11, "") => "r11",
                        (11, "d") => "r11d",
                        (11, "w") => "r11w",
                        (11, "b") => "r11b",
                        (12, "") => "r12",
                        (12, "d") => "r12d",
                        (12, "w") => "r12w",
                        (12, "b") => "r12b",
                        (13, "") => "r13",
                        (13, "d") => "r13d",
                        (13, "w") => "r13w",
                        (13, "b") => "r13b",
                        (14, "") => "r14",
                        (14, "d") => "r14d",
                        (14, "w") => "r14w",
                        (14, "b") => "r14b",
                        (15, "") => "r15",
                        (15, "d") => "r15d",
                        (15, "w") => "r15w",
                        (15, "b") => "r15b",
                        _ => unreachable!(),
                    };
                    return Some(Self {
                        canonical,
                        width,
                        name: static_name,
                    });
                }
            }
        }
        None
    }

    pub(crate) fn width(self) -> u8 {
        self.width
    }

    pub(crate) fn canonical(self) -> CanonicalRegister {
        self.canonical
    }

    pub(crate) fn name(self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    Immediate(u64),
    Register(RegisterRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Mov {
        source: Operand,
        destination: RegisterRef,
        width: u8,
    },
    Add {
        source: Operand,
        destination: RegisterRef,
        width: u8,
    },
    Sub {
        source: Operand,
        destination: RegisterRef,
        width: u8,
    },
    Syscall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegisterFile {
    values: [u64; 16],
}

impl Default for RegisterFile {
    fn default() -> Self {
        let mut values = [0; 16];
        values[CanonicalRegister::Rsp.index()] = STACK_START;
        Self { values }
    }
}

impl RegisterFile {
    pub(crate) fn canonical(&self, register: CanonicalRegister) -> u64 {
        self.values[register.index()]
    }

    pub(crate) fn read(&self, register: RegisterRef) -> u64 {
        self.canonical(register.canonical) & width_mask(register.width)
    }

    pub(crate) fn write(&mut self, register: RegisterRef, value: u64) -> (u64, u64) {
        let index = register.canonical.index();
        let before = self.values[index];
        let truncated = value & width_mask(register.width);
        self.values[index] = match register.width {
            64 => truncated,
            32 => truncated,
            16 | 8 => (before & !width_mask(register.width)) | truncated,
            _ => unreachable!("validated register width"),
        };
        (before, self.values[index])
    }

    fn restore(&mut self, register: CanonicalRegister, value: u64) {
        self.values[register.index()] = value;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Machine {
    pub(crate) registers: RegisterFile,
    pub(crate) flags: Flags,
    pub(crate) pc: usize,
    pub(crate) status: MachineStatus,
}

impl Machine {
    pub(crate) fn new(entry: usize) -> Self {
        Self {
            registers: RegisterFile::default(),
            flags: Flags::default(),
            pc: entry,
            status: MachineStatus::Paused,
        }
    }

    pub(crate) fn execute(&mut self, program: &Program) -> ExecutedStep {
        let instruction = program
            .instruction(self.pc)
            .expect("a paused machine always points at an instruction")
            .clone();
        let before_pc = self.pc;
        let before_status = self.status.clone();
        let before_flags = self.flags;
        let mut writes = Vec::new();
        let mut events = vec![StepEvent::Instruction {
            location: instruction.location.clone(),
            text: instruction.text.clone(),
        }];

        self.pc += 1;
        let explanation = self.apply_instruction(&instruction, &mut writes, &mut events);

        if self.status.can_step() && self.pc >= program.instructions.len() {
            let message =
                "execution fell off the end of .text; _start has no implicit return".to_string();
            self.status = MachineStatus::Faulted {
                code: "fell_off_text".to_string(),
                message: message.clone(),
            };
            events.push(StepEvent::Fault {
                code: "fell_off_text".to_string(),
                message,
            });
        }

        let delta = StepDelta {
            location: instruction.location,
            pc_before: before_pc,
            pc_after: self.pc,
            status_before: before_status,
            status_after: self.status.clone(),
            flags_before: before_flags,
            flags_after: self.flags,
            register_writes: writes,
        };
        ExecutedStep {
            delta,
            events,
            explanation,
        }
    }

    fn apply_instruction(
        &mut self,
        instruction: &Instruction,
        writes: &mut Vec<RegisterWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> String {
        match &instruction.operation {
            Operation::Mov {
                source,
                destination,
                width,
            } => {
                let value = self.read_operand(source, *width, events);
                self.write_register(*destination, value, writes, events);
                format!(
                    "Moved {} into %{}. mov does not change the flags.",
                    format_width(value, *width),
                    destination.name()
                )
            }
            Operation::Add {
                source,
                destination,
                width,
            } => self.apply_arithmetic(
                ArithmeticKind::Add,
                source,
                *destination,
                *width,
                writes,
                events,
            ),
            Operation::Sub {
                source,
                destination,
                width,
            } => self.apply_arithmetic(
                ArithmeticKind::Sub,
                source,
                *destination,
                *width,
                writes,
                events,
            ),
            Operation::Syscall => self.apply_syscall(events),
        }
    }

    fn read_operand(&self, operand: &Operand, width: u8, events: &mut Vec<StepEvent>) -> u64 {
        match operand {
            Operand::Immediate(value) => *value & width_mask(width),
            Operand::Register(register) => {
                let value = self.registers.read(*register);
                events.push(StepEvent::RegisterRead {
                    register: register.name().to_string(),
                    value: format_width(value, width),
                    width,
                });
                value
            }
        }
    }

    fn write_register(
        &mut self,
        register: RegisterRef,
        value: u64,
        writes: &mut Vec<RegisterWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) {
        let (before, after) = self.registers.write(register, value);
        writes.push(RegisterWriteDelta {
            register,
            before,
            after,
        });
        events.push(StepEvent::RegisterWrite {
            register: register.name().to_string(),
            canonical: register.canonical().name().to_string(),
            before: hex64(before),
            after: hex64(after),
            width: register.width(),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_arithmetic(
        &mut self,
        kind: ArithmeticKind,
        source: &Operand,
        destination: RegisterRef,
        width: u8,
        writes: &mut Vec<RegisterWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> String {
        let right = self.read_operand(source, width, events);
        let left = self.registers.read(destination);
        events.push(StepEvent::RegisterRead {
            register: destination.name().to_string(),
            value: format_width(left, width),
            width,
        });
        let (result, flags) = arithmetic(kind, left, right, width);
        let old_flags = self.flags;
        self.flags = flags;
        self.write_register(destination, result, writes, events);
        events.push(StepEvent::Arithmetic {
            operation: kind.name().to_string(),
            left: format_width(left, width),
            right: format_width(right, width),
            result: format_width(result, width),
            width,
        });
        events.push(StepEvent::FlagsChanged {
            before: FlagsView::from(old_flags),
            after: FlagsView::from(flags),
        });
        format!(
            "AT&T syntax puts the destination on the right: %{} = {} {} {} = {}.",
            destination.name(),
            format_width(left, width),
            kind.symbol(),
            format_width(right, width),
            format_width(result, width)
        )
    }

    fn apply_syscall(&mut self, events: &mut Vec<StepEvent>) -> String {
        let rax = RegisterRef::parse("rax").unwrap();
        let rdi = RegisterRef::parse("rdi").unwrap();
        let number = self.registers.read(rax);
        events.push(StepEvent::RegisterRead {
            register: "rax".to_string(),
            value: hex64(number),
            width: 64,
        });
        events.push(StepEvent::Syscall {
            number: number.to_string(),
            name: (number == 60).then(|| "exit".to_string()),
        });
        if number == 60 {
            let raw = self.registers.read(rdi);
            events.push(StepEvent::RegisterRead {
                register: "rdi".to_string(),
                value: hex64(raw),
                width: 64,
            });
            let shell_status = (raw & 0xff) as u8;
            let signed = (raw as i64).to_string();
            self.status = MachineStatus::Exited {
                raw_hex: hex64(raw),
                signed: signed.clone(),
                shell_status,
            };
            events.push(StepEvent::Exit {
                raw_hex: hex64(raw),
                signed,
                shell_status,
            });
            format!(
                "syscall 60 is exit. %rdi contains {}; a shell reports only its low 8 bits, {}.",
                raw as i64, shell_status
            )
        } else {
            let message = format!(
                "syscall {number} is outside the current Lecture 3 slice (only exit/60 is ready)"
            );
            self.status = MachineStatus::Faulted {
                code: "unsupported_syscall".to_string(),
                message: message.clone(),
            };
            events.push(StepEvent::Fault {
                code: "unsupported_syscall".to_string(),
                message: message.clone(),
            });
            message
        }
    }

    pub(crate) fn undo(&mut self, delta: &StepDelta) {
        for write in delta.register_writes.iter().rev() {
            self.registers
                .restore(write.register.canonical(), write.before);
        }
        self.flags = delta.flags_before;
        self.pc = delta.pc_before;
        self.status = delta.status_before.clone();
    }
}

#[derive(Clone, Copy, Debug)]
enum ArithmeticKind {
    Add,
    Sub,
}

impl ArithmeticKind {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "−",
        }
    }
}

fn arithmetic(kind: ArithmeticKind, left: u64, right: u64, width: u8) -> (u64, Flags) {
    let mask = width_mask(width);
    let sign = 1_u64 << (width - 1);
    let left = left & mask;
    let right = right & mask;
    let result = match kind {
        ArithmeticKind::Add => left.wrapping_add(right) & mask,
        ArithmeticKind::Sub => left.wrapping_sub(right) & mask,
    };
    let cf = match kind {
        ArithmeticKind::Add if width == 64 => left.overflowing_add(right).1,
        ArithmeticKind::Add => (left as u128 + right as u128) > mask as u128,
        ArithmeticKind::Sub => left < right,
    };
    let left_sign = left & sign != 0;
    let right_sign = right & sign != 0;
    let result_sign = result & sign != 0;
    let of = match kind {
        ArithmeticKind::Add => left_sign == right_sign && result_sign != left_sign,
        ArithmeticKind::Sub => left_sign != right_sign && result_sign != left_sign,
    };
    (
        result,
        Flags {
            cf,
            pf: (result as u8).count_ones() & 1 == 0,
            af: (left ^ right ^ result) & 0x10 != 0,
            zf: result == 0,
            sf: result_sign,
            of,
        },
    )
}

fn width_mask(width: u8) -> u64 {
    match width {
        64 => u64::MAX,
        32 => u32::MAX as u64,
        16 => u16::MAX as u64,
        8 => u8::MAX as u64,
        _ => unreachable!("validated width"),
    }
}

fn format_width(value: u64, width: u8) -> String {
    format!(
        "0x{:0digits$x}",
        value & width_mask(width),
        digits = width as usize / 4
    )
}

#[derive(Clone, Debug)]
pub(crate) struct RegisterWriteDelta {
    register: RegisterRef,
    before: u64,
    after: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct StepDelta {
    pub(crate) location: SourceLocation,
    pc_before: usize,
    #[allow(dead_code)]
    pc_after: usize,
    status_before: MachineStatus,
    #[allow(dead_code)]
    status_after: MachineStatus,
    flags_before: Flags,
    #[allow(dead_code)]
    flags_after: Flags,
    register_writes: Vec<RegisterWriteDelta>,
}

impl StepDelta {
    pub(crate) fn reversed_events(&self) -> Vec<StepEvent> {
        let mut events = self
            .register_writes
            .iter()
            .rev()
            .map(|write| StepEvent::RegisterWrite {
                register: write.register.name().to_string(),
                canonical: write.register.canonical().name().to_string(),
                before: hex64(write.after),
                after: hex64(write.before),
                width: write.register.width(),
            })
            .collect::<Vec<_>>();
        if self.flags_before != self.flags_after {
            events.push(StepEvent::FlagsChanged {
                before: self.flags_after.into(),
                after: self.flags_before.into(),
            });
        }
        events
    }
}

pub(crate) struct ExecutedStep {
    pub(crate) delta: StepDelta,
    pub(crate) events: Vec<StepEvent>,
    pub(crate) explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_eax_clears_the_upper_half() {
        let mut registers = RegisterFile::default();
        let rax = RegisterRef::parse("rax").unwrap();
        let eax = RegisterRef::parse("eax").unwrap();
        registers.write(rax, u64::MAX);
        registers.write(eax, 0x1234_5678);
        assert_eq!(registers.read(rax), 0x1234_5678);
    }

    #[test]
    fn writing_al_preserves_the_upper_bits() {
        let mut registers = RegisterFile::default();
        let rax = RegisterRef::parse("rax").unwrap();
        let al = RegisterRef::parse("al").unwrap();
        registers.write(rax, 0x1122_3344_5566_7788);
        registers.write(al, 0xaa);
        assert_eq!(registers.read(rax), 0x1122_3344_5566_77aa);
    }

    #[test]
    fn add_sets_signed_overflow_without_unsigned_carry() {
        let (result, flags) = arithmetic(ArithmeticKind::Add, i64::MAX as u64, 1, 64);
        assert_eq!(result, i64::MIN as u64);
        assert!(flags.of);
        assert!(!flags.cf);
        assert!(flags.sf);
    }

    #[test]
    fn sub_sets_borrow_and_zero_signals() {
        let (result, flags) = arithmetic(ArithmeticKind::Sub, 0, 1, 64);
        assert_eq!(result, u64::MAX);
        assert!(flags.cf);
        assert!(flags.sf);
        assert!(!flags.zf);
    }
}
