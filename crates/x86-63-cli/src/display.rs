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
            StepEvent::Call {
                target,
                return_address,
                return_location,
                stack_pointer_before,
                aligned_before,
            } => println!(
                "  call {target}: %rsp {stack_pointer_before} was {}; pushed {return_address}{}",
                if *aligned_before {
                    "16-byte aligned"
                } else {
                    "not 16-byte aligned"
                },
                return_location
                    .as_ref()
                    .map(|location| format!(" (return to {}:{})", location.module, location.line))
                    .unwrap_or_default()
            ),
            StepEvent::Division {
                dividend_high,
                dividend_low,
                divisor,
                quotient,
                remainder,
                ..
            } => println!(
                "  div {dividend_high}:{dividend_low} by {divisor}: quotient {quotient}, remainder {remainder}"
            ),
            StepEvent::Return {
                return_address,
                return_location,
            } => println!(
                "  ret: popped {return_address}{}",
                return_location
                    .as_ref()
                    .map(|location| format!(" → {}:{}", location.module, location.line))
                    .unwrap_or_default()
            ),
            StepEvent::StackPush {
                value,
                stack_pointer,
            } => println!("  stack push {value}; %rsp = {stack_pointer}"),
            StepEvent::StackPop {
                value,
                stack_pointer,
            } => println!("  stack pop {value}; %rsp = {stack_pointer}"),
            StepEvent::Output { fd, escaped, .. } => {
                println!("  write fd {fd}: `{escaped}`")
            }
            StepEvent::InputRequested { count, address, .. } => {
                println!("  read blocked: waiting for up to {count} bytes at {address}")
            }
            StepEvent::InputSubmitted { escaped, .. } => {
                println!("  stdin submitted: `{escaped}`")
            }
            StepEvent::InputRead { escaped, .. } => {
                println!("  read consumed: `{escaped}`")
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
        MachineStatus::WaitingForInput { count, .. } => {
            println!("Program is waiting for a stdin line (up to {count} bytes).")
        }
        MachineStatus::Paused => {}
    }
}

pub fn print_memory(view: &MachineView) {
    if view.memory.bytes.is_empty() {
        println!("  no data bytes are mapped");
        return;
    }
    for symbol in &view.memory.symbols {
        let name = directive_name(symbol.element_width);
        let count = symbol.size.div_ceil(symbol.element_width);
        println!(
            "  {} @ {}  {}  {name} × {count}",
            symbol.name, symbol.address, symbol.section
        );
        let bytes = &view.memory.bytes[symbol.offset..symbol.offset + symbol.size];
        let compact_byte_buffer = symbol.element_width == 1 && symbol.size > 32;
        let display_width = if compact_byte_buffer {
            8
        } else {
            symbol.element_width
        };
        for (index, element) in bytes.chunks(display_width).enumerate() {
            if compact_byte_buffer {
                println!(
                    "    [+{:>3}] {:<23} text `{}`",
                    index * display_width,
                    format_bytes(element),
                    escape_bytes(element)
                );
            } else {
                println!(
                    "    [{index:>2}] {:<23} signed {}",
                    format_bytes(element),
                    little_signed(element)
                );
            }
        }
    }
}

pub fn print_output(view: &MachineView) {
    if view.io.stdin_bytes.is_empty()
        && view.io.stdout_bytes.is_empty()
        && view.io.stderr_bytes.is_empty()
    {
        println!("  no process output yet");
        return;
    }
    if !view.io.stdin_bytes.is_empty() {
        println!(
            "  stdin: `{}` ({} of {} byte(s) consumed)",
            view.io.stdin_escaped,
            view.io.stdin_consumed,
            view.io.stdin_bytes.len()
        );
    }
    if !view.io.stdout_bytes.is_empty() {
        println!("  stdout: `{}`", view.io.stdout_escaped);
    }
    if !view.io.stderr_bytes.is_empty() {
        println!("  stderr: `{}`", view.io.stderr_escaped);
    }
}

pub fn print_stack(view: &MachineView) {
    println!(
        "  %rsp {} (mod 16 = {}, {})  %rbp {}",
        view.stack.rsp,
        view.stack.rsp_mod_16,
        if view.stack.aligned_for_call {
            "ready for call"
        } else {
            "not call-aligned"
        },
        view.stack.rbp
    );
    for frame in &view.stack.frames {
        println!(
            "  frame {}  {}  %rbp {}  return {}{}",
            frame.depth,
            frame.function.as_deref().unwrap_or("unknown function"),
            frame.rbp,
            frame.return_address,
            frame
                .return_location
                .as_ref()
                .map(|location| format!(" → {}:{}", location.module, location.line))
                .unwrap_or_default()
        );
    }
    if view.stack.slots.is_empty() {
        println!("  stack is empty at {}", view.stack.top);
        return;
    }
    for slot in &view.stack.slots {
        println!(
            "  {}  {}  signed {}{}",
            slot.address,
            slot.value,
            slot.signed,
            slot.label
                .as_deref()
                .map(|label| format!("  {label}"))
                .unwrap_or_default()
        );
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

fn escape_bytes(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),
            b'\\' => escaped.push_str("\\\\"),
            0 => escaped.push_str("\\0"),
            0x20..=0x7e => escaped.push(char::from(byte)),
            _ => escaped.push_str(&format!("\\x{byte:02x}")),
        }
    }
    escaped
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
