//! GDB-style queries over a session: `print`, `x`, `backtrace`, `info
//! registers`, and breakpoint/`set` argument resolution. They live in the core
//! so the browser console, the TUI, and the native REPL share one meaning and
//! one set of error messages.

use super::Session;
use crate::machine::{CanonicalRegister, RegisterRef};
use crate::program::DATA_BASE;
use crate::{Command, expression};

const MAX_EXAMINE_UNITS: usize = 512;
const MAX_STRING_BYTES: usize = 256;

/// A value produced by a debugger expression, with the width that decides how
/// its signed and binary forms are printed.
struct Value {
    bits: u64,
    width: u8,
}

impl Session {
    /// Implements `print[/FMT] EXPR` and `p/x`, `p/d`, `p/u`, `p/t`, `p/o`,
    /// `p/c`, numbering results like GDB's value history (`$1 = ...`).
    pub fn print_expression(&mut self, format: Option<char>, text: &str) -> Result<String, String> {
        let value = self.evaluate(text)?;
        let rendered = format_value(&value, format.unwrap_or('d'))?;
        self.print_count += 1;
        Ok(format!("${} = {rendered}", self.print_count))
    }

    /// Implements `x/NFU ADDRESS`: N units of size U (b, h, w, g) shown in
    /// format F (x, d, u, t, o, c, s for strings, or i for instructions).
    pub fn examine(&self, specification: &str, text: &str) -> Result<String, String> {
        let (count, format, unit) = parse_examine_spec(specification)?;
        let address = self.evaluate(text)?.bits;
        let bare_symbol = self.program.data_symbols.contains_key(text.trim());
        let unmapped = |address: u64| {
            let mut message = format!("Cannot access memory at address {}", short_hex(address));
            if bare_symbol {
                message.push_str(&format!(
                    "; `{0}` is the value stored at {0}, write `&{0}` for its address",
                    text.trim()
                ));
            }
            message
        };

        if format == 'i' {
            let mut lines = Vec::new();
            for offset in 0..count as u64 {
                let target = address.wrapping_add(offset);
                let index = self
                    .program
                    .instruction_index_for_address(target)
                    .ok_or_else(|| unmapped(target))?;
                let instruction = &self.program.instructions[index];
                let marker = if index == self.machine.pc {
                    "=> "
                } else {
                    "   "
                };
                lines.push(format!(
                    "{marker}{}{}:\t{}\t({}:{})",
                    short_hex(target),
                    self.symbolize(target),
                    instruction.text.trim(),
                    instruction.location.module,
                    instruction.location.line
                ));
            }
            return Ok(lines.join("\n"));
        }

        if format == 's' {
            let mut lines = Vec::new();
            let mut cursor = address;
            for _ in 0..count {
                let mut bytes = Vec::new();
                loop {
                    let byte = self
                        .machine
                        .peek(cursor, 1)
                        .ok_or_else(|| unmapped(cursor))?[0];
                    cursor = cursor.wrapping_add(1);
                    if byte == 0 || bytes.len() == MAX_STRING_BYTES {
                        break;
                    }
                    bytes.push(byte);
                }
                let start = cursor.wrapping_sub(bytes.len() as u64 + 1);
                lines.push(format!(
                    "{}{}:\t\"{}\"",
                    short_hex(start),
                    self.symbolize(start),
                    crate::protocol::escaped_bytes(&bytes)
                ));
            }
            return Ok(lines.join("\n"));
        }

        let size = usize::from(unit);
        let bytes = self
            .machine
            .peek(address, count * size)
            .ok_or_else(|| unmapped(address))?;
        let per_line = match unit {
            1 | 2 => 8,
            4 => 4,
            _ => 2,
        };
        let mut lines = Vec::new();
        for (line_index, line) in bytes.chunks(per_line * size).enumerate() {
            let line_address = address + (line_index * per_line * size) as u64;
            let items = line
                .chunks(size)
                .map(|chunk| {
                    let mut padded = [0_u8; 8];
                    padded[..chunk.len()].copy_from_slice(chunk);
                    let value = Value {
                        bits: u64::from_le_bytes(padded),
                        width: unit * 8,
                    };
                    match format {
                        'x' => format!("0x{:0digits$x}", value.bits, digits = size * 2),
                        't' => format!("{:0digits$b}", value.bits, digits = size * 8),
                        _ => format_value(&value, format).unwrap_or_default(),
                    }
                })
                .collect::<Vec<_>>();
            lines.push(format!(
                "{}{}:\t{}",
                short_hex(line_address),
                self.symbolize(line_address),
                items.join("\t")
            ));
        }
        Ok(lines.join("\n"))
    }

    /// Implements `backtrace`/`bt`: the current location, then each saved
    /// `%rbp` frame and the call site it will return to.
    pub fn backtrace(&self) -> String {
        let view = self.view();
        let mut lines = Vec::new();
        match &view.next_instruction {
            Some(location) => lines.push(format!(
                "#0  {}:{}\t{}",
                location.module,
                location.line,
                view.next_text.as_deref().unwrap_or("").trim()
            )),
            None => lines.push("#0  (the process has halted)".to_string()),
        }
        for frame in &view.stack.frames {
            let returns_to = frame
                .return_location
                .as_ref()
                .map(|location| format!("{}:{}", location.module, location.line))
                .unwrap_or_else(|| frame.return_address.clone());
            lines.push(format!(
                "#{}  in {} (frame %rbp={}) returns to {returns_to}",
                frame.depth + 1,
                frame.function.as_deref().unwrap_or("??"),
                frame.rbp
            ));
        }
        if view.stack.frames.is_empty() {
            lines.push(
                "(no saved %rbp frames; a function's prologue creates one with push %rbp / mov %rsp,%rbp)"
                    .to_string(),
            );
        }
        lines.join("\n")
    }

    /// Implements `info frame` for the innermost `%rbp` frame.
    pub fn frame_info(&self) -> String {
        let view = self.view();
        let mut lines = vec![format!(
            "%rsp = {} (mod 16 = {}), %rbp = {}",
            view.stack.rsp, view.stack.rsp_mod_16, view.stack.rbp
        )];
        match view.stack.frames.first() {
            Some(frame) => {
                lines.push(format!(
                    "frame of {}: saved caller %rbp {} at {}",
                    frame.function.as_deref().unwrap_or("??"),
                    frame.saved_rbp,
                    frame.rbp
                ));
                lines.push(format!(
                    "return address {}{}",
                    frame.return_address,
                    frame
                        .return_location
                        .as_ref()
                        .map(|location| format!(" → {}:{}", location.module, location.line))
                        .unwrap_or_default()
                ));
                lines.push(format!(
                    "%rsp was {}16-byte aligned at the call",
                    if frame.aligned_at_call { "" } else { "NOT " }
                ));
            }
            None => lines.push("no saved %rbp frame is active".to_string()),
        }
        lines.join("\n")
    }

    /// Implements `info registers` in GDB's three-column layout.
    pub fn registers_text(&self) -> String {
        let mut lines = CanonicalRegister::ALL
            .into_iter()
            .map(|register| {
                let value = self.machine.registers.canonical(register);
                format!(
                    "{:<6}{:<20}{}",
                    register.name(),
                    short_hex(value),
                    value as i64
                )
            })
            .collect::<Vec<_>>();
        lines.push(format!(
            "{:<6}{:<20}{}",
            "rip",
            short_hex(self.program.text_address(self.machine.pc)),
            self.view()
                .next_instruction
                .map(|location| format!("{}:{}", location.module, location.line))
                .unwrap_or_default()
        ));
        let flags = self.machine.flags;
        let set = [
            ("CF", flags.cf),
            ("PF", flags.pf),
            ("AF", flags.af),
            ("ZF", flags.zf),
            ("SF", flags.sf),
            ("OF", flags.of),
        ]
        .into_iter()
        .filter_map(|(name, on)| on.then_some(name))
        .collect::<Vec<_>>();
        lines.push(if set.is_empty() {
            "eflags [ ]".to_string()
        } else {
            format!("eflags [ {} ]", set.join(" "))
        });
        lines.join("\n")
    }

    /// Implements `modules`: each linked source module and its line count.
    pub fn modules_text(&self) -> String {
        self.program
            .modules
            .iter()
            .map(|module| format!("{}\t{} lines", module.name, module.source.lines().count()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Resolves `break LINE`, `break MODULE:LINE`, `break LABEL`, or
    /// `break *ADDRESS` to the shared breakpoint command.
    pub fn breakpoint_command(&self, specification: &str) -> Result<Command, String> {
        let specification = specification.trim();
        if specification.is_empty() {
            return Err(
                "usage: break LINE, break MODULE:LINE, break LABEL, or break *ADDRESS".to_string(),
            );
        }
        let to_command = |index: usize| {
            let location = &self.program.instructions[index].location;
            Command::ToggleBreakpoint {
                module: location.module.clone(),
                line: location.line,
            }
        };
        if let Some(address) = specification.strip_prefix('*') {
            let address = self.evaluate(address)?.bits;
            return self
                .program
                .instruction_index_for_address(address)
                .map(to_command)
                .ok_or_else(|| format!("no instruction at address {}", short_hex(address)));
        }
        if let Some(index) = self.program.labels.get(specification) {
            return Ok(to_command(*index));
        }
        let (module, line_text) = match specification.rsplit_once(':') {
            Some((module, line)) => (module.to_string(), line),
            None => (
                self.view()
                    .next_instruction
                    .map(|location| location.module)
                    .or_else(|| {
                        self.program
                            .modules
                            .first()
                            .map(|module| module.name.clone())
                    })
                    .ok_or_else(|| "the program has no source module".to_string())?,
                specification,
            ),
        };
        let line = line_text
            .parse::<usize>()
            .map_err(|_| format!("`{line_text}` is not a source line number or a .text label"))?;
        Ok(Command::ToggleBreakpoint { module, line })
    }

    pub(super) fn register_for_set(&self, name: &str) -> Result<RegisterRef, String> {
        let name = name.trim();
        let bare = name.strip_prefix('$').unwrap_or(name);
        let bare = bare.strip_prefix('%').unwrap_or(bare);
        if matches!(bare, "rip" | "pc" | "eflags") {
            return Err(format!(
                "`set ${bare}` is not supported; change control flow by editing the program"
            ));
        }
        RegisterRef::parse(bare).ok_or_else(|| format!("`{name}` is not a register"))
    }

    pub(super) fn evaluate_for_set(&self, text: &str) -> Result<u64, String> {
        self.evaluate(text).map(|value| value.bits)
    }

    fn evaluate(&self, text: &str) -> Result<Value, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("expected an expression, such as $rax, &msg, or 0x400000".to_string());
        }
        let single_register = text
            .strip_prefix('$')
            .and_then(RegisterRef::parse)
            .map(RegisterRef::width);
        let value = expression::evaluate(text, |token| self.resolve_token(token))?;
        let bits = expression::to_u64(value, text)?;
        let width = single_register.unwrap_or(64);
        Ok(Value {
            bits: bits & width_mask(width),
            width,
        })
    }

    fn resolve_token(&self, token: &str) -> Result<i128, String> {
        if let Some(name) = token.strip_prefix('$') {
            if matches!(name, "rip" | "pc") {
                return Ok(i128::from(self.program.text_address(self.machine.pc)));
            }
            let register =
                RegisterRef::parse(name).ok_or_else(|| format!("`${name}` is not a register"))?;
            return Ok(i128::from(self.machine.registers.read(register)));
        }
        if let Some(name) = token.strip_prefix('&') {
            if let Some(address) = self.program.data_symbols.get(name) {
                return Ok(i128::from(*address));
            }
            if let Some(index) = self.program.labels.get(name) {
                return Ok(i128::from(self.program.text_address(*index)));
            }
            return Err(format!("no symbol `{name}` in the program"));
        }
        if let Some(value) = self.program.constants.get(token) {
            return Ok(i128::from(*value as i64));
        }
        if let Some(address) = self.program.data_symbols.get(token) {
            let width = self
                .program
                .data_symbol_widths
                .get(token)
                .copied()
                .unwrap_or(8);
            let bytes = self
                .machine
                .peek(*address, width)
                .ok_or_else(|| format!("`{token}` is not fully mapped"))?;
            let mut padded = [0_u8; 8];
            padded[..bytes.len()].copy_from_slice(bytes);
            return Ok(i128::from(u64::from_le_bytes(padded)));
        }
        if let Some(index) = self.program.labels.get(token) {
            return Ok(i128::from(self.program.text_address(*index)));
        }
        if let Some(character) = token
            .strip_prefix('\'')
            .and_then(|rest| rest.strip_suffix('\''))
        {
            let value = match character {
                "\\n" => b'\n',
                "\\t" => b'\t',
                "\\0" => 0,
                _ if character.len() == 1 && character.is_ascii() => character.as_bytes()[0],
                _ => return Err(format!("invalid character `{token}`")),
            };
            return Ok(i128::from(value));
        }
        let parsed = if let Some(hex) = token.strip_prefix("0x") {
            u64::from_str_radix(hex, 16)
        } else {
            token.parse::<u64>()
        };
        parsed.map(i128::from).map_err(|_| {
            if RegisterRef::parse(token).is_some() {
                format!("write `${token}` to read the register {token}")
            } else {
                format!("no symbol `{token}` in the program")
            }
        })
    }

    /// Names an address as `<symbol+offset>` when it falls inside data or text.
    fn symbolize(&self, address: u64) -> String {
        if let Some(index) = self.program.instruction_index_for_address(address) {
            if let Some((name, label_index)) = self
                .program
                .labels
                .iter()
                .filter(|(_, label_index)| **label_index <= index)
                .max_by_key(|(_, label_index)| **label_index)
            {
                let offset = index - label_index;
                return if offset == 0 {
                    format!(" <{name}>")
                } else {
                    format!(" <{name}+{offset}>")
                };
            }
            return String::new();
        }
        let data_end = DATA_BASE + self.machine.memory.len() as u64;
        if (DATA_BASE..data_end).contains(&address)
            && let Some((name, symbol_address)) = self
                .program
                .data_symbols
                .iter()
                .filter(|(_, symbol_address)| **symbol_address <= address)
                .max_by_key(|(_, symbol_address)| **symbol_address)
        {
            let offset = address - symbol_address;
            return if offset == 0 {
                format!(" <{name}>")
            } else {
                format!(" <{name}+{offset}>")
            };
        }
        String::new()
    }
}

fn parse_examine_spec(specification: &str) -> Result<(usize, char, u8), String> {
    let digits = specification
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(specification.len());
    let count = if digits == 0 {
        1
    } else {
        specification[..digits]
            .parse::<usize>()
            .map_err(|_| format!("invalid count in `x/{specification}`"))?
    };
    if count == 0 || count > MAX_EXAMINE_UNITS {
        return Err(format!(
            "x can show between 1 and {MAX_EXAMINE_UNITS} units at once"
        ));
    }
    let mut format = None;
    let mut unit = None;
    for letter in specification[digits..].chars() {
        match letter {
            'b' => unit = Some(1),
            'h' => unit = Some(2),
            'w' => unit = Some(4),
            'g' => unit = Some(8),
            'x' | 'd' | 'u' | 't' | 'o' | 'c' | 's' | 'i' => format = Some(letter),
            _ => {
                return Err(format!(
                    "unknown letter `{letter}` in `x/{specification}`; formats are x d u t o c s i and sizes are b h w g"
                ));
            }
        }
    }
    let format = format.unwrap_or('x');
    let unit = unit.unwrap_or(if format == 'c' { 1 } else { 4 });
    Ok((count, format, unit))
}

fn format_value(value: &Value, format: char) -> Result<String, String> {
    let signed = sign_extend(value.bits, value.width);
    Ok(match format {
        'x' => format!("0x{:x}", value.bits),
        'd' => signed.to_string(),
        'u' => value.bits.to_string(),
        't' => format!("{:b}", value.bits),
        'o' => format!("0{:o}", value.bits),
        'c' => {
            let byte = (value.bits & 0xff) as u8;
            format!(
                "{} '{}'",
                byte as i8,
                crate::protocol::escaped_bytes(&[byte])
            )
        }
        _ => {
            return Err(format!(
                "unknown print format `/{format}`; use /x /d /u /t /o or /c"
            ));
        }
    })
}

fn sign_extend(bits: u64, width: u8) -> i64 {
    let shift = 64 - u32::from(width);
    ((bits << shift) as i64) >> shift
}

fn width_mask(width: u8) -> u64 {
    if width >= 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    }
}

fn short_hex(value: u64) -> String {
    format!("0x{value:x}")
}
