use std::fmt;

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::machine::{CanonicalRegister, Machine, StepDelta};
use crate::parser::compile;
use crate::program::{Program, ProgramView, SourceModule};
use crate::protocol::{
    Command, CommandResult, MachineView, PROTOCOL_VERSION, RegisterView, StepEvent, hex64,
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
        let machine = Machine::new(program.entry());
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
            Command::Step | Command::Next => self.step(),
            Command::Back => self.back(),
            Command::Continue { max_steps } => self.continue_for(max_steps),
        }
    }

    pub fn view(&self) -> MachineView {
        let next = self.program.instruction(self.machine.pc);
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
            history_depth: self.history.len(),
        }
    }

    pub fn program(&self) -> ProgramView {
        self.program.view()
    }

    pub fn last_explanation(&self) -> &str {
        &self.last_explanation
    }

    fn reset(&mut self) -> CommandResult {
        self.machine = Machine::new(self.program.entry());
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
        self.history.push(executed.delta);
        self.result(1, executed.events, Vec::new())
    }

    fn back(&mut self) -> CommandResult {
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

    fn continue_for(&mut self, max_steps: usize) -> CommandResult {
        let max_steps = max_steps.max(1);
        let mut steps = 0;
        let mut events = Vec::new();
        while self.machine.status.can_step() && steps < max_steps {
            let executed = self.machine.execute(&self.program);
            self.last_explanation = executed.explanation;
            self.history.push(executed.delta);
            events.extend(executed.events);
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
}
