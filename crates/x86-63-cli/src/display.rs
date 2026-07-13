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
        match event {
            StepEvent::RegisterWrite {
                register,
                before,
                after,
                ..
            } => println!("  %{register}: {before} → {after}"),
            StepEvent::EffectiveAddress {
                expression,
                address,
                symbol,
            } => println!(
                "  address {expression} = {address}{}",
                symbol
                    .as_deref()
                    .map(|name| format!(" ({name})"))
                    .unwrap_or_default()
            ),
            StepEvent::MemoryRead {
                address,
                value,
                width,
                symbol,
            } => println!(
                "  read {width} bits from {}{}: {value}",
                symbol.as_deref().unwrap_or(address),
                if symbol.is_some() {
                    format!(" ({address})")
                } else {
                    String::new()
                }
            ),
            StepEvent::MemoryWrite {
                address,
                before,
                after,
                symbol,
                ..
            } => println!(
                "  memory {}{}: {before} → {after}",
                symbol.as_deref().unwrap_or(""),
                if symbol.is_some() {
                    format!(" ({address})")
                } else {
                    address.clone()
                }
            ),
            StepEvent::Compare {
                destination,
                source,
                result,
                ..
            } => println!("  cmp: {destination} − {source} = {result}; result not stored"),
            StepEvent::Branch {
                condition,
                predicate,
                target,
                taken,
            } => println!(
                "  {condition} {target}: {predicate} → {}",
                if *taken { "taken" } else { "not taken" }
            ),
            StepEvent::Output { fd, escaped, .. } => {
                println!("  write fd {fd}: `{escaped}`")
            }
            _ => {}
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

pub fn print_memory(view: &MachineView) {
    if view.memory.bytes.is_empty() {
        println!("  no .data bytes are mapped");
        return;
    }
    for symbol in &view.memory.symbols {
        let name = directive_name(symbol.element_width);
        let count = symbol.size.div_ceil(symbol.element_width);
        println!("  {} @ {}  {name} × {count}", symbol.name, symbol.address);
        let bytes = &view.memory.bytes[symbol.offset..symbol.offset + symbol.size];
        for (index, element) in bytes.chunks(symbol.element_width).enumerate() {
            println!(
                "    [{index:>2}] {:<23} signed {}",
                format_bytes(element),
                little_signed(element)
            );
        }
    }
}

pub fn print_output(view: &MachineView) {
    if view.io.stdout_bytes.is_empty() && view.io.stderr_bytes.is_empty() {
        println!("  no process output yet");
        return;
    }
    if !view.io.stdout_bytes.is_empty() {
        println!("  stdout: `{}`", view.io.stdout_escaped);
    }
    if !view.io.stderr_bytes.is_empty() {
        println!("  stderr: `{}`", view.io.stderr_escaped);
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

fn directive_name(width: usize) -> &'static str {
    match width {
        1 => ".byte",
        2 => ".word",
        4 => ".long",
        8 => ".quad",
        _ => "data",
    }
}

fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn little_signed(bytes: &[u8]) -> i128 {
    let value = bytes
        .iter()
        .enumerate()
        .fold(0_u128, |value, (index, byte)| {
            value | (u128::from(*byte) << (index * 8))
        });
    let bits = bytes.len() * 8;
    if bits > 0 && value & (1_u128 << (bits - 1)) != 0 {
        value as i128 - (1_i128 << bits)
    } else {
        value as i128
    }
}
