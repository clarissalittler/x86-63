use crate::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplInput {
    Command(Command),
    Registers,
    Memory,
    Stack,
    Output,
    Input(String),
    Why,
    Help,
    Quit,
    Empty,
    Unknown(String),
}

pub fn parse_repl_input(input: &str) -> ReplInput {
    let trimmed = input.trim();
    let normalized = trimmed.to_ascii_lowercase();
    if normalized == "input" {
        return ReplInput::Input(String::new());
    }
    if normalized.starts_with("input ") {
        return ReplInput::Input(trimmed[5..].trim_start().to_string());
    }
    match normalized.as_str() {
        "" => ReplInput::Empty,
        "si" | "s" | "step" => ReplInput::Command(Command::Step),
        "ni" | "n" | "next" => ReplInput::Command(Command::Next),
        "back" | "reverse-stepi" | "rs" => ReplInput::Command(Command::Back),
        "r" | "reset" | "start" => ReplInput::Command(Command::Reset),
        "c" | "continue" | "run" => ReplInput::Command(Command::Continue { max_steps: 10_000 }),
        "regs" | "registers" | "info registers" => ReplInput::Registers,
        "mem" | "memory" | "x/data" => ReplInput::Memory,
        "stack" | "x/stack" => ReplInput::Stack,
        "io" | "output" | "stdout" => ReplInput::Output,
        "why" => ReplInput::Why,
        "h" | "help" | "?" => ReplInput::Help,
        "q" | "quit" | "exit" => ReplInput::Quit,
        _ => ReplInput::Unknown(input.trim().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_gdb_and_friendly_aliases() {
        assert_eq!(parse_repl_input("si"), ReplInput::Command(Command::Step));
        assert_eq!(parse_repl_input("step"), ReplInput::Command(Command::Step));
        assert_eq!(parse_repl_input("info registers"), ReplInput::Registers);
    }
}
