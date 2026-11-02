use std::fs;
use std::io::{self, Write};
use std::path::Path;

use x86_63_core::{ReplInput, Session, parse_repl_input, run_repl_line};

use crate::display;

/// Nested `source` files deeper than this are refused, which also stops a
/// script from sourcing itself forever.
const MAX_SOURCE_DEPTH: usize = 8;

pub fn run(mut session: Session, scripts: &[impl AsRef<Path>], batch: bool) -> Result<(), String> {
    let modules = session.program().modules;
    if !batch {
        println!("x86-63 CS201 REPL");
        println!(
            "Type `si`, `ni`, `back`, `c`, `break LINE`, `input TEXT`, `p/x $rax`, `x/8xb &buff`, `bt`, `why`, `help`, or `quit`."
        );
        display::print_source_context(&session.view(), &modules);
    }

    for script in scripts {
        if run_script(&mut session, script.as_ref(), 0)? {
            return Ok(());
        }
    }
    if batch {
        return Ok(());
    }

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
        if run_line(&mut session, &line, 0)? {
            return Ok(());
        }
    }
}

/// Runs one line, returning true when it asked to quit.
fn run_line(session: &mut Session, line: &str, depth: usize) -> Result<bool, String> {
    let modules = session.program().modules;
    match parse_repl_input(line) {
        ReplInput::Registers => display::print_registers(&session.view()),
        ReplInput::Memory => display::print_memory(&session.view()),
        ReplInput::Stack => display::print_stack(&session.view()),
        ReplInput::Output => display::print_output(&session.view()),
        ReplInput::Source(path) if !path.is_empty() => {
            return run_script(session, Path::new(&path), depth + 1);
        }
        _ => {
            let response = run_repl_line(session, line);
            if response.quit {
                return Ok(true);
            }
            match &response.result {
                Some(result) => {
                    display::print_result(result);
                    display::print_source_context(&result.view, &modules);
                }
                None if !response.output.is_empty() => println!("{}", response.output),
                None => {}
            }
        }
    }
    Ok(false)
}

/// Replays a command file, echoing each line after the prompt as GDB does with
/// `-x`. Blank lines and `#` comments are skipped. Returns true on `quit`.
fn run_script(session: &mut Session, path: &Path, depth: usize) -> Result<bool, String> {
    if depth > MAX_SOURCE_DEPTH {
        return Err(format!(
            "{}: `source` is nested more than {MAX_SOURCE_DEPTH} levels deep",
            path.display()
        ));
    }
    let script = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    for line in script_lines(&script) {
        println!("(x86-63) {line}");
        if run_line(session, line, depth)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn script_lines(script: &str) -> impl Iterator<Item = &str> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}
