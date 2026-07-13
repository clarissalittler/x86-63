use x86_63_core::{CommandResult, Diagnostic, MachineStatus, MachineView, StepEvent};

pub fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        let location = diagnostic
            .location
            .as_ref()
            .map(|location| {
                format!(
                    "{}:{}:{}: ",
                    location.module, location.line, location.column
                )
            })
            .unwrap_or_default();
        eprintln!(
            "{location}{:?}[{}]: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
        if let Some(help) = &diagnostic.help {
            eprintln!("  help: {help}");
        }
    }
}

pub fn print_result(result: &CommandResult) {
    println!("{}", result.explanation);
    for event in &result.events {
        if let StepEvent::RegisterWrite {
            register,
            before,
            after,
            ..
        } = event
        {
            println!("  %{register}: {before} → {after}");
        }
    }
    if !result.diagnostics.is_empty() {
        print_diagnostics(&result.diagnostics);
    }
    match &result.view.status {
        MachineStatus::Exited {
            signed,
            shell_status,
            ..
        } => println!("Program exited with {signed}; shell status = {shell_status}."),
        MachineStatus::Faulted { code, message } => {
            println!("Program faulted ({code}): {message}.")
        }
        MachineStatus::Paused => {}
    }
}

pub fn print_registers(view: &MachineView) {
    for register in &view.registers {
        println!(
            "  {:>3}  {}  signed {}",
            register.name, register.hex, register.signed
        );
    }
    println!(
        "  flags CF={} PF={} AF={} ZF={} SF={} OF={}",
        bit(view.flags.cf),
        bit(view.flags.pf),
        bit(view.flags.af),
        bit(view.flags.zf),
        bit(view.flags.sf),
        bit(view.flags.of)
    );
}

pub fn print_source_context(view: &MachineView, modules: &[x86_63_core::SourceModule]) {
    let Some(location) = &view.next_instruction else {
        return;
    };
    let Some(module) = modules.iter().find(|module| module.name == location.module) else {
        return;
    };
    let start = location.line.saturating_sub(2).max(1);
    let end = (location.line + 1).min(module.source.lines().count());
    for (index, line) in module.source.lines().enumerate() {
        let line_number = index + 1;
        if (start..=end).contains(&line_number) {
            let marker = if line_number == location.line {
                "▶"
            } else {
                " "
            };
            println!("{marker} {line_number:>3}  {line}");
        }
    }
}

fn bit(value: bool) -> u8 {
    u8::from(value)
}
