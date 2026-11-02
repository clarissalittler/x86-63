use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;
use crate::expression;
use crate::machine::{JumpCondition, MemoryAddress, Operand, Operation, RegisterRef};
use crate::program::{DATA_BASE, Instruction, Program, SourceLocation, SourceModule};

const MAX_DATA_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Text,
    Data,
    Bss,
    Rodata,
    Other,
}

impl Section {
    fn data_name(self) -> Option<&'static str> {
        match self {
            Self::Data => Some(".data"),
            Self::Bss => Some(".bss"),
            Self::Rodata => Some(".rodata"),
            Self::Text | Self::Other => None,
        }
    }

    fn is_data(self) -> bool {
        self.data_name().is_some()
    }
}

#[derive(Clone, Debug)]
struct RawInstruction {
    text: String,
    location: SourceLocation,
}

/// A GNU numeric local label such as `1:`, referenced as `1b` (the nearest
/// definition at or above the reference) or `1f` (the nearest one below).
#[derive(Clone, Debug)]
struct LocalLabel {
    module: String,
    name: String,
    line: usize,
    instruction_index: usize,
}

pub fn compile(modules: Vec<SourceModule>) -> Result<Program, Vec<Diagnostic>> {
    if modules.is_empty() {
        return Err(vec![Diagnostic::error(
            "E001",
            "no source modules were provided",
            None,
        )]);
    }

    let mut raw_instructions = Vec::new();
    let mut labels = BTreeMap::new();
    let mut local_labels = Vec::new();
    let mut data_symbols = BTreeMap::new();
    let mut data_symbol_widths = BTreeMap::new();
    let mut data_symbol_sections = BTreeMap::new();
    let mut constants = BTreeMap::new();
    let mut initial_data = Vec::new();
    let mut diagnostics = Vec::new();

    for module in &modules {
        let mut section = None;
        for (line_index, original) in module.source.lines().enumerate() {
            let location = SourceLocation {
                module: module.name.clone(),
                line: line_index + 1,
                column: original
                    .find(|character: char| !character.is_whitespace())
                    .unwrap_or(0)
                    + 1,
            };
            let mut code = strip_comment(original).trim();
            if code.is_empty() {
                continue;
            }

            loop {
                if let Some((name, rest)) = take_local_label(code) {
                    if section == Some(Section::Text) {
                        local_labels.push(LocalLabel {
                            module: module.name.clone(),
                            name: name.to_string(),
                            line: location.line,
                            instruction_index: raw_instructions.len(),
                        });
                    } else {
                        diagnostics.push(
                            Diagnostic::error(
                                "E104",
                                format!("numeric local label `{name}:` is only supported in .text"),
                                Some(location.clone()),
                            )
                            .with_help("give data a named label such as `values:`"),
                        );
                    }
                    code = rest.trim();
                    if code.is_empty() {
                        break;
                    }
                    continue;
                }
                let Some((label, rest)) = take_label(code) else {
                    break;
                };
                define_label(
                    label,
                    section,
                    raw_instructions.len(),
                    initial_data.len(),
                    &location,
                    &mut labels,
                    &mut data_symbols,
                    &mut data_symbol_sections,
                    &constants,
                    &mut diagnostics,
                );
                code = rest.trim();
                if code.is_empty() {
                    break;
                }
            }
            if code.is_empty() {
                continue;
            }

            if let Some((name, expression)) = take_equate(code) {
                define_equate(
                    name,
                    expression,
                    section,
                    initial_data.len(),
                    &location,
                    &labels,
                    &data_symbols,
                    &mut constants,
                    &mut diagnostics,
                );
                continue;
            }

            if code.starts_with('.') {
                let data_offset = initial_data.len();
                let element_width = directive_element_width(code);
                parse_directive(
                    code,
                    &mut section,
                    &mut initial_data,
                    &location,
                    &mut diagnostics,
                );
                if let Some(element_width) = element_width {
                    let address = DATA_BASE + data_offset as u64;
                    for (name, _) in data_symbols
                        .iter()
                        .filter(|(_, symbol_address)| **symbol_address == address)
                    {
                        data_symbol_widths.insert(name.clone(), element_width);
                    }
                }
                continue;
            }

            if section == Some(Section::Text) {
                raw_instructions.push(RawInstruction {
                    text: code.to_string(),
                    location,
                });
            } else {
                diagnostics.push(
                    Diagnostic::error(
                        "E110",
                        "instruction appears outside the .text section",
                        Some(location),
                    )
                    .with_help("put `.section .text` before executable instructions"),
                );
            }
        }
    }

    let mut instructions = Vec::with_capacity(raw_instructions.len());
    for raw in raw_instructions {
        match parse_instruction(
            &raw.text,
            raw.location.clone(),
            &labels,
            &local_labels,
            &data_symbols,
            &constants,
        ) {
            Ok(operation) => instructions.push(Instruction {
                operation,
                location: raw.location,
                text: raw.text,
            }),
            Err(diagnostic) => diagnostics.push(diagnostic),
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
            data_symbols,
            data_symbol_widths,
            data_symbol_sections,
            constants,
            initial_data,
            entry,
        })
    } else {
        Err(diagnostics)
    }
}

fn directive_element_width(code: &str) -> Option<usize> {
    match code.split_whitespace().next()? {
        ".byte" | ".ascii" | ".asciz" | ".skip" | ".zero" | ".space" => Some(1),
        ".word" => Some(2),
        ".long" => Some(4),
        ".quad" => Some(8),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn define_label(
    label: &str,
    section: Option<Section>,
    instruction_index: usize,
    data_offset: usize,
    location: &SourceLocation,
    labels: &mut BTreeMap<String, usize>,
    data_symbols: &mut BTreeMap<String, u64>,
    data_symbol_sections: &mut BTreeMap<String, String>,
    constants: &BTreeMap<String, u64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if labels.contains_key(label)
        || data_symbols.contains_key(label)
        || constants.contains_key(label)
    {
        diagnostics.push(
            Diagnostic::error(
                "E102",
                format!("symbol `{label}` is defined more than once"),
                Some(location.clone()),
            )
            .with_help("rename one label or remove the duplicate definition"),
        );
        return;
    }

    match section {
        Some(Section::Text) => {
            labels.insert(label.to_string(), instruction_index);
        }
        Some(section) if section.is_data() => {
            data_symbols.insert(label.to_string(), DATA_BASE + data_offset as u64);
            data_symbol_sections.insert(
                label.to_string(),
                section.data_name().expect("data section").to_string(),
            );
        }
        _ => diagnostics.push(
            Diagnostic::error(
                "E103",
                format!("symbol `{label}` is defined outside a supported section"),
                Some(location.clone()),
            )
            .with_help("place code labels in .text and data labels in .data, .bss, or .rodata"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn define_equate(
    name: &str,
    expression: &str,
    section: Option<Section>,
    data_offset: usize,
    location: &SourceLocation,
    labels: &BTreeMap<String, usize>,
    data_symbols: &BTreeMap<String, u64>,
    constants: &mut BTreeMap<String, u64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if labels.contains_key(name) || data_symbols.contains_key(name) || constants.contains_key(name)
    {
        diagnostics.push(Diagnostic::error(
            "E102",
            format!("symbol `{name}` is defined more than once"),
            Some(location.clone()),
        ));
        return;
    }

    let current = section
        .is_some_and(Section::is_data)
        .then_some(DATA_BASE + data_offset as u64);
    match parse_equate_expression(expression, current, data_symbols, constants) {
        Ok(value) => {
            constants.insert(name.to_string(), value);
        }
        Err(message) => diagnostics.push(
            Diagnostic::error("E140", message, Some(location.clone()))
                .with_help("equates support integers, symbols, `.`, parentheses, and + - * / %"),
        ),
    }
}

fn take_equate(code: &str) -> Option<(&str, &str)> {
    let (name, expression) = code.split_once('=')?;
    let name = name.trim();
    valid_label(name).then_some((name, expression.trim()))
}

fn parse_equate_expression(
    expression: &str,
    current: Option<u64>,
    data_symbols: &BTreeMap<String, u64>,
    constants: &BTreeMap<String, u64>,
) -> Result<u64, String> {
    evaluate_expression(expression, current, data_symbols, constants)
}

/// Evaluates the GNU-style integer expressions the course uses in equates and
/// immediates. The result keeps the same 64-bit two's-complement range as a
/// literal immediate.
fn evaluate_expression(
    expression: &str,
    current: Option<u64>,
    data_symbols: &BTreeMap<String, u64>,
    constants: &BTreeMap<String, u64>,
) -> Result<u64, String> {
    let text = expression.trim();
    if let Ok(value) = resolve_value(text, data_symbols, constants) {
        return Ok(value);
    }
    let value = expression::evaluate(text, |token| {
        if token == "." {
            return current
                .map(i128::from)
                .ok_or_else(|| "`.` is only available while laying out .data".to_string());
        }
        let value = resolve_value(token, data_symbols, constants)?;
        // Literals keep their written sign; symbols are addresses, and a
        // stored two's-complement constant reads back as signed.
        Ok(if constants.contains_key(token) {
            i128::from(value as i64)
        } else {
            i128::from(value)
        })
    })?;
    expression::to_u64(value, text)
}

fn take_local_label(code: &str) -> Option<(&str, &str)> {
    let colon = code.find(':')?;
    let candidate = code[..colon].trim();
    (!candidate.is_empty()
        && candidate
            .chars()
            .all(|character| character.is_ascii_digit()))
    .then_some((candidate, &code[colon + 1..]))
}

fn resolve_local_label(
    reference: &str,
    location: &SourceLocation,
    local_labels: &[LocalLabel],
) -> Option<Result<usize, String>> {
    let (name, direction) = reference.split_at(reference.len().checked_sub(1)?);
    if name.is_empty() || !name.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let mut candidates = local_labels
        .iter()
        .filter(|label| label.module == location.module && label.name == name);
    let found = match direction {
        "b" => candidates.rfind(|label| label.line <= location.line),
        "f" => candidates.find(|label| label.line > location.line),
        _ => return None,
    };
    Some(found.map(|label| label.instruction_index).ok_or_else(|| {
        format!(
            "no local label `{name}:` {} this line",
            if direction == "b" { "above" } else { "below" }
        )
    }))
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

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_directive(
    code: &str,
    section: &mut Option<Section>,
    data: &mut Vec<u8>,
    location: &SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (directive, arguments) = code
        .split_once(char::is_whitespace)
        .map(|(directive, arguments)| (directive, arguments.trim()))
        .unwrap_or((code, ""));
    match directive {
        ".text" => *section = Some(Section::Text),
        ".data" => *section = Some(Section::Data),
        ".bss" => *section = Some(Section::Bss),
        ".rodata" => *section = Some(Section::Rodata),
        ".section" => match arguments.split_whitespace().next() {
            Some(".text") => *section = Some(Section::Text),
            Some(".data") => *section = Some(Section::Data),
            Some(".bss") => *section = Some(Section::Bss),
            Some(".rodata") => *section = Some(Section::Rodata),
            Some(name) => {
                *section = Some(Section::Other);
                diagnostics.push(
                    Diagnostic::error(
                        "E133",
                        format!("section `{name}` is not in the CS201 teaching subset"),
                        Some(location.clone()),
                    )
                    .with_help("currently supported sections are .text, .data, .bss, and .rodata"),
                );
            }
            None => diagnostics.push(
                Diagnostic::error(
                    "E130",
                    "`.section` needs a section name",
                    Some(location.clone()),
                )
                .with_help("write `.section .text`, `.section .data`, or `.section .bss`"),
            ),
        },
        ".global" | ".globl" | ".extern" => {
            if arguments.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "E131",
                    format!("`{directive}` needs a symbol name"),
                    Some(location.clone()),
                ));
            }
        }
        ".byte" => parse_integer_data(arguments, 1, *section, data, location, diagnostics),
        ".word" => parse_integer_data(arguments, 2, *section, data, location, diagnostics),
        ".long" => parse_integer_data(arguments, 4, *section, data, location, diagnostics),
        ".quad" => parse_integer_data(arguments, 8, *section, data, location, diagnostics),
        ".ascii" | ".asciz" => {
            if !section.is_some_and(Section::is_data) || *section == Some(Section::Bss) {
                diagnostics.push(Diagnostic::error(
                    "E134",
                    format!("`{directive}` needs an initialized data section"),
                    Some(location.clone()),
                ));
                return;
            }
            match parse_string_literal(arguments) {
                Ok(mut bytes) => {
                    data.append(&mut bytes);
                    if directive == ".asciz" {
                        data.push(0);
                    }
                }
                Err(message) => diagnostics.push(Diagnostic::error(
                    "E142",
                    message,
                    Some(location.clone()),
                )),
            }
        }
        ".skip" | ".zero" | ".space" => {
            if *section != Some(Section::Bss) {
                diagnostics.push(
                    Diagnostic::error(
                        "E134",
                        format!("`{directive}` appears outside .bss"),
                        Some(location.clone()),
                    )
                    .with_help("put zero-initialized storage after `.section .bss`"),
                );
                return;
            }
            match parse_storage_size(arguments, data.len()) {
                Ok(new_len) => data.resize(new_len, 0),
                Err(message) => diagnostics.push(Diagnostic::error(
                    "E143",
                    message,
                    Some(location.clone()),
                )),
            }
        }
        _ => diagnostics.push(
            Diagnostic::error(
                "E132",
                format!("directive `{directive}` is not in the CS201 teaching subset"),
                Some(location.clone()),
            )
            .with_help(
                "supported directives include .text/.data/.bss/.rodata, .global/.globl/.extern, integer/string data, and .skip/.zero",
            ),
        ),
    }
}

fn parse_storage_size(text: &str, current_size: usize) -> Result<usize, String> {
    let text = text.trim();
    if text.starts_with('-') {
        return Err(format!("storage size `{text}` must not be negative"));
    }
    let size = usize::try_from(parse_immediate(text)?)
        .map_err(|_| format!("storage size `{text}` does not fit usize"))?;
    let new_size = current_size
        .checked_add(size)
        .ok_or_else(|| format!("storage size `{text}` overflows the data region"))?;
    if new_size > MAX_DATA_BYTES {
        return Err(format!(
            "data region would be {new_size} bytes; this teaching machine limits it to {MAX_DATA_BYTES} bytes"
        ));
    }
    Ok(new_size)
}

fn parse_integer_data(
    arguments: &str,
    width: usize,
    section: Option<Section>,
    data: &mut Vec<u8>,
    location: &SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !section.is_some_and(Section::is_data) || section == Some(Section::Bss) {
        diagnostics.push(Diagnostic::error(
            "E134",
            "initialized data directive needs .data or .rodata",
            Some(location.clone()),
        ));
        return;
    }
    if arguments.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E141",
            "data directive needs at least one value",
            Some(location.clone()),
        ));
        return;
    }
    for argument in arguments.split(',') {
        match parse_immediate(argument.trim()) {
            Ok(value) => data.extend_from_slice(&value.to_le_bytes()[..width]),
            Err(message) => {
                diagnostics.push(Diagnostic::error("E141", message, Some(location.clone())))
            }
        }
    }
}

fn parse_string_literal(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim();
    let Some(inner) = text
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err("string data must be enclosed in double quotes".to_string());
    };
    let mut bytes = Vec::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            let mut encoded = [0; 4];
            bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            continue;
        }
        let escaped = characters
            .next()
            .ok_or_else(|| "string ends with an incomplete escape".to_string())?;
        match escaped {
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            '0' => bytes.push(0),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            other => return Err(format!("unsupported string escape `\\{other}`")),
        }
    }
    Ok(bytes)
}

fn parse_instruction(
    code: &str,
    location: SourceLocation,
    labels: &BTreeMap<String, usize>,
    local_labels: &[LocalLabel],
    data_symbols: &BTreeMap<String, u64>,
    constants: &BTreeMap<String, u64>,
) -> Result<Operation, Diagnostic> {
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

    if matches!(mnemonic.as_str(), "ret" | "retq") {
        if operands.is_empty() {
            return Ok(Operation::Ret);
        }
        return Err(Diagnostic::error(
            "E224",
            "`ret` does not take an explicit operand",
            Some(location),
        ));
    }

    if mnemonic == "leave" {
        if operands.is_empty() {
            return Ok(Operation::Leave);
        }
        return Err(Diagnostic::error(
            "E224",
            "`leave` does not take an explicit operand",
            Some(location),
        ));
    }

    if matches!(mnemonic.as_str(), "call" | "callq") {
        let target_label = one_operand(operands, &mnemonic, &location)?;
        let target = labels.get(target_label).copied().ok_or_else(|| {
            Diagnostic::error(
                "E219",
                format!("call target `{target_label}` is not a .text label"),
                Some(location.clone()),
            )
            .with_help("define the function with a label in the .text section")
        })?;
        return Ok(Operation::Call {
            target,
            target_label: target_label.to_string(),
        });
    }

    if matches!(mnemonic.as_str(), "push" | "pushq") {
        let operand_text = one_operand(operands, &mnemonic, &location)?;
        let source = parse_operand(operand_text, data_symbols, constants, &location, false)?;
        validate_operand_width(&source, 64, &mnemonic, &location)?;
        return Ok(Operation::Push { source });
    }

    if matches!(mnemonic.as_str(), "pop" | "popq") {
        let operand_text = one_operand(operands, &mnemonic, &location)?;
        let destination = parse_operand(operand_text, data_symbols, constants, &location, true)?;
        if matches!(destination, Operand::Immediate(_)) {
            return Err(Diagnostic::error(
                "E215",
                "pop destination cannot be an immediate value",
                Some(location),
            ));
        }
        validate_operand_width(&destination, 64, &mnemonic, &location)?;
        return Ok(Operation::Pop { destination });
    }

    if matches!(mnemonic.as_str(), "movzbl" | "movzbq") {
        let (source_text, destination_text) = split_operands(operands).map_err(|message| {
            Diagnostic::error(
                "E203",
                format!("`{mnemonic}` {message}"),
                Some(location.clone()),
            )
        })?;
        let source = parse_operand(source_text, data_symbols, constants, &location, false)?;
        let destination =
            parse_operand(destination_text, data_symbols, constants, &location, true)?;
        let Operand::Register(destination) = destination else {
            return Err(Diagnostic::error(
                "E221",
                "zero-extension destination must be a register",
                Some(location),
            ));
        };
        let destination_width = if mnemonic == "movzbl" { 32 } else { 64 };
        validate_operand_width(&source, 8, &mnemonic, &location)?;
        ensure_register_width(destination, destination_width, &mnemonic, &location)?;
        return Ok(Operation::MovZeroExtend {
            source,
            destination,
            source_width: 8,
            destination_width,
        });
    }

    if matches!(mnemonic.as_str(), "cqo" | "cqto" | "cltq" | "cdqe") {
        if !operands.is_empty() {
            return Err(Diagnostic::error(
                "E224",
                format!("`{mnemonic}` does not take an explicit operand"),
                Some(location),
            )
            .with_help("it always reads %rax/%eax and writes %rdx or %rax"));
        }
        let width = if matches!(mnemonic.as_str(), "cqo" | "cqto") {
            128
        } else {
            64
        };
        return Ok(Operation::SignExtendAccumulator { width });
    }

    if matches!(mnemonic.as_str(), "div" | "divq" | "idiv" | "idivq") {
        let signed = mnemonic.starts_with('i');
        let operand_text = one_operand(operands, &mnemonic, &location)?;
        let source = parse_operand(operand_text, data_symbols, constants, &location, false)?;
        if matches!(source, Operand::Immediate(_)) {
            return Err(Diagnostic::error(
                "E215",
                format!("{mnemonic} divisor must be a register or memory operand"),
                Some(location),
            )
            .with_help("move the divisor into a register such as %rbx first"));
        }
        let width = if mnemonic.ends_with('q') {
            64
        } else {
            operand_register(&source)
                .map(RegisterRef::width)
                .ok_or_else(|| {
                    Diagnostic::error(
                        "E206",
                        format!("cannot infer the width of `{mnemonic}` from a memory operand"),
                        Some(location.clone()),
                    )
                    .with_help(format!("add a suffix, such as `{mnemonic}q`"))
                })?
        };
        if width != 64 {
            return Err(Diagnostic::error(
                "E202",
                format!("only the 64-bit form of `{mnemonic}` is in the teaching subset"),
                Some(location),
            )
            .with_help("use a 64-bit divisor register, which divides %rdx:%rax"));
        }
        validate_operand_width(&source, 64, &mnemonic, &location)?;
        return Ok(Operation::Div {
            source,
            width: 64,
            signed,
        });
    }

    if let Some((family, explicit_width)) = parse_unary_family(&mnemonic) {
        let operand_text = one_operand(operands, &mnemonic, &location)?;
        let destination = parse_operand(operand_text, data_symbols, constants, &location, true)?;
        if matches!(destination, Operand::Immediate(_)) {
            return Err(Diagnostic::error(
                "E215",
                format!("{mnemonic} destination cannot be an immediate value"),
                Some(location),
            ));
        }
        let width = explicit_width
            .or_else(|| operand_register(&destination).map(RegisterRef::width))
            .ok_or_else(|| {
                Diagnostic::error(
                    "E206",
                    format!("cannot infer the width of `{mnemonic}` from a memory operand"),
                    Some(location.clone()),
                )
                .with_help(format!("add a suffix, such as `{mnemonic}q`"))
            })?;
        validate_operand_width(&destination, width, &mnemonic, &location)?;
        return Ok(match family {
            UnaryFamily::Inc => Operation::Inc { destination, width },
            UnaryFamily::Dec => Operation::Dec { destination, width },
            UnaryFamily::Neg => Operation::Neg { destination, width },
        });
    }

    if let Some(condition) = parse_jump(&mnemonic) {
        if operands.is_empty() || operands.contains(',') {
            return Err(Diagnostic::error(
                "E218",
                format!("`{mnemonic}` needs exactly one label operand"),
                Some(location),
            ));
        }
        if let Some(resolved) = resolve_local_label(operands, &location, local_labels) {
            let target = resolved.map_err(|message| {
                Diagnostic::error("E219", message, Some(location.clone())).with_help(
                    "`Nb` jumps back to the nearest `N:` above; `Nf` jumps forward to the nearest `N:` below",
                )
            })?;
            return Ok(Operation::Jump {
                condition,
                mnemonic: mnemonic.clone(),
                target,
                target_label: operands.to_string(),
            });
        }
        let target = labels.get(operands).copied().ok_or_else(|| {
            if data_symbols.contains_key(operands) {
                return Diagnostic::error(
                    "E219",
                    format!("jump target `{operands}` is data, not code"),
                    Some(location.clone()),
                )
                .with_help(
                    "GNU as accepts this, but the CPU would try to execute the data bytes and crash; jump to a .text label instead",
                );
            }
            Diagnostic::error(
                "E219",
                format!("jump target `{operands}` is not a .text label"),
                Some(location.clone()),
            )
            .with_help("define the target with `label:` in the .text section")
        })?;
        return Ok(Operation::Jump {
            condition,
            mnemonic: mnemonic.clone(),
            target,
            target_label: operands.to_string(),
        });
    }

    let (family, explicit_width) = parse_family(&mnemonic).ok_or_else(|| {
        Diagnostic::error(
            "E202",
            format!("instruction `{mnemonic}` is not in the CS201 teaching subset"),
            Some(location.clone()),
        )
        .with_help(
            "supported families include data movement, arithmetic, flags, branches, calls, stack frames, div/idiv, cqo, and syscall",
        )
    })?;

    let (source_text, destination_text) = split_operands(operands).map_err(|message| {
        Diagnostic::error(
            "E203",
            format!("`{mnemonic}` {message}"),
            Some(location.clone()),
        )
        .with_help("AT&T syntax is `instruction source,destination`")
    })?;
    let source = parse_operand(source_text, data_symbols, constants, &location, false)?;
    let destination = parse_operand(destination_text, data_symbols, constants, &location, true)?;

    if family == Family::Lea {
        let Operand::Memory(source) = source else {
            return Err(Diagnostic::error(
                "E220",
                "`lea` needs an address expression as its source",
                Some(location),
            )
            .with_help("for example: `lea num(%rip),%rbx`"));
        };
        let Operand::Register(destination) = destination else {
            return Err(Diagnostic::error(
                "E221",
                "`lea` destination must be a register",
                Some(location),
            ));
        };
        let width = explicit_width.unwrap_or(destination.width());
        ensure_register_width(destination, width, &mnemonic, &location)?;
        return Ok(Operation::Lea {
            source,
            destination,
            width,
        });
    }

    if matches!(destination, Operand::Immediate(_)) {
        return Err(Diagnostic::error(
            "E215",
            "destination cannot be an immediate value",
            Some(location),
        ));
    }
    let width = infer_width(explicit_width, &source, &destination).ok_or_else(|| {
        Diagnostic::error(
            "E206",
            format!("cannot infer the width of `{mnemonic}` from two non-register operands"),
            Some(location.clone()),
        )
        .with_help("add b/w/l/q to the mnemonic, such as `addq $10,(%rbx)`")
    })?;
    validate_operand_width(&source, width, &mnemonic, &location)?;
    validate_operand_width(&destination, width, &mnemonic, &location)?;
    if matches!(source, Operand::Memory(_)) && matches!(destination, Operand::Memory(_)) {
        return Err(Diagnostic::error(
            "E217",
            "x86 does not allow two explicit memory operands here",
            Some(location),
        )
        .with_help("move one value through a register first"));
    }
    if family == Family::Imul && !matches!(destination, Operand::Register(_)) {
        return Err(Diagnostic::error(
            "E221",
            "two-operand imul requires a register destination",
            Some(location),
        ));
    }

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
        Family::Cmp => Operation::Cmp {
            source,
            destination,
            width,
        },
        Family::Xor => Operation::Xor {
            source,
            destination,
            width,
        },
        Family::Imul => Operation::Imul {
            source,
            destination,
            width,
        },
        Family::Test => Operation::Test {
            source,
            destination,
            width,
        },
        Family::Lea => unreachable!("handled above"),
    })
}

fn one_operand<'a>(
    operands: &'a str,
    mnemonic: &str,
    location: &SourceLocation,
) -> Result<&'a str, Diagnostic> {
    if operands.is_empty() || operands.contains(',') {
        return Err(Diagnostic::error(
            "E224",
            format!("`{mnemonic}` needs exactly one operand"),
            Some(location.clone()),
        ));
    }
    Ok(operands.trim())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Mov,
    Add,
    Sub,
    Lea,
    Cmp,
    Xor,
    Imul,
    Test,
}

fn parse_family(mnemonic: &str) -> Option<(Family, Option<u8>)> {
    if mnemonic == "movabsq" {
        return Some((Family::Mov, Some(64)));
    }
    for (name, family) in [
        ("mov", Family::Mov),
        ("add", Family::Add),
        ("sub", Family::Sub),
        ("lea", Family::Lea),
        ("cmp", Family::Cmp),
        ("xor", Family::Xor),
        ("imul", Family::Imul),
        ("test", Family::Test),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum UnaryFamily {
    Inc,
    Dec,
    Neg,
}

fn parse_unary_family(mnemonic: &str) -> Option<(UnaryFamily, Option<u8>)> {
    for (name, family) in [
        ("inc", UnaryFamily::Inc),
        ("dec", UnaryFamily::Dec),
        ("neg", UnaryFamily::Neg),
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

fn parse_jump(mnemonic: &str) -> Option<JumpCondition> {
    Some(match mnemonic {
        "jmp" => JumpCondition::Always,
        "je" | "jz" => JumpCondition::Equal,
        "jne" | "jnz" => JumpCondition::NotEqual,
        "jl" | "jnge" => JumpCondition::Less,
        "jle" | "jng" => JumpCondition::LessOrEqual,
        "jg" | "jnle" => JumpCondition::Greater,
        "jge" | "jnl" => JumpCondition::GreaterOrEqual,
        "jb" | "jc" | "jnae" => JumpCondition::Below,
        "jbe" | "jna" => JumpCondition::BelowOrEqual,
        "ja" | "jnbe" => JumpCondition::Above,
        "jae" | "jnc" | "jnb" => JumpCondition::AboveOrEqual,
        "jo" => JumpCondition::Overflow,
        "jno" => JumpCondition::NoOverflow,
        "js" => JumpCondition::Sign,
        "jns" => JumpCondition::NoSign,
        "jp" | "jpe" => JumpCondition::Parity,
        "jnp" | "jpo" => JumpCondition::NoParity,
        _ => return None,
    })
}

fn split_operands(text: &str) -> Result<(&str, &str), &'static str> {
    let mut depth = 0_u8;
    let mut separator = None;
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                if separator.is_some() {
                    return Err("takes exactly two operands");
                }
                separator = Some(index);
            }
            _ => {}
        }
    }
    let Some(index) = separator else {
        return Err("needs source and destination operands separated by a comma");
    };
    let source = text[..index].trim();
    let destination = text[index + 1..].trim();
    if source.is_empty() || destination.is_empty() {
        return Err("needs both a source and a destination operand");
    }
    Ok((source, destination))
}

fn parse_operand(
    text: &str,
    data_symbols: &BTreeMap<String, u64>,
    constants: &BTreeMap<String, u64>,
    location: &SourceLocation,
    destination: bool,
) -> Result<Operand, Diagnostic> {
    let text = text.trim();
    if let Some(immediate) = text.strip_prefix('$') {
        return evaluate_expression(immediate, None, data_symbols, constants)
            .map(Operand::Immediate)
            .map_err(|message| Diagnostic::error("E210", message, Some(location.clone())));
    }
    if text.starts_with('%') {
        return parse_register(text, location, destination).map(Operand::Register);
    }
    if RegisterRef::parse(text).is_some() {
        return Err(Diagnostic::error(
            "E214",
            format!("register `{text}` is missing `%`"),
            Some(location.clone()),
        )
        .with_help(format!("write `%{text}` for a register")));
    }
    if parse_immediate(text).is_ok() {
        return Err(Diagnostic::error(
            "E212",
            format!("immediate value `{text}` is missing `$`"),
            Some(location.clone()),
        )
        .with_help(format!("write `${text}` for an immediate value")));
    }
    parse_memory(text, data_symbols, location).map(Operand::Memory)
}

fn parse_memory(
    text: &str,
    data_symbols: &BTreeMap<String, u64>,
    location: &SourceLocation,
) -> Result<MemoryAddress, Diagnostic> {
    let (displacement_text, components) = if let Some(open) = text.find('(') {
        if !text.ends_with(')') {
            return Err(address_error(
                "memory operand has an unmatched parenthesis",
                location,
            ));
        }
        (&text[..open], Some(&text[open + 1..text.len() - 1]))
    } else {
        (text, None)
    };

    let (symbol, symbol_address, displacement) = if displacement_text.trim().is_empty() {
        (None, None, 0)
    } else if let Ok(value) = parse_displacement(displacement_text.trim()) {
        (None, None, value)
    } else {
        let name = displacement_text.trim();
        let Some(address) = data_symbols.get(name).copied() else {
            return Err(
                address_error(format!("unknown data symbol `{name}`"), location)
                    .with_help("define the symbol with a label in .data"),
            );
        };
        (Some(name.to_string()), Some(address), 0)
    };

    let mut base = None;
    let mut index = None;
    let mut scale = 1;
    let mut rip_relative = false;
    if let Some(components) = components {
        let parts = components.split(',').map(str::trim).collect::<Vec<_>>();
        if parts.len() > 3 {
            return Err(address_error(
                "addressing mode has more than base, index, and scale",
                location,
            ));
        }
        if let Some(base_text) = parts.first().filter(|part| !part.is_empty()) {
            if *base_text == "%rip" {
                rip_relative = true;
            } else {
                base = Some(parse_address_register(base_text, location)?);
            }
        }
        if let Some(index_text) = parts.get(1).filter(|part| !part.is_empty()) {
            if *index_text == "%rip" {
                return Err(address_error("%rip cannot be an index register", location));
            }
            index = Some(parse_address_register(index_text, location)?);
        }
        if let Some(scale_text) = parts.get(2).filter(|part| !part.is_empty()) {
            scale = scale_text
                .parse::<u8>()
                .map_err(|_| address_error(format!("invalid scale `{scale_text}`"), location))?;
            if !matches!(scale, 1 | 2 | 4 | 8) {
                return Err(address_error("scale must be 1, 2, 4, or 8", location));
            }
            if index.is_none() {
                return Err(address_error(
                    "a scale requires an index register",
                    location,
                ));
            }
        }
    } else if symbol.is_none() {
        return Err(address_error(
            format!("unsupported operand `{text}`"),
            location,
        ));
    }

    if rip_relative && symbol.is_none() {
        return Err(address_error(
            "this slice requires a symbol in RIP-relative addressing",
            location,
        )
        .with_help("for example: `num(%rip)`"));
    }

    Ok(MemoryAddress {
        text: text.to_string(),
        symbol,
        symbol_address,
        displacement,
        base,
        index,
        scale,
        rip_relative,
    })
}

fn parse_address_register(
    text: &str,
    location: &SourceLocation,
) -> Result<RegisterRef, Diagnostic> {
    let register = parse_register(text, location, false)?;
    if register.width() != 64 {
        return Err(address_error(
            format!("address register %{} must be 64-bit", register.name()),
            location,
        ));
    }
    Ok(register)
}

fn address_error(message: impl Into<String>, location: &SourceLocation) -> Diagnostic {
    Diagnostic::error("E223", message, Some(location.clone()))
}

fn parse_displacement(text: &str) -> Result<i64, String> {
    if let Some(hex) = text.strip_prefix("0x") {
        return i64::from_str_radix(hex, 16)
            .map_err(|_| format!("invalid address displacement `{text}`"));
    }
    text.parse::<i64>()
        .map_err(|_| format!("invalid address displacement `{text}`"))
}

fn parse_register(
    text: &str,
    location: &SourceLocation,
    destination: bool,
) -> Result<RegisterRef, Diagnostic> {
    let Some(name) = text.strip_prefix('%') else {
        let role = if destination { "destination" } else { "source" };
        return Err(Diagnostic::error(
            "E215",
            format!("{role} `{text}` is not a register"),
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

fn infer_width(explicit: Option<u8>, source: &Operand, destination: &Operand) -> Option<u8> {
    explicit.or_else(|| {
        operand_register(destination)
            .or_else(|| operand_register(source))
            .map(RegisterRef::width)
    })
}

fn operand_register(operand: &Operand) -> Option<RegisterRef> {
    match operand {
        Operand::Register(register) => Some(*register),
        Operand::Immediate(_) | Operand::Memory(_) => None,
    }
}

fn validate_operand_width(
    operand: &Operand,
    width: u8,
    mnemonic: &str,
    location: &SourceLocation,
) -> Result<(), Diagnostic> {
    if let Operand::Register(register) = operand {
        ensure_register_width(*register, width, mnemonic, location)?;
    }
    Ok(())
}

fn ensure_register_width(
    register: RegisterRef,
    width: u8,
    mnemonic: &str,
    location: &SourceLocation,
) -> Result<(), Diagnostic> {
    if register.width() == width {
        return Ok(());
    }
    Err(Diagnostic::error(
        "E205",
        format!(
            "`{mnemonic}` is {width}-bit, but %{} is {}-bit",
            register.name(),
            register.width()
        ),
        Some(location.clone()),
    )
    .with_help("use register names whose widths match the instruction suffix"))
}

fn resolve_value(
    text: &str,
    data_symbols: &BTreeMap<String, u64>,
    constants: &BTreeMap<String, u64>,
) -> Result<u64, String> {
    if let Ok(value) = parse_immediate(text) {
        return Ok(value);
    }
    constants
        .get(text)
        .or_else(|| data_symbols.get(text))
        .copied()
        .ok_or_else(|| format!("unknown immediate symbol `{text}`"))
}

fn parse_immediate(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if text.len() >= 3 && text.starts_with('\'') && text.ends_with('\'') {
        let inner = &text[1..text.len() - 1];
        let value = match inner {
            "\\n" => b'\n',
            "\\t" => b'\t',
            "\\0" => 0,
            _ if inner.is_ascii() && inner.len() == 1 => inner.as_bytes()[0],
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
    fn lays_out_little_endian_data_and_equates() {
        let program = compile(module(
            ".data\nnum: .long 50,100\nmsg: .asciz \"A\\n\"\nlen = . - msg\n.text\n.global _start\n_start:\n mov $len,%rdi\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(&program.initial_data[..8], &[50, 0, 0, 0, 100, 0, 0, 0]);
        assert_eq!(&program.initial_data[8..], &[b'A', b'\n', 0]);
        assert_eq!(program.constants["len"], 3);
        assert_eq!(program.data_symbols["num"], DATA_BASE);
    }

    #[test]
    fn lays_out_bss_and_rejects_dangerous_storage_sizes() {
        let program = compile(module(
            ".data\nhead: .byte 7\n.bss\nbuff: .skip 16\n.text\n.global _start\n_start:\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(program.initial_data.len(), 17);
        assert_eq!(program.initial_data[0], 7);
        assert_eq!(&program.initial_data[1..], &[0; 16]);
        assert_eq!(program.data_symbol_sections["buff"], ".bss");

        for size in ["-1", "65537", "18446744073709551615"] {
            let source = format!(
                ".bss\nbuff: .skip {size}\n.text\n.global _start\n_start:\n mov $60,%rax\n syscall\n"
            );
            let diagnostics = compile(module(&source)).unwrap_err();
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "E143"),
                "{size} should produce a bounded-storage diagnostic"
            );
        }
    }

    #[test]
    fn parses_scaled_and_rip_relative_addresses() {
        let program = compile(module(
            ".data\nnum: .quad 1,2\n.text\n.global _start\n_start:\n lea num(%rip),%rbx\n mov $1,%rcx\n movq (%rbx,%rcx,8),%rdi\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(program.instructions.len(), 5);
    }

    #[test]
    fn resolves_forward_jump_labels() {
        let program = compile(module(
            ".text\n.global _start\n_start:\n cmp $0,%rax\n jge done\n mov $1,%rdi\ndone:\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(program.labels["done"], 3);
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

    #[test]
    fn diagnoses_invalid_scales_and_ambiguous_memory_widths() {
        let diagnostics = compile(module(
            ".data\nnum: .quad 1\n.text\n.global _start\n_start:\n movq (%rbx,%rcx,3),%rax\n add $1,num(%rip)\n",
        ))
        .unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E223")
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E206")
        );
    }

    #[test]
    fn rejects_two_explicit_memory_operands() {
        let diagnostics = compile(module(
            ".data\nleft: .quad 1\nright: .quad 2\n.text\n.global _start\n_start:\n movq left(%rip),right(%rip)\n",
        ))
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "E217");
    }

    #[test]
    fn resolves_numeric_local_labels_in_both_directions() {
        let program = compile(module(
            ".text\n.global _start\n_start:\n jmp 1f\n1:\n dec %rcx\n jnz 1b\n jmp 1f\n nop_free:\n1: mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        let targets: Vec<_> = program
            .instructions
            .iter()
            .filter_map(|instruction| match &instruction.operation {
                Operation::Jump { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![1, 1, 4]);

        let diagnostics = compile(module(
            ".text\n.global _start\n_start:\n jmp 2b\n2:\n mov $60,%rax\n syscall\n",
        ))
        .unwrap_err();
        assert!(diagnostics[0].message.contains("no local label `2:` above"));
    }

    #[test]
    fn evaluates_parenthesized_equate_and_immediate_expressions() {
        let program = compile(module(
            ".data\narr: .quad 7,3,11,4,9\narrlen = (. - arr) / 8\nbytes = arrlen * 8 + 0x10 - 16\nback = -16\n.text\n.global _start\n_start:\n mov $(arrlen - 1) * 2,%rcx\n mov $60,%rax\n syscall\n",
        ))
        .unwrap();
        assert_eq!(program.constants["arrlen"], 5);
        assert_eq!(program.constants["bytes"], 40);
        assert_eq!(program.constants["back"], (-16_i64) as u64);
        assert!(matches!(
            &program.instructions[0].operation,
            Operation::Mov {
                source: Operand::Immediate(8),
                ..
            }
        ));

        let diagnostics = compile(module(
            ".data\nbad = (4 + 2\n.text\n.global _start\n_start:\n syscall\n",
        ))
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "E140");
    }
}
