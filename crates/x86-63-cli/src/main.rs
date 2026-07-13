mod display;
mod repl;
mod tui;

use std::fs;
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
    Repl(Input),
    /// Run until exit, fault, or the safety limit.
    Run(Input),
    /// Parse and validate source without running it.
    Check(Input),
    /// Emit one versioned JSON object per executed instruction.
    Trace(Input),
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
}

impl Default for Input {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            example: Some("firstadd".to_string()),
            max_steps: 10_000,
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
            Ok(())
        }
        Mode::Trace(input) => {
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
        Mode::Repl(input) => repl::run(build_session(&input)?),
        Mode::Tui(input) => tui::run(build_session(&input)?).map_err(|error| error.to_string()),
    }
}

fn build_session(input: &Input) -> Result<Session, String> {
    Session::from_modules(load_modules(input)?).map_err(|error| {
        display::print_diagnostics(&error.diagnostics);
        error.to_string()
    })
}

fn load_modules(input: &Input) -> Result<Vec<SourceModule>, String> {
    if let Some(id) = &input.example {
        let lesson = x86_63_course::lesson(id).ok_or_else(|| {
            format!("unknown example `{id}`; run `x86-63 examples` to list choices")
        })?;
        return Ok(vec![SourceModule::new(lesson.module_name, lesson.source)]);
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
