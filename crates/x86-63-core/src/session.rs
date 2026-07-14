use std::fmt;

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::machine::{CanonicalRegister, Machine, STACK_BASE, STACK_SIZE, STACK_START, StepDelta};
use crate::parser::compile;
use crate::program::{DATA_BASE, Program, ProgramView, SourceModule};
use crate::protocol::{
    Command, CommandResult, IoView, MachineView, MemoryView, PROTOCOL_VERSION, RegisterView,
    StackFrameView, StackSlotView, StackView, StepEvent, escaped_bytes, hex64,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildError {
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "source contains {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for BuildError {}

pub struct Session {
    program: Program,
    machine: Machine,
    history: Vec<StepDelta>,
    last_explanation: String,
}

impl Session {
    pub fn from_modules(modules: Vec<SourceModule>) -> Result<Self, BuildError> {
        let program = compile(modules).map_err(|diagnostics| BuildError { diagnostics })?;
        let machine = Machine::new(&program);
        Ok(Self {
            program,
            machine,
            history: Vec::new(),
            last_explanation: "Ready at _start. Step to execute the highlighted instruction."
                .to_string(),
        })
    }

    pub fn execute(&mut self, command: Command) -> CommandResult {
        match command {
            Command::Reset => self.reset(),
            Command::Step => self.step(),
            Command::Next => self.next(),
            Command::Back => self.back(),
            Command::Continue { max_steps } => self.continue_for(max_steps),
            Command::SubmitInput { text } => self.submit_input(&text),
        }
    }

    pub fn view(&self) -> MachineView {
        let next = self.program.instruction(self.machine.pc);
        let symbols = self.program.view().symbols;
        MachineView {
            protocol_version: PROTOCOL_VERSION,
            status: self.machine.status.clone(),
            next_instruction: next.map(|instruction| instruction.location.clone()),
            next_text: next.map(|instruction| instruction.text.clone()),
            registers: CanonicalRegister::ALL
                .into_iter()
                .map(|register| {
                    let value = self.machine.registers.canonical(register);
                    RegisterView {
                        name: register.name().to_string(),
                        hex: hex64(value),
                        signed: (value as i64).to_string(),
                        unsigned: value.to_string(),
                    }
                })
                .collect(),
            flags: self.machine.flags.into(),
            memory: MemoryView {
                base: hex64(DATA_BASE),
                bytes: self.machine.memory.clone(),
                symbols,
            },
            stack: self.stack_view(),
            io: IoView {
                stdin_bytes: self.machine.stdin.clone(),
                stdin_escaped: escaped_bytes(&self.machine.stdin),
                stdin_consumed: self.machine.stdin_cursor,
                stdout_bytes: self.machine.stdout.clone(),
                stdout_escaped: escaped_bytes(&self.machine.stdout),
                stderr_bytes: self.machine.stderr.clone(),
                stderr_escaped: escaped_bytes(&self.machine.stderr),
            },
            history_depth: self.history.len(),
        }
    }

    pub fn program(&self) -> ProgramView {
        self.program.view()
    }

    pub fn last_explanation(&self) -> &str {
        &self.last_explanation
    }

    fn stack_view(&self) -> StackView {
        let rsp = self.machine.registers.canonical(CanonicalRegister::Rsp);
        let rbp = self.machine.registers.canonical(CanonicalRegister::Rbp);
        let active_base = if (STACK_BASE..=STACK_START).contains(&rsp) {
            rsp
        } else {
            STACK_START
        };
        let start = usize::try_from(active_base - STACK_BASE)
            .unwrap_or(STACK_SIZE)
            .min(STACK_SIZE);
        let bytes = self.machine.stack[start..].to_vec();
        let slots = bytes
            .chunks(8)
            .enumerate()
            .map(|(index, chunk)| {
                let address = active_base + (index * 8) as u64;
                let mut padded = [0_u8; 8];
                padded[..chunk.len()].copy_from_slice(chunk);
                let value = u64::from_le_bytes(padded);
                let offset_from_rbp = (rbp != 0)
                    .then(|| address as i128 - rbp as i128)
                    .and_then(|offset| i64::try_from(offset).ok());
                let mut labels = Vec::new();
                if address == rsp {
                    labels.push("top of stack (%rsp)".to_string());
                }
                if address == rbp && rbp != 0 {
                    labels.push("saved caller %rbp".to_string());
                } else if address == rbp.wrapping_add(8) && rbp != 0 {
                    labels.push("frame return address".to_string());
                } else if let Some(offset) = offset_from_rbp.filter(|offset| *offset < 0) {
                    labels.push(format!("local {offset}(%rbp)"));
                }
                if let Some(location) = self.program.location_for_text_address(value) {
                    labels.push(format!("return to {}:{}", location.module, location.line));
                }
                StackSlotView {
                    address: hex64(address),
                    value: hex64(value),
                    signed: (value as i64).to_string(),
                    offset_from_rbp,
                    label: (!labels.is_empty()).then(|| labels.join(" · ")),
                }
            })
            .collect();
        let frames = self.stack_frames(rbp);
        StackView {
            base: hex64(active_base),
            top: hex64(STACK_START),
            rsp: hex64(rsp),
            rbp: hex64(rbp),
            rsp_mod_16: (rsp % 16) as u8,
            aligned_for_call: rsp % 16 == 0,
            bytes,
            slots,
            frames,
        }
    }

    fn stack_frames(&self, initial_rbp: u64) -> Vec<StackFrameView> {
        let mut frames = Vec::new();
        let mut rbp = initial_rbp;
        let mut seen = Vec::new();
        while rbp != 0 && frames.len() < 128 && !seen.contains(&rbp) {
            seen.push(rbp);
            let Some(saved_rbp) = self.stack_quad(rbp) else {
                break;
            };
            let Some(return_address) = rbp
                .checked_add(8)
                .and_then(|address| self.stack_quad(address))
            else {
                break;
            };
            frames.push(StackFrameView {
                depth: frames.len(),
                function: self
                    .program
                    .call_target_for_return_address(return_address)
                    .map(str::to_string),
                rbp: hex64(rbp),
                saved_rbp: hex64(saved_rbp),
                return_address: hex64(return_address),
                return_location: self
                    .program
                    .location_for_text_address(return_address)
                    .cloned(),
                aligned_at_call: rbp
                    .checked_add(16)
                    .is_some_and(|stack_pointer| stack_pointer % 16 == 0),
            });
            rbp = saved_rbp;
        }
        frames
    }

    fn stack_quad(&self, address: u64) -> Option<u64> {
        let offset = usize::try_from(address.checked_sub(STACK_BASE)?).ok()?;
        let bytes: [u8; 8] = self
            .machine
            .stack
            .get(offset..offset.checked_add(8)?)?
            .try_into()
            .ok()?;
        Some(u64::from_le_bytes(bytes))
    }

    fn reset(&mut self) -> CommandResult {
        self.machine = Machine::new(&self.program);
        self.history.clear();
        self.last_explanation =
            "Reset to _start. Registers are zero except for the aligned stack pointer.".to_string();
        self.result(0, vec![StepEvent::Reset], Vec::new())
    }

    fn step(&mut self) -> CommandResult {
        if !self.machine.status.can_step() {
            let diagnostic = Diagnostic::error(
                "E300",
                "the machine is halted; use `back` or `reset` before stepping",
                None,
            );
            return self.result(0, Vec::new(), vec![diagnostic]);
        }
        let executed = self.machine.execute(&self.program);
        self.last_explanation = executed.explanation;
        let steps = usize::from(executed.completed);
        if executed.completed {
            self.history.push(executed.delta);
        }
        self.result(steps, executed.events, Vec::new())
    }

    fn next(&mut self) -> CommandResult {
        if !self.machine.status.can_step() || !self.program.current_is_call(self.machine.pc) {
            return self.step();
        }

        let return_pc = self.machine.pc + 1;
        let mut steps = 0;
        let mut events = Vec::new();
        const NEXT_LIMIT: usize = 10_000;
        while self.machine.status.can_step() && steps < NEXT_LIMIT {
            let executed = self.machine.execute(&self.program);
            self.last_explanation = executed.explanation;
            events.extend(executed.events);
            if !executed.completed {
                break;
            }
            self.history.push(executed.delta);
            steps += 1;
            if self.machine.pc == return_pc {
                self.last_explanation = format!(
                    "Next stepped over the call and stopped at its return point after {steps} instruction(s)."
                );
                break;
            }
        }
        let diagnostics = if self.machine.status.can_step() && self.machine.pc != return_pc {
            vec![Diagnostic::error(
                "E304",
                format!("next stopped after the {NEXT_LIMIT}-instruction safety limit"),
                None,
            )]
        } else {
            Vec::new()
        };
        self.result(steps, events, diagnostics)
    }

    fn back(&mut self) -> CommandResult {
        if self.machine.cancel_input_wait() {
            self.last_explanation =
                "Cancelled the blocked read and returned to the same syscall instruction."
                    .to_string();
            return self.result(0, Vec::new(), Vec::new());
        }
        let Some(delta) = self.history.pop() else {
            let diagnostic = Diagnostic::error(
                "E301",
                "already at the beginning of execution history",
                None,
            );
            return self.result(0, Vec::new(), vec![diagnostic]);
        };
        let location = delta.location.clone();
        let mut events = delta.reversed_events();
        self.machine.undo(&delta);
        self.last_explanation = format!(
            "Reversed the instruction at {}:{} and restored its previous machine state.",
            location.module, location.line
        );
        events.push(StepEvent::Reversed { location });
        self.result(0, events, Vec::new())
    }

    fn submit_input(&mut self, text: &str) -> CommandResult {
        match self.machine.submit_input_line(text) {
            Ok(bytes) => {
                let escaped = escaped_bytes(&bytes);
                self.last_explanation = format!(
                    "Submitted one terminal line (`{escaped}`). A waiting read can now be stepped or continued."
                );
                self.result(
                    0,
                    vec![StepEvent::InputSubmitted { bytes, escaped }],
                    Vec::new(),
                )
            }
            Err(message) => self.result(
                0,
                Vec::new(),
                vec![Diagnostic::error("E303", message, None)],
            ),
        }
    }

    fn continue_for(&mut self, max_steps: usize) -> CommandResult {
        let max_steps = max_steps.max(1);
        let mut steps = 0;
        let mut events = Vec::new();
        while self.machine.status.can_step() && steps < max_steps {
            let executed = self.machine.execute(&self.program);
            self.last_explanation = executed.explanation;
            events.extend(executed.events);
            if !executed.completed {
                break;
            }
            self.history.push(executed.delta);
            steps += 1;
        }
        let diagnostics = if self.machine.status.can_step() {
            vec![
                Diagnostic::error(
                    "E302",
                    format!("continue stopped after the {max_steps}-instruction safety limit"),
                    None,
                )
                .with_help("step manually or continue again after checking for an infinite loop"),
            ]
        } else {
            Vec::new()
        };
        self.result(steps, events, diagnostics)
    }

    fn result(
        &self,
        steps_executed: usize,
        events: Vec<StepEvent>,
        diagnostics: Vec<Diagnostic>,
    ) -> CommandResult {
        CommandResult {
            protocol_version: PROTOCOL_VERSION,
            steps_executed,
            view: self.view(),
            events,
            diagnostics,
            explanation: self.last_explanation.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MachineStatus;

    fn session(source: &str) -> Session {
        Session::from_modules(vec![SourceModule::new("test.s", source)]).unwrap()
    }

    fn register(view: &MachineView, name: &str) -> String {
        view.registers
            .iter()
            .find(|register| register.name == name)
            .unwrap()
            .hex
            .clone()
    }

    #[test]
    fn first_program_faults_after_falling_off_text() {
        let mut session = session(".text\n.global _start\n_start:\n mov $60,%rax\n");
        let result = session.execute(Command::Step);
        assert_eq!(register(&result.view, "rax"), "0x000000000000003c");
        assert!(matches!(result.view.status, MachineStatus::Faulted { .. }));
    }

    #[test]
    fn exit_reports_the_shell_visible_low_byte() {
        let mut session =
            session(".text\n.global _start\n_start:\n mov $60,%rax\n mov $-1,%rdi\n syscall\n");
        let result = session.execute(Command::Continue { max_steps: 10 });
        assert!(matches!(
            result.view.status,
            MachineStatus::Exited {
                shell_status: 255,
                ..
            }
        ));
    }

    #[test]
    fn reverse_step_restores_registers_and_status() {
        let mut session = session(".text\n.global _start\n_start:\n mov $60,%rax\n");
        session.execute(Command::Step);
        let result = session.execute(Command::Back);
        assert_eq!(register(&result.view, "rax"), "0x0000000000000000");
        assert_eq!(result.view.status, MachineStatus::Paused);
        assert!(result.events.iter().any(|event| matches!(
            event,
            StepEvent::RegisterWrite {
                canonical,
                after,
                ..
            } if canonical == "rax" && after == "0x0000000000000000"
        )));
    }

    #[test]
    fn unmapped_data_access_faults_and_can_be_reversed() {
        let mut session = session(
            ".data\nnum: .quad 1\n.text\n.global _start\n_start:\n lea num(%rip),%rbx\n add $8,%rbx\n movq (%rbx),%rdi\n",
        );
        let fault = session.execute(Command::Continue { max_steps: 10 });
        assert!(matches!(
            fault.view.status,
            MachineStatus::Faulted { ref code, .. } if code == "unmapped_memory"
        ));
        assert_eq!(fault.view.memory.bytes, 1_u64.to_le_bytes());
        assert!(fault.events.iter().any(|event| matches!(
            event,
            StepEvent::EffectiveAddress { address, .. }
                if address == "0x0000000000400008"
        )));

        let reversed = session.execute(Command::Back);
        assert_eq!(reversed.view.status, MachineStatus::Paused);
        assert_eq!(
            reversed.view.next_text.as_deref().map(str::trim),
            Some("movq (%rbx),%rdi")
        );
    }

    #[test]
    fn write_rejects_unknown_file_descriptors_without_output() {
        let mut session = session(
            ".data\nmsg: .byte 65\n.text\n.global _start\n_start:\n mov $1,%rax\n mov $9,%rdi\n lea msg(%rip),%rsi\n mov $1,%rdx\n syscall\n",
        );
        let fault = session.execute(Command::Continue { max_steps: 10 });
        assert!(matches!(
            fault.view.status,
            MachineStatus::Faulted { ref code, .. } if code == "bad_file_descriptor"
        ));
        assert!(fault.view.io.stdout_bytes.is_empty());
        assert!(fault.view.io.stderr_bytes.is_empty());
    }

    #[test]
    fn divq_reports_both_results_and_faults_on_zero() {
        let mut successful = session(
            ".text\n.global _start\n_start:\n mov $123,%rax\n xor %rdx,%rdx\n mov $10,%rbx\n divq %rbx\n mov $60,%rax\n syscall\n",
        );
        successful.execute(Command::Step);
        successful.execute(Command::Step);
        successful.execute(Command::Step);
        let division = successful.execute(Command::Step);
        assert_eq!(register(&division.view, "rax"), "0x000000000000000c");
        assert_eq!(register(&division.view, "rdx"), "0x0000000000000003");
        assert!(division.events.iter().any(|event| matches!(
            event,
            StepEvent::Division {
                quotient,
                remainder,
                ..
            } if quotient == "0x000000000000000c" && remainder == "0x0000000000000003"
        )));

        let mut zero = session(".text\n.global _start\n_start:\n xor %rbx,%rbx\n divq %rbx\n");
        zero.execute(Command::Step);
        let fault = zero.execute(Command::Step);
        assert!(matches!(
            fault.view.status,
            MachineStatus::Faulted { ref code, .. } if code == "division_by_zero"
        ));
        let reversed = zero.execute(Command::Back);
        assert_eq!(reversed.view.status, MachineStatus::Paused);
        assert_eq!(
            reversed.view.next_text.as_deref().map(str::trim),
            Some("divq %rbx")
        );
    }

    #[test]
    fn inc_and_dec_preserve_the_carry_flag() {
        let mut session = session(
            ".text\n.global _start\n_start:\n xor %rax,%rax\n xor %rbx,%rbx\n sub $1,%rax\n inc %rbx\n dec %rbx\n",
        );
        for _ in 0..4 {
            session.execute(Command::Step);
        }
        assert!(session.view().flags.cf);
        assert_eq!(register(&session.view(), "rbx"), "0x0000000000000001");
        session.execute(Command::Step);
        assert!(session.view().flags.cf);
        assert_eq!(register(&session.view(), "rbx"), "0x0000000000000000");
    }

    #[test]
    fn continue_stops_at_its_safety_limit_inside_a_loop() {
        let mut session = session(".text\n.global _start\n_start:\n jmp _start\n");
        let result = session.execute(Command::Continue { max_steps: 3 });
        assert_eq!(result.steps_executed, 3);
        assert_eq!(result.view.status, MachineStatus::Paused);
        assert_eq!(result.view.history_depth, 3);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E302")
        );
    }
}
