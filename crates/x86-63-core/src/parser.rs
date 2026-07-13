use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;
use crate::machine::{Operand, Operation, RegisterRef};
use crate::program::{Instruction, Program, SourceLocation, SourceModule};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Text,
    Other,
}

pub fn compile(modules: Vec<SourceModule>) -> Result<Program, Vec<Diagnostic>> {
    if modules.is_empty() {
        return Err(vec![Diagnostic::error(
            "E001",
            "no source modules were provided",
            None,
        )]);
    }

    let mut instructions = Vec::new();
    let mut labels = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for module in &modules {
        let mut section = None;
        for (line_index, original) in module.source.lines().enumerate() {
            let line_number = line_index + 1;
            let column = original
                .find(|character: char| !character.is_whitespace())
                .unwrap_or(0)
                + 1;
            let location = SourceLocation {
                module: module.name.clone(),
                line: line_number,
                column,
            };
            let mut code = original.split('#').next().unwrap_or("").trim();
            if code.is_empty() {
                continue;
            }

            while let Some((label, rest)) = take_label(code) {
                if labels
                    .insert(label.to_string(), instructions.len())
                    .is_some()
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "E102",
                            format!("symbol `{label}` is defined more than once"),
                            Some(location.clone()),
                        )
                        .with_help("rename one label or remove the duplicate definition"),
                    );
                }
                code = rest.trim();
                if code.is_empty() {
                    break;
                }
            }
            if code.is_empty() {
                continue;
            }

            if code.starts_with('.') {
                parse_directive(code, &mut section, &location, &mut diagnostics);
                continue;
            }

            if section != Some(Section::Text) {
                diagnostics.push(
                    Diagnostic::error(
                        "E110",
                        "instruction appears outside the .text section",
                        Some(location),
                    )
                    .with_help("put `.section .text` before executable instructions"),
                );
                continue;
            }

            match parse_instruction(code, location.clone()) {
                Ok(operation) => instructions.push(Instruction {
                    operation,
                    location,
                    text: code.to_string(),
                }),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
    }

    let Some(entry) = labels.get("_start").copied() else {
        diagnostics.push(
            Diagnostic::error("E120", "entry label `_start` is not defined", None)
                .with_help("add `_start:` in .text and declare it with `.global _start`"),
        );
        return Err(diagnostics);
    };

    if instructions.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E121",
            "the program contains no supported instructions",
            None,
        ));
    }
    if entry >= instructions.len() {
        diagnostics.push(
            Diagnostic::error("E122", "`_start` does not point at an instruction", None)
                .with_help("place at least one instruction after `_start:`"),
        );
    }

    if diagnostics.is_empty() {
        Ok(Program {
            modules,
            instructions,
            labels,
            entry,
        })
    } else {
        Err(diagnostics)
    }
}

fn take_label(code: &str) -> Option<(&str, &str)> {
    let colon = code.find(':')?;
    let candidate = code[..colon].trim();
    if candidate.is_empty() || candidate.chars().any(char::is_whitespace) || !valid_label(candidate)
    {
        return None;
    }
    Some((candidate, &code[colon + 1..]))
}

fn valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '.'))
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
}

fn parse_directive(
    code: &str,
    section: &mut Option<Section>,
    location: &SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut parts = code.split_whitespace();
    let directive = parts.next().unwrap_or_default();
    match directive {
        ".text" => *section = Some(Section::Text),
        ".section" => match parts.next() {
            Some(".text") => *section = Some(Section::Text),
            Some(_) => *section = Some(Section::Other),
            None => diagnostics.push(
                Diagnostic::error(
                    "E130",
                    "`.section` needs a section name",
                    Some(location.clone()),
                )
                .with_help("for code, write `.section .text`"),
            ),
        },
        ".global" | ".globl" => {
            if parts.next().is_none() {
                diagnostics.push(Diagnostic::error(
                    "E131",
                    format!("`{directive}` needs a symbol name"),
                    Some(location.clone()),
                ));
            }
        }
        _ => diagnostics.push(
            Diagnostic::error(
                "E132",
                format!("directive `{directive}` is not in the Lecture 3 slice yet"),
                Some(location.clone()),
            )
            .with_help("the current slice supports .text/.section .text and .global/.globl"),
        ),
    }
}

fn parse_instruction(code: &str, location: SourceLocation) -> Result<Operation, Diagnostic> {
    let (mnemonic, operands) = code
        .split_once(char::is_whitespace)
        .map(|(mnemonic, operands)| (mnemonic, operands.trim()))
        .unwrap_or((code, ""));
    let mnemonic = mnemonic.to_ascii_lowercase();
    if mnemonic == "syscall" {
        if operands.is_empty() {
            return Ok(Operation::Syscall);
        }
        return Err(Diagnostic::error(
            "E201",
            "`syscall` does not take explicit operands",
            Some(location),
        ));
    }

    let (family, explicit_width) = parse_family(&mnemonic).ok_or_else(|| {
        Diagnostic::error(
            "E202",
            format!("instruction `{mnemonic}` is not in the Lecture 3 slice yet"),
            Some(location.clone()),
        )
        .with_help("currently supported instruction families are mov, add, sub, and syscall")
    })?;

    let Some((source_text, destination_text)) = operands.split_once(',') else {
        return Err(Diagnostic::error(
            "E203",
            format!("`{mnemonic}` needs source and destination operands separated by a comma"),
            Some(location),
        )
        .with_help("AT&T syntax is `instruction source,destination`"));
    };
    if destination_text.contains(',') {
        return Err(Diagnostic::error(
            "E204",
            format!("`{mnemonic}` takes exactly two operands"),
            Some(location),
        ));
    }

    let destination = parse_register(destination_text.trim(), &location, true)?;
    let width = explicit_width.unwrap_or(destination.width());
    if destination.width() != width {
        return Err(Diagnostic::error(
            "E205",
            format!(
                "`{mnemonic}` is {width}-bit, but %{} is {}-bit",
                destination.name(),
                destination.width()
            ),
            Some(location),
        )
        .with_help("use a register name whose width matches the instruction suffix"));
    }
    let source = parse_operand(source_text.trim(), width, &location)?;

    Ok(match family {
        Family::Mov => Operation::Mov {
            source,
            destination,
            width,
        },
        Family::Add => Operation::Add {
            source,
            destination,
            width,
        },
        Family::Sub => Operation::Sub {
            source,
            destination,
            width,
        },
    })
}

#[derive(Clone, Copy)]
enum Family {
    Mov,
    Add,
    Sub,
}

fn parse_family(mnemonic: &str) -> Option<(Family, Option<u8>)> {
    for (name, family) in [
        ("mov", Family::Mov),
        ("add", Family::Add),
        ("sub", Family::Sub),
    ] {
        if mnemonic == name {
            return Some((family, None));
        }
        for (suffix, width) in [('b', 8), ('w', 16), ('l', 32), ('q', 64)] {
            if mnemonic == format!("{name}{suffix}") {
                return Some((family, Some(width)));
            }
        }
    }
    None
}

fn parse_operand(text: &str, width: u8, location: &SourceLocation) -> Result<Operand, Diagnostic> {
    if let Some(immediate) = text.strip_prefix('$') {
        return parse_immediate(immediate)
            .map(|value| Operand::Immediate(value & mask(width)))
            .map_err(|message| Diagnostic::error("E210", message, Some(location.clone())));
    }
    if text.starts_with('%') {
        let register = parse_register(text, location, false)?;
        if register.width() != width {
            return Err(Diagnostic::error(
                "E211",
                format!(
                    "source %{} is {}-bit, but the destination is {width}-bit",
                    register.name(),
                    register.width()
                ),
                Some(location.clone()),
            )
            .with_help("use matching register widths on both sides"));
        }
        return Ok(Operand::Register(register));
    }
    if parse_immediate(text).is_ok() {
        return Err(Diagnostic::error(
            "E212",
            format!("immediate value `{text}` is missing `$`"),
            Some(location.clone()),
        )
        .with_help(format!("write `${text}` for an immediate value")));
    }
    Err(Diagnostic::error(
        "E213",
        format!("unsupported operand `{text}`"),
        Some(location.clone()),
    ))
}

fn parse_register(
    text: &str,
    location: &SourceLocation,
    destination: bool,
) -> Result<RegisterRef, Diagnostic> {
    let Some(name) = text.strip_prefix('%') else {
        if RegisterRef::parse(text).is_some() {
            return Err(Diagnostic::error(
                "E214",
                format!("register `{text}` is missing `%`"),
                Some(location.clone()),
            )
            .with_help(format!("write `%{text}` for a register")));
        }
        let role = if destination { "destination" } else { "source" };
        return Err(Diagnostic::error(
            "E215",
            format!("{role} `{text}` is not a register in this slice"),
            Some(location.clone()),
        ));
    };
    RegisterRef::parse(name).ok_or_else(|| {
        Diagnostic::error(
            "E216",
            format!("unknown or unsupported register `%{name}`"),
            Some(location.clone()),
        )
    })
}

fn parse_immediate(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if text.len() >= 3 && text.starts_with('\'') && text.ends_with('\'') {
        let inner = &text[1..text.len() - 1];
        let value = match inner {
            "\\n" => b'\n',
            "\\t" => b'\t',
            "\\0" => 0,
            _ if inner.chars().count() == 1 => inner.as_bytes()[0],
            _ => return Err(format!("invalid character immediate `{text}`")),
        };
        return Ok(value as u64);
    }

    let negative = text.starts_with('-');
    let unsigned_text = text.strip_prefix('-').unwrap_or(text);
    let parsed = if let Some(hex) = unsigned_text.strip_prefix("0x") {
        u128::from_str_radix(hex, 16)
    } else {
        unsigned_text.parse::<u128>()
    }
    .map_err(|_| format!("invalid immediate value `{text}`"))?;

    if negative {
        if parsed > (1_u128 << 63) {
            return Err(format!("immediate `{text}` is below the 64-bit range"));
        }
        Ok((-(parsed as i128)) as u64)
    } else if parsed <= u64::MAX as u128 {
        Ok(parsed as u64)
    } else {
        Err(format!("immediate `{text}` is above the 64-bit range"))
    }
}

fn mask(width: u8) -> u64 {
    match width {
        64 => u64::MAX,
        32 => u32::MAX as u64,
        16 => u16::MAX as u64,
        8 => u8::MAX as u64,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(source: &str) -> Vec<SourceModule> {
        vec![SourceModule::new("test.s", source)]
    }

    #[test]
    fn parses_a_minimal_program() {
        let program = compile(module(
            ".section .text\n.global _start\n_start:\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(program.instructions.len(), 2);
        assert_eq!(program.entry, 0);
    }

    #[test]
    fn explains_a_missing_immediate_marker() {
        let diagnostics = compile(module(
            ".section .text\n.global _start\n_start:\n mov 60,%rax\n syscall\n",
        ))
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "E212");
        assert!(diagnostics[0].help.as_deref().unwrap().contains("$60"));
    }

    #[test]
    fn checks_suffix_and_register_width() {
        let diagnostics = compile(module(
            ".section .text\n.global _start\n_start:\n movl $60,%rax\n",
        ))
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "E205");
    }
}
