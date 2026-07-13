use std::io::{self, Write};

use x86_63_core::{ReplInput, Session, parse_repl_input};

use crate::display;

pub fn run(mut session: Session) -> Result<(), String> {
    println!("x86-63 CS201 REPL");
    println!(
        "Type `step`/`si`, `back`, `run`, `regs`, `memory`, `output`, `why`, `reset`, `help`, or `quit`."
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
            ReplInput::Output => display::print_output(&session.view()),
            ReplInput::Why => println!("{}", session.last_explanation()),
            ReplInput::Help => println!(
                "step|si: one instruction; next|ni: step over; back: undo; run|c: continue;\n\
                 regs: registers/flags; memory: .data; output: stdout/stderr; why: explanation;\n\
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
