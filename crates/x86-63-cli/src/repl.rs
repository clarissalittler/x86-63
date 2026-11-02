use std::io::{self, Write};

use x86_63_core::{Command, MachineView, ProgramView, ReplInput, Session, parse_repl_input};

use crate::display;

pub fn run(mut session: Session) -> Result<(), String> {
    println!("x86-63 CS201 REPL");
    println!(
        "Type `step`/`si`, `next`, `back`, `run`, `break LINE`, `input TEXT`, `regs`, `memory`, `stack`, `output`, `why`, `reset`, `help`, or `quit`."
    );
    let modules = session.program().modules;
    display::print_source_context(&session.view(), &modules);

    let stdin = io::stdin();
    loop {
        print!("(x86-63) ");
        io::stdout().flush().map_err(|error| error.to_string())?;
        let mut line = String::new();
        if stdin
            .read_line(&mut line)
            .map_err(|error| error.to_string())?
            == 0
        {
            println!();
            return Ok(());
        }
        match parse_repl_input(&line) {
            ReplInput::Command(command) => {
                let result = session.execute(command);
                display::print_result(&result);
                display::print_source_context(&result.view, &modules);
            }
            ReplInput::Registers => display::print_registers(&session.view()),
            ReplInput::Memory => display::print_memory(&session.view()),
            ReplInput::Stack => display::print_stack(&session.view()),
            ReplInput::Output => display::print_output(&session.view()),
            ReplInput::Breakpoint(specification) => {
                match breakpoint_command(&specification, &session.view(), &session.program()) {
                    Ok(command) => {
                        let result = session.execute(command);
                        display::print_result(&result);
                        display::print_source_context(&result.view, &modules);
                    }
                    Err(message) => println!("{message}"),
                }
            }
            ReplInput::Breakpoints => print_breakpoints(&session.view()),
            ReplInput::ClearBreakpoints => {
                let result = session.execute(Command::ClearBreakpoints);
                display::print_result(&result);
            }
            ReplInput::Input(text) => {
                let result = session.execute(Command::SubmitInput { text });
                display::print_result(&result);
                display::print_source_context(&result.view, &modules);
            }
            ReplInput::Why => println!("{}", session.last_explanation()),
            ReplInput::Help => println!(
                "step|si: one instruction; next|ni: step over; back: undo; run|c: continue;\n\
                 input TEXT: submit a terminal line; regs: registers/flags; memory: data;\n\
                 stack: active stack slots; output: stdin/stdout/stderr; why: explanation;\n\
                 break LINE|MODULE:LINE|LABEL: toggle source breakpoint; info breakpoints: list;\n\
                 clear breakpoints: remove all breakpoints;\n\
                 reset: restart; quit: leave"
            ),
            ReplInput::Quit => return Ok(()),
            ReplInput::Empty => {}
            ReplInput::Unknown(command) => {
                println!("unknown command `{command}`; type `help`");
            }
        }
    }
}

fn breakpoint_command(
    specification: &str,
    view: &MachineView,
    program: &ProgramView,
) -> Result<Command, String> {
    let specification = specification.trim();
    if specification.is_empty() {
        return Err("usage: break LINE, break MODULE:LINE, or break LABEL".to_string());
    }

    if let Some(index) = program.labels.get(specification)
        && let Some(instruction) = program.instructions.get(*index)
    {
        return Ok(Command::ToggleBreakpoint {
            module: instruction.location.module.clone(),
            line: instruction.location.line,
        });
    }

    let (module, line_text) = if let Some((module, line)) = specification.rsplit_once(':') {
        (module.to_string(), line)
    } else {
        let module = view
            .next_instruction
            .as_ref()
            .map(|location| location.module.clone())
            .or_else(|| program.modules.first().map(|module| module.name.clone()))
            .ok_or_else(|| "the program has no source module".to_string())?;
        (module, specification)
    };
    let line = line_text
        .parse::<usize>()
        .map_err(|_| format!("`{line_text}` is not a source line number"))?;
    Ok(Command::ToggleBreakpoint { module, line })
}

fn print_breakpoints(view: &MachineView) {
    if view.breakpoints.is_empty() {
        println!("no breakpoints set");
        return;
    }
    println!("breakpoints:");
    for location in &view.breakpoints {
        println!("  {}:{}", location.module, location.line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x86_63_core::SourceModule;

    #[test]
    fn resolves_lines_modules_and_labels() {
        let session = Session::from_modules(vec![SourceModule::new(
            "main.s",
            ".text\n.global _start\n_start:\n mov $60,%rax\n syscall\n",
        )])
        .unwrap();
        let view = session.view();
        let program = session.program();
        assert!(matches!(
            breakpoint_command("4", &view, &program),
            Ok(Command::ToggleBreakpoint { module, line }) if module == "main.s" && line == 4
        ));
        assert!(matches!(
            breakpoint_command("_start", &view, &program),
            Ok(Command::ToggleBreakpoint { module, line }) if module == "main.s" && line == 4
        ));
    }
}
