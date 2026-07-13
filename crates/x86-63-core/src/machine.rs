use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::program::{DATA_BASE, Instruction, Program, SourceLocation};
use crate::protocol::{FlagsView, StepEvent, escaped_bytes, hex64};

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
    Memory(MemoryAddress),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MemoryAddress {
    pub(crate) text: String,
    pub(crate) symbol: Option<String>,
    pub(crate) symbol_address: Option<u64>,
    pub(crate) displacement: i64,
    pub(crate) base: Option<RegisterRef>,
    pub(crate) index: Option<RegisterRef>,
    pub(crate) scale: u8,
    pub(crate) rip_relative: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JumpCondition {
    Always,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Below,
    BelowOrEqual,
    Above,
    AboveOrEqual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Mov {
        source: Operand,
        destination: Operand,
        width: u8,
    },
    Add {
        source: Operand,
        destination: Operand,
        width: u8,
    },
    Sub {
        source: Operand,
        destination: Operand,
        width: u8,
    },
    Lea {
        source: MemoryAddress,
        destination: RegisterRef,
        width: u8,
    },
    Cmp {
        source: Operand,
        destination: Operand,
        width: u8,
    },
    Xor {
        source: Operand,
        destination: Operand,
        width: u8,
    },
    Jump {
        condition: JumpCondition,
        target: usize,
        target_label: String,
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
    pub(crate) memory: Vec<u8>,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    data_symbols: BTreeMap<String, u64>,
}

impl Machine {
    pub(crate) fn new(program: &Program) -> Self {
        Self {
            registers: RegisterFile::default(),
            flags: Flags::default(),
            pc: program.entry(),
            status: MachineStatus::Paused,
            memory: program.initial_data().to_vec(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            data_symbols: program.data_symbols.clone(),
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
        let stdout_len_before = self.stdout.len();
        let stderr_len_before = self.stderr.len();
        let mut register_writes = Vec::new();
        let mut memory_writes = Vec::new();
        let mut events = vec![StepEvent::Instruction {
            location: instruction.location.clone(),
            text: instruction.text.clone(),
        }];

        self.pc += 1;
        let explanation = match self.apply_instruction(
            &instruction,
            &mut register_writes,
            &mut memory_writes,
            &mut events,
        ) {
            Ok(explanation) => explanation,
            Err(fault) => {
                self.status = MachineStatus::Faulted {
                    code: fault.code.to_string(),
                    message: fault.message.clone(),
                };
                events.push(StepEvent::Fault {
                    code: fault.code.to_string(),
                    message: fault.message.clone(),
                });
                fault.message
            }
        };

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
            register_writes,
            memory_writes,
            stdout_len_before,
            stderr_len_before,
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
        register_writes: &mut Vec<RegisterWriteDelta>,
        memory_writes: &mut Vec<MemoryWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<String, RuntimeFault> {
        match &instruction.operation {
            Operation::Mov {
                source,
                destination,
                width,
            } => {
                let value = self.read_operand(source, *width, events)?;
                self.write_operand(
                    destination,
                    value,
                    *width,
                    register_writes,
                    memory_writes,
                    events,
                )?;
                Ok(format!(
                    "Moved {} into {}. mov does not change the flags.",
                    format_width(value, *width),
                    operand_name(destination)
                ))
            }
            Operation::Add {
                source,
                destination,
                width,
            } => self.apply_arithmetic(
                ArithmeticKind::Add,
                source,
                destination,
                *width,
                register_writes,
                memory_writes,
                events,
            ),
            Operation::Sub {
                source,
                destination,
                width,
            } => self.apply_arithmetic(
                ArithmeticKind::Sub,
                source,
                destination,
                *width,
                register_writes,
                memory_writes,
                events,
            ),
            Operation::Lea {
                source,
                destination,
                width,
            } => {
                let address = self.effective_address(source, events)?;
                self.write_register(*destination, address, register_writes, events);
                Ok(format!(
                    "lea computed {} = {} without reading memory, then wrote the address to %{}.",
                    source.text,
                    format_width(address, *width),
                    destination.name()
                ))
            }
            Operation::Cmp {
                source,
                destination,
                width,
            } => self.apply_compare(source, destination, *width, events),
            Operation::Xor {
                source,
                destination,
                width,
            } => self.apply_xor(
                source,
                destination,
                *width,
                register_writes,
                memory_writes,
                events,
            ),
            Operation::Jump {
                condition,
                target,
                target_label,
            } => Ok(self.apply_jump(*condition, *target, target_label, events)),
            Operation::Syscall => self.apply_syscall(register_writes, events),
        }
    }

    fn read_operand(
        &self,
        operand: &Operand,
        width: u8,
        events: &mut Vec<StepEvent>,
    ) -> Result<u64, RuntimeFault> {
        match operand {
            Operand::Immediate(value) => Ok(*value & width_mask(width)),
            Operand::Register(register) => {
                let value = self.registers.read(*register);
                events.push(StepEvent::RegisterRead {
                    register: register.name().to_string(),
                    value: format_width(value, width),
                    width,
                });
                Ok(value)
            }
            Operand::Memory(address) => self.read_memory(address, width, events),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_operand(
        &mut self,
        operand: &Operand,
        value: u64,
        width: u8,
        register_writes: &mut Vec<RegisterWriteDelta>,
        memory_writes: &mut Vec<MemoryWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<(), RuntimeFault> {
        match operand {
            Operand::Register(register) => {
                self.write_register(*register, value, register_writes, events);
                Ok(())
            }
            Operand::Memory(address) => {
                self.write_memory(address, value, width, memory_writes, events)
            }
            Operand::Immediate(_) => unreachable!("parser rejects immediate destinations"),
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
        destination: &Operand,
        width: u8,
        register_writes: &mut Vec<RegisterWriteDelta>,
        memory_writes: &mut Vec<MemoryWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<String, RuntimeFault> {
        let right = self.read_operand(source, width, events)?;
        let left = self.read_operand(destination, width, events)?;
        let (result, flags) = arithmetic(kind, left, right, width);
        let old_flags = self.flags;
        self.write_operand(
            destination,
            result,
            width,
            register_writes,
            memory_writes,
            events,
        )?;
        self.flags = flags;
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
        Ok(format!(
            "AT&T syntax puts the destination on the right: {} = {} {} {} = {}.",
            operand_name(destination),
            format_width(left, width),
            kind.symbol(),
            format_width(right, width),
            format_width(result, width)
        ))
    }

    fn apply_compare(
        &mut self,
        source: &Operand,
        destination: &Operand,
        width: u8,
        events: &mut Vec<StepEvent>,
    ) -> Result<String, RuntimeFault> {
        let right = self.read_operand(source, width, events)?;
        let left = self.read_operand(destination, width, events)?;
        let (result, flags) = arithmetic(ArithmeticKind::Sub, left, right, width);
        let old_flags = self.flags;
        self.flags = flags;
        events.push(StepEvent::Compare {
            destination: format_width(left, width),
            source: format_width(right, width),
            result: format_width(result, width),
            width,
        });
        events.push(StepEvent::FlagsChanged {
            before: old_flags.into(),
            after: flags.into(),
        });
        Ok(format!(
            "cmp sets flags from destination minus source: {} − {} = {}; it stores no result.",
            format_width(left, width),
            format_width(right, width),
            format_width(result, width)
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_xor(
        &mut self,
        source: &Operand,
        destination: &Operand,
        width: u8,
        register_writes: &mut Vec<RegisterWriteDelta>,
        memory_writes: &mut Vec<MemoryWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<String, RuntimeFault> {
        let right = self.read_operand(source, width, events)?;
        let left = self.read_operand(destination, width, events)?;
        let result = (left ^ right) & width_mask(width);
        let flags = logical_flags(result, width);
        let old_flags = self.flags;
        self.write_operand(
            destination,
            result,
            width,
            register_writes,
            memory_writes,
            events,
        )?;
        self.flags = flags;
        events.push(StepEvent::Arithmetic {
            operation: "xor".to_string(),
            left: format_width(left, width),
            right: format_width(right, width),
            result: format_width(result, width),
            width,
        });
        events.push(StepEvent::FlagsChanged {
            before: old_flags.into(),
            after: flags.into(),
        });
        Ok(format!(
            "{} = {} XOR {} = {}. xor clears CF and OF and updates ZF, SF, and PF.",
            operand_name(destination),
            format_width(left, width),
            format_width(right, width),
            format_width(result, width)
        ))
    }

    fn apply_jump(
        &mut self,
        condition: JumpCondition,
        target: usize,
        target_label: &str,
        events: &mut Vec<StepEvent>,
    ) -> String {
        let taken = condition.evaluate(self.flags);
        let predicate = condition.predicate(self.flags);
        if taken {
            self.pc = target;
        }
        events.push(StepEvent::Branch {
            condition: condition.mnemonic().to_string(),
            predicate: predicate.clone(),
            target: target_label.to_string(),
            taken,
        });
        format!(
            "{} checks {}. The predicate is {}, so the branch to `{}` is {}.",
            condition.mnemonic(),
            condition.meaning(),
            predicate,
            target_label,
            if taken { "taken" } else { "not taken" }
        )
    }

    fn apply_syscall(
        &mut self,
        register_writes: &mut Vec<RegisterWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<String, RuntimeFault> {
        let rax = RegisterRef::parse("rax").unwrap();
        let rdi = RegisterRef::parse("rdi").unwrap();
        let number = self.read_register(rax, events);
        events.push(StepEvent::Syscall {
            number: number.to_string(),
            name: match number {
                1 => Some("write".to_string()),
                60 => Some("exit".to_string()),
                _ => None,
            },
        });
        match number {
            1 => {
                let rsi = RegisterRef::parse("rsi").unwrap();
                let rdx = RegisterRef::parse("rdx").unwrap();
                let fd = self.read_register(rdi, events);
                let address = self.read_register(rsi, events);
                let count = self.read_register(rdx, events);
                if !matches!(fd, 1 | 2) {
                    return Err(RuntimeFault::new(
                        "bad_file_descriptor",
                        format!(
                            "write uses file descriptor {fd}; this teaching process exposes stdout as 1 and stderr as 2"
                        ),
                    ));
                }
                let count = usize::try_from(count).map_err(|_| {
                    RuntimeFault::new("invalid_write_count", "write byte count does not fit usize")
                })?;
                let bytes = self.read_bytes(address, count)?;
                events.push(StepEvent::EffectiveAddress {
                    expression: "buffer address from %rsi".to_string(),
                    address: hex64(address),
                    symbol: self.symbol_for_address(address),
                });
                events.push(StepEvent::MemoryRead {
                    address: hex64(address),
                    value: format_bytes(&bytes),
                    width: count.saturating_mul(8),
                    symbol: self.symbol_for_address(address),
                });
                if fd == 1 {
                    self.stdout.extend_from_slice(&bytes);
                } else {
                    self.stderr.extend_from_slice(&bytes);
                }
                let escaped = escaped_bytes(&bytes);
                events.push(StepEvent::Output {
                    fd,
                    bytes,
                    escaped: escaped.clone(),
                });
                self.write_register(rax, count as u64, register_writes, events);
                Ok(format!(
                    "syscall 1 is write. fd={fd}, buffer={}, count={count}; it emitted `{escaped}` and returned {count} in %rax.",
                    hex64(address)
                ))
            }
            60 => {
                let raw = self.read_register(rdi, events);
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
                Ok(format!(
                    "syscall 60 is exit. %rdi contains {}; a shell reports only its low 8 bits, {}.",
                    raw as i64, shell_status
                ))
            }
            _ => Err(RuntimeFault::new(
                "unsupported_syscall",
                format!(
                    "syscall {number} is outside the current Lecture 4 slice (write/1 and exit/60 are ready)"
                ),
            )),
        }
    }

    fn read_register(&self, register: RegisterRef, events: &mut Vec<StepEvent>) -> u64 {
        let value = self.registers.read(register);
        events.push(StepEvent::RegisterRead {
            register: register.name().to_string(),
            value: hex64(value),
            width: register.width(),
        });
        value
    }

    fn effective_address(
        &self,
        address: &MemoryAddress,
        events: &mut Vec<StepEvent>,
    ) -> Result<u64, RuntimeFault> {
        let mut value = if address.rip_relative {
            address
                .symbol_address
                .expect("parser requires a symbol for RIP-relative addressing")
        } else {
            address.symbol_address.unwrap_or(0)
        };
        value = value.wrapping_add_signed(address.displacement);
        if let Some(base) = address.base {
            value = value.wrapping_add(self.read_register(base, events));
        }
        if let Some(index) = address.index {
            value = value.wrapping_add(
                self.read_register(index, events)
                    .wrapping_mul(u64::from(address.scale)),
            );
        }
        events.push(StepEvent::EffectiveAddress {
            expression: address.text.clone(),
            address: hex64(value),
            symbol: address
                .symbol
                .clone()
                .or_else(|| self.symbol_for_address(value)),
        });
        Ok(value)
    }

    fn read_memory(
        &self,
        address: &MemoryAddress,
        width: u8,
        events: &mut Vec<StepEvent>,
    ) -> Result<u64, RuntimeFault> {
        let effective = self.effective_address(address, events)?;
        let bytes = self.read_bytes(effective, usize::from(width / 8))?;
        let mut padded = [0_u8; 8];
        padded[..bytes.len()].copy_from_slice(&bytes);
        let value = u64::from_le_bytes(padded);
        events.push(StepEvent::MemoryRead {
            address: hex64(effective),
            value: format_width(value, width),
            width: usize::from(width),
            symbol: address
                .symbol
                .clone()
                .or_else(|| self.symbol_for_address(effective)),
        });
        Ok(value)
    }

    fn write_memory(
        &mut self,
        address: &MemoryAddress,
        value: u64,
        width: u8,
        writes: &mut Vec<MemoryWriteDelta>,
        events: &mut Vec<StepEvent>,
    ) -> Result<(), RuntimeFault> {
        let effective = self.effective_address(address, events)?;
        let range = self.memory_range(effective, usize::from(width / 8))?;
        let before = self.memory[range.clone()].to_vec();
        let bytes = value.to_le_bytes();
        let after = bytes[..range.len()].to_vec();
        self.memory[range].copy_from_slice(&after);
        let symbol = address
            .symbol
            .clone()
            .or_else(|| self.symbol_for_address(effective));
        writes.push(MemoryWriteDelta {
            address: effective,
            before: before.clone(),
            after: after.clone(),
            symbol: symbol.clone(),
        });
        events.push(StepEvent::MemoryWrite {
            address: hex64(effective),
            before: format_bytes(&before),
            after: format_bytes(&after),
            width: usize::from(width),
            symbol,
        });
        Ok(())
    }

    fn read_bytes(&self, address: u64, count: usize) -> Result<Vec<u8>, RuntimeFault> {
        let range = self.memory_range(address, count)?;
        Ok(self.memory[range].to_vec())
    }

    fn memory_range(
        &self,
        address: u64,
        count: usize,
    ) -> Result<std::ops::Range<usize>, RuntimeFault> {
        let Some(offset) = address.checked_sub(DATA_BASE) else {
            return Err(self.memory_fault(address, count));
        };
        let Ok(offset) = usize::try_from(offset) else {
            return Err(self.memory_fault(address, count));
        };
        let Some(end) = offset.checked_add(count) else {
            return Err(self.memory_fault(address, count));
        };
        if end > self.memory.len() {
            return Err(self.memory_fault(address, count));
        }
        Ok(offset..end)
    }

    fn memory_fault(&self, address: u64, count: usize) -> RuntimeFault {
        RuntimeFault::new(
            "unmapped_memory",
            format!(
                "memory access at {} for {count} byte(s) is outside the mapped .data range {}..{}",
                hex64(address),
                hex64(DATA_BASE),
                hex64(DATA_BASE + self.memory.len() as u64)
            ),
        )
    }

    fn symbol_for_address(&self, address: u64) -> Option<String> {
        self.data_symbols
            .iter()
            .filter(|(_, symbol_address)| **symbol_address <= address)
            .max_by_key(|(_, symbol_address)| *symbol_address)
            .map(|(name, _)| name.clone())
    }

    pub(crate) fn undo(&mut self, delta: &StepDelta) {
        for write in delta.register_writes.iter().rev() {
            self.registers
                .restore(write.register.canonical(), write.before);
        }
        for write in delta.memory_writes.iter().rev() {
            let offset = (write.address - DATA_BASE) as usize;
            self.memory[offset..offset + write.before.len()].copy_from_slice(&write.before);
        }
        self.stdout.truncate(delta.stdout_len_before);
        self.stderr.truncate(delta.stderr_len_before);
        self.flags = delta.flags_before;
        self.pc = delta.pc_before;
        self.status = delta.status_before.clone();
    }
}

#[derive(Clone, Debug)]
struct RuntimeFault {
    code: &'static str,
    message: String,
}

impl RuntimeFault {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ArithmeticKind {
    Add,
    Sub,
}

impl JumpCondition {
    fn mnemonic(self) -> &'static str {
        match self {
            Self::Always => "jmp",
            Self::Equal => "je",
            Self::NotEqual => "jne",
            Self::Less => "jl",
            Self::LessOrEqual => "jle",
            Self::Greater => "jg",
            Self::GreaterOrEqual => "jge",
            Self::Below => "jb",
            Self::BelowOrEqual => "jbe",
            Self::Above => "ja",
            Self::AboveOrEqual => "jae",
        }
    }

    fn meaning(self) -> &'static str {
        match self {
            Self::Always => "an unconditional branch",
            Self::Equal => "equality (ZF = 1)",
            Self::NotEqual => "inequality (ZF = 0)",
            Self::Less => "signed less-than (SF ≠ OF)",
            Self::LessOrEqual => "signed less-or-equal (ZF = 1 or SF ≠ OF)",
            Self::Greater => "signed greater-than (ZF = 0 and SF = OF)",
            Self::GreaterOrEqual => "signed greater-or-equal (SF = OF)",
            Self::Below => "unsigned below (CF = 1)",
            Self::BelowOrEqual => "unsigned below-or-equal (CF = 1 or ZF = 1)",
            Self::Above => "unsigned above (CF = 0 and ZF = 0)",
            Self::AboveOrEqual => "unsigned above-or-equal (CF = 0)",
        }
    }

    fn evaluate(self, flags: Flags) -> bool {
        match self {
            Self::Always => true,
            Self::Equal => flags.zf,
            Self::NotEqual => !flags.zf,
            Self::Less => flags.sf != flags.of,
            Self::LessOrEqual => flags.zf || flags.sf != flags.of,
            Self::Greater => !flags.zf && flags.sf == flags.of,
            Self::GreaterOrEqual => flags.sf == flags.of,
            Self::Below => flags.cf,
            Self::BelowOrEqual => flags.cf || flags.zf,
            Self::Above => !flags.cf && !flags.zf,
            Self::AboveOrEqual => !flags.cf,
        }
    }

    fn predicate(self, flags: Flags) -> String {
        match self {
            Self::Always => "always true".to_string(),
            Self::Equal => format!("ZF={}", bit(flags.zf)),
            Self::NotEqual => format!("ZF={} (needs 0)", bit(flags.zf)),
            Self::Less => format!("SF={} and OF={}", bit(flags.sf), bit(flags.of)),
            Self::LessOrEqual => format!(
                "ZF={} or SF={} differs from OF={}",
                bit(flags.zf),
                bit(flags.sf),
                bit(flags.of)
            ),
            Self::Greater => format!(
                "ZF={} and SF={} equals OF={}",
                bit(flags.zf),
                bit(flags.sf),
                bit(flags.of)
            ),
            Self::GreaterOrEqual => {
                format!("SF={} equals OF={}", bit(flags.sf), bit(flags.of))
            }
            Self::Below => format!("CF={}", bit(flags.cf)),
            Self::BelowOrEqual => {
                format!("CF={} or ZF={}", bit(flags.cf), bit(flags.zf))
            }
            Self::Above => format!("CF={} and ZF={}", bit(flags.cf), bit(flags.zf)),
            Self::AboveOrEqual => format!("CF={} (needs 0)", bit(flags.cf)),
        }
    }
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

fn logical_flags(result: u64, width: u8) -> Flags {
    let result = result & width_mask(width);
    Flags {
        cf: false,
        pf: (result as u8).count_ones() & 1 == 0,
        af: false,
        zf: result == 0,
        sf: result & (1_u64 << (width - 1)) != 0,
        of: false,
    }
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

fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn operand_name(operand: &Operand) -> String {
    match operand {
        Operand::Immediate(value) => format!("${}", hex64(*value)),
        Operand::Register(register) => format!("%{}", register.name()),
        Operand::Memory(address) => address.text.clone(),
    }
}

fn bit(value: bool) -> u8 {
    u8::from(value)
}

#[derive(Clone, Debug)]
pub(crate) struct RegisterWriteDelta {
    register: RegisterRef,
    before: u64,
    after: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct MemoryWriteDelta {
    address: u64,
    before: Vec<u8>,
    after: Vec<u8>,
    symbol: Option<String>,
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
    memory_writes: Vec<MemoryWriteDelta>,
    stdout_len_before: usize,
    stderr_len_before: usize,
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
        events.extend(
            self.memory_writes
                .iter()
                .rev()
                .map(|write| StepEvent::MemoryWrite {
                    address: hex64(write.address),
                    before: format_bytes(&write.after),
                    after: format_bytes(&write.before),
                    width: write.before.len() * 8,
                    symbol: write.symbol.clone(),
                }),
        );
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

    #[test]
    fn signed_and_unsigned_jumps_read_different_flags() {
        let signed_less_with_overflow = Flags {
            sf: false,
            of: true,
            cf: false,
            zf: false,
            ..Flags::default()
        };
        assert!(JumpCondition::Less.evaluate(signed_less_with_overflow));
        assert!(!JumpCondition::GreaterOrEqual.evaluate(signed_less_with_overflow));
        assert!(!JumpCondition::Below.evaluate(signed_less_with_overflow));
        assert!(JumpCondition::Above.evaluate(signed_less_with_overflow));

        let equal = Flags {
            zf: true,
            ..Flags::default()
        };
        assert!(JumpCondition::Equal.evaluate(equal));
        assert!(JumpCondition::LessOrEqual.evaluate(equal));
        assert!(JumpCondition::BelowOrEqual.evaluate(equal));
        assert!(!JumpCondition::Greater.evaluate(equal));
        assert!(!JumpCondition::Above.evaluate(equal));
    }
}
