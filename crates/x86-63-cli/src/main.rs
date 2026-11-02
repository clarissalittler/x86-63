mod display;
mod repl;
mod tui;

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use x86_63_core::{Command, MachineStatus, Session, SourceModule, compile};

#[derive(Parser)]
#[command(
    name = "x86-63",
    version,
    about = "Step through the CS201 teaching subset of x86-64"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Mode>,
}

#[derive(Subcommand)]
enum Mode {
    /// Open the full-screen terminal visualizer (the default mode).
    Tui(Input),
    /// Open a line-oriented, GDB-like REPL.
    Repl {
        #[command(flatten)]
        input: Input,
        /// Replay a command file before the prompt, like `gdb -x` (repeatable).
        #[arg(short = 'x', long = "commands", value_name = "SCRIPT")]
        commands: Vec<PathBuf>,
        /// Exit after the command files instead of opening the prompt.
        #[arg(long, requires = "commands")]
        batch: bool,
    },
    /// Run until exit, fault, or the safety limit.
    Run(Input),
    /// Behave like the assembled executable: program output goes to stdout and
    /// stderr, reads take lines from stdin, and the exit status matches.
    Exec(Input),
    /// Parse and validate source without running it.
    Check(Input),
    /// Emit one versioned JSON object per executed instruction, or per line of
    /// a command script with `--commands`.
    Trace {
        #[command(flatten)]
        input: Input,
        /// Replay this command file and emit each line's response instead.
        #[arg(short = 'x', long = "commands", value_name = "SCRIPT")]
        commands: Option<PathBuf>,
    },
    /// List the bundled course examples.
    Examples,
}

#[derive(clap::Args, Clone)]
struct Input {
    /// Assembly modules to load and link together.
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Load a bundled lesson instead of files.
    #[arg(long, value_name = "ID", conflicts_with = "files")]
    example: Option<String>,

    /// Maximum instructions for run/continue.
    #[arg(long, default_value_t = 10_000)]
    max_steps: usize,

    /// Queue one line of stdin before execution (repeatable).
    #[arg(long = "stdin", value_name = "LINE")]
    stdin: Vec<String>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            example: Some("firstadd".to_string()),
            max_steps: 10_000,
            stdin: Vec::new(),
        }
    }
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("x86-63: {message}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command.unwrap_or_else(|| Mode::Tui(Input::default())) {
        Mode::Examples => {
            for lesson in x86_63_course::lessons() {
                println!("{:<12} {}", lesson.id, lesson.title);
                println!("             {}", lesson.summary);
            }
            Ok(())
        }
        Mode::Check(input) => {
            let modules = load_modules(&input)?;
            match compile(modules) {
                Ok(program) => {
                    println!(
                        "ok: {} supported instruction(s), entry at _start",
                        program.view().instructions.len()
                    );
                    Ok(())
                }
                Err(diagnostics) => {
                    display::print_diagnostics(&diagnostics);
                    Err("source did not pass validation".to_string())
                }
            }
        }
        Mode::Run(input) => {
            let mut session = build_session(&input)?;
            let result = session.execute(Command::Continue {
                max_steps: input.max_steps,
            });
            display::print_result(&result);
            display::print_registers(&result.view);
            display::print_memory(&result.view);
            display::print_stack(&result.view);
            display::print_output(&result.view);
            Ok(())
        }
        Mode::Exec(input) => {
            let status = exec(build_session(&input)?, input.max_steps)?;
            std::process::exit(i32::from(status));
        }
        Mode::Trace {
            input,
            commands: Some(script),
        } => {
            let mut session = build_session(&input)?;
            let script = fs::read_to_string(&script)
                .map_err(|error| format!("could not read {}: {error}", script.display()))?;
            for line in repl::script_lines(&script) {
                let response = x86_63_core::run_repl_line(&mut session, line);
                // Written by hand so the response keeps its struct field order,
                // which the native/WASM parity check compares byte for byte.
                println!(
                    "{{\"line\":{},\"response\":{}}}",
                    serde_json::to_string(line).map_err(|error| error.to_string())?,
                    serde_json::to_string(&response).map_err(|error| error.to_string())?
                );
                if response.quit {
                    break;
                }
            }
            Ok(())
        }
        Mode::Trace {
            input,
            commands: None,
        } => {
            let mut session = build_session(&input)?;
            let mut steps = 0;
            while matches!(session.view().status, MachineStatus::Paused) && steps < input.max_steps
            {
                let result = session.execute(Command::Step);
                println!(
                    "{}",
                    serde_json::to_string(&result).map_err(|error| error.to_string())?
                );
                steps += 1;
            }
            if matches!(session.view().status, MachineStatus::Paused) {
                return Err(format!(
                    "trace stopped at the {}-instruction safety limit",
                    input.max_steps
                ));
            }
            Ok(())
        }
        Mode::Repl {
            input,
            commands,
            batch,
        } => repl::run(build_session(&input)?, &commands, batch),
        Mode::Tui(input) => tui::run(build_session(&input)?).map_err(|error| error.to_string()),
    }
}

/// Runs to completion like a native process and returns its shell status.
/// Faults map to the status a shell reports for the matching signal (SIGFPE
/// for division faults, SIGSEGV otherwise) so differential checks can compare.
fn exec(mut session: Session, max_steps: usize) -> Result<u8, String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let (mut written_out, mut written_err) = (0, 0);
    loop {
        let result = session.execute(Command::Continue { max_steps });
        let io_view = &result.view.io;
        stdout
            .write_all(&io_view.stdout_bytes[written_out..])
            .and_then(|()| stdout.flush())
            .map_err(|error| error.to_string())?;
        stderr
            .write_all(&io_view.stderr_bytes[written_err..])
            .map_err(|error| error.to_string())?;
        written_out = io_view.stdout_bytes.len();
        written_err = io_view.stderr_bytes.len();
        match &result.view.status {
            MachineStatus::Exited { shell_status, .. } => return Ok(*shell_status),
            MachineStatus::Faulted { code, message } => {
                eprintln!("x86-63: {code}: {message}");
                return Ok(if code.starts_with("division") {
                    136
                } else {
                    139
                });
            }
            MachineStatus::WaitingForInput { .. } => {
                let mut line = String::new();
                if stdin
                    .lock()
                    .read_line(&mut line)
                    .map_err(|error| error.to_string())?
                    == 0
                {
                    return Err("stdin ended while the program was waiting to read".to_string());
                }
                session.execute(Command::SubmitInput { text: line });
            }
            MachineStatus::Paused => {
                return Err(format!(
                    "stopped at the {max_steps}-instruction safety limit"
                ));
            }
        }
    }
}

fn build_session(input: &Input) -> Result<Session, String> {
    let mut session = Session::from_modules(load_modules(input)?).map_err(|error| {
        display::print_diagnostics(&error.diagnostics);
        error.to_string()
    })?;
    for line in &input.stdin {
        let result = session.execute(Command::SubmitInput { text: line.clone() });
        if !result.diagnostics.is_empty() {
            display::print_diagnostics(&result.diagnostics);
            return Err("could not queue stdin".to_string());
        }
    }
    Ok(session)
}

fn load_modules(input: &Input) -> Result<Vec<SourceModule>, String> {
    if let Some(id) = &input.example {
        let lesson = x86_63_course::lesson(id).ok_or_else(|| {
            format!("unknown example `{id}`; run `x86-63 examples` to list choices")
        })?;
        let mut modules = vec![SourceModule::new(lesson.module_name, lesson.source)];
        modules.extend(
            x86_63_course::support_modules(lesson.id)
                .iter()
                .map(|module| SourceModule::new(module.module_name, module.source)),
        );
        return Ok(modules);
    }
    if input.files.is_empty() {
        let lesson = x86_63_course::lesson("firstadd").expect("bundled firstadd lesson");
        return Ok(vec![SourceModule::new(lesson.module_name, lesson.source)]);
    }
    input
        .files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map(|source| SourceModule::new(path.display().to_string(), source))
                .map_err(|error| format!("could not read {}: {error}", path.display()))
        })
        .collect()
}
