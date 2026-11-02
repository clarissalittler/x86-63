use serde::Serialize;

use crate::{Command, CommandResult, Session};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplInput {
    Command(Command),
    Registers,
    Memory,
    Stack,
    Output,
    Input(String),
    Breakpoint(String),
    Breakpoints,
    ClearBreakpoints,
    Print {
        format: Option<char>,
        expression: String,
    },
    Examine {
        specification: String,
        expression: String,
    },
    Set {
        register: String,
        value: String,
    },
    Backtrace,
    Frame,
    Modules,
    Source(String),
    Layout,
    Why,
    Help(Option<String>),
    Quit,
    Empty,
    Unknown(String),
}

/// What one REPL line produced. `result` is present when the line ran a core
/// command, so scripted sessions can be compared event-for-event across the
/// native and WebAssembly builds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReplResponse {
    pub output: String,
    pub result: Option<CommandResult>,
    pub quit: bool,
}

pub fn parse_repl_input(input: &str) -> ReplInput {
    let trimmed = input.trim();
    let normalized = trimmed.to_ascii_lowercase();
    let (word, rest) = trimmed
        .split_once(char::is_whitespace)
        .map(|(word, rest)| (word, rest.trim()))
        .unwrap_or((trimmed, ""));
    let word = word.to_ascii_lowercase();

    // `print/x`, `p/x`, `x/8xb`: the format is glued to the command word.
    let (base, slash) = word
        .split_once('/')
        .map_or((word.as_str(), None), |(base, format)| (base, Some(format)));
    match base {
        "p" | "print" => {
            if let Some((register, value)) = rest.split_once('=')
                && slash.is_none()
                && register.trim().starts_with('$')
            {
                return ReplInput::Set {
                    register: register.trim().to_string(),
                    value: value.trim().to_string(),
                };
            }
            let format = match slash {
                None => None,
                Some(format) if format.chars().count() == 1 => format.chars().next(),
                Some(format) => {
                    return ReplInput::Unknown(format!(
                        "print/{format}: use one format letter, such as p/x $rax"
                    ));
                }
            };
            return ReplInput::Print {
                format,
                expression: rest.to_string(),
            };
        }
        "x" => {
            return ReplInput::Examine {
                specification: slash.unwrap_or("").to_string(),
                expression: rest.to_string(),
            };
        }
        _ => {}
    }

    match word.as_str() {
        "input" | "stdin" => return ReplInput::Input(rest.to_string()),
        "break" | "b" => return ReplInput::Breakpoint(rest.to_string()),
        "source" => return ReplInput::Source(rest.to_string()),
        "layout" => return ReplInput::Layout,
        "help" | "h" | "?" => {
            return ReplInput::Help((!rest.is_empty()).then(|| rest.to_ascii_lowercase()));
        }
        "set" => {
            let assignment = rest.strip_prefix("var ").unwrap_or(rest);
            return match assignment.split_once('=') {
                Some((register, value)) if !value.trim().is_empty() => ReplInput::Set {
                    register: register.trim().to_string(),
                    value: value.trim().to_string(),
                },
                _ => ReplInput::Unknown("usage: set $REGISTER = EXPRESSION".to_string()),
            };
        }
        _ => {}
    }

    match normalized.as_str() {
        "" => ReplInput::Empty,
        "si" | "s" | "step" | "stepi" => ReplInput::Command(Command::Step),
        "ni" | "n" | "next" | "nexti" => ReplInput::Command(Command::Next),
        "back" | "reverse-stepi" | "rs" | "rsi" => ReplInput::Command(Command::Back),
        "r" | "reset" | "start" => ReplInput::Command(Command::Reset),
        "c" | "continue" | "run" => ReplInput::Command(Command::Continue { max_steps: 10_000 }),
        "regs" | "registers" | "info registers" | "i r" | "info reg" => ReplInput::Registers,
        "mem" | "memory" | "x/data" => ReplInput::Memory,
        "stack" | "x/stack" => ReplInput::Stack,
        "io" | "output" | "stdout" => ReplInput::Output,
        "breakpoints" | "info breakpoints" | "info break" | "info b" | "i b" => {
            ReplInput::Breakpoints
        }
        "clear breakpoints" | "delete breakpoints" | "delete" | "d" => ReplInput::ClearBreakpoints,
        "bt" | "backtrace" | "where" => ReplInput::Backtrace,
        "info frame" | "i f" => ReplInput::Frame,
        "modules" | "info sources" | "info modules" => ReplInput::Modules,
        "why" => ReplInput::Why,
        "q" | "quit" | "exit" => ReplInput::Quit,
        _ => ReplInput::Unknown(format!("unknown command `{trimmed}`; type `help`")),
    }
}

/// Runs one REPL line against a session with the shared, surface-neutral text
/// rendering. Native front ends may render command results and state panes
/// more richly, but every other answer comes from here.
pub fn run_repl_line(session: &mut Session, line: &str) -> ReplResponse {
    let respond = |output: String| ReplResponse {
        output,
        result: None,
        quit: false,
    };
    let command = |session: &mut Session, command: Command| {
        let result = session.execute(command);
        let mut output = result.explanation.clone();
        for diagnostic in &result.diagnostics {
            output.push_str(&format!("\n{}: {}", diagnostic.code, diagnostic.message));
            if let Some(help) = &diagnostic.help {
                output.push_str(&format!("\n  help: {help}"));
            }
        }
        ReplResponse {
            output,
            result: Some(result),
            quit: false,
        }
    };
    match parse_repl_input(line) {
        ReplInput::Command(next) => command(session, next),
        ReplInput::Input(text) => command(session, Command::SubmitInput { text }),
        ReplInput::Breakpoint(specification) => match session.breakpoint_command(&specification) {
            Ok(next) => command(session, next),
            Err(message) => respond(message),
        },
        ReplInput::ClearBreakpoints => command(session, Command::ClearBreakpoints),
        ReplInput::Set { register, value } => {
            command(session, Command::SetRegister { register, value })
        }
        ReplInput::Registers => respond(session.registers_text()),
        ReplInput::Memory => {
            let view = session.view();
            if view.memory.bytes.is_empty() {
                respond("the program has no .data, .rodata, or .bss bytes".to_string())
            } else {
                let units = view.memory.bytes.len().min(256);
                respond(
                    session
                        .examine(&format!("{units}xb"), &view.memory.base)
                        .unwrap_or_else(|message| message),
                )
            }
        }
        ReplInput::Stack => {
            let view = session.view();
            let mut lines = vec![format!(
                "%rsp {} (mod 16 = {}), %rbp {}",
                view.stack.rsp, view.stack.rsp_mod_16, view.stack.rbp
            )];
            lines.extend(view.stack.slots.iter().take(32).map(|slot| {
                format!(
                    "{}: {}  {}",
                    slot.address,
                    slot.value,
                    slot.label.as_deref().unwrap_or("")
                )
                .trim_end()
                .to_string()
            }));
            respond(lines.join("\n"))
        }
        ReplInput::Output => {
            let io = session.view().io;
            respond(format!(
                "stdin:  `{}` ({} of {} bytes consumed)\nstdout: `{}`\nstderr: `{}`",
                io.stdin_escaped,
                io.stdin_consumed,
                io.stdin_bytes.len(),
                io.stdout_escaped,
                io.stderr_escaped
            ))
        }
        ReplInput::Breakpoints => {
            let view = session.view();
            if view.breakpoints.is_empty() {
                respond("no breakpoints set".to_string())
            } else {
                respond(
                    std::iter::once("breakpoints:".to_string())
                        .chain(
                            view.breakpoints
                                .iter()
                                .map(|location| format!("  {}:{}", location.module, location.line)),
                        )
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            }
        }
        ReplInput::Print { format, expression } => respond(
            session
                .print_expression(format, &expression)
                .unwrap_or_else(|message| message),
        ),
        ReplInput::Examine {
            specification,
            expression,
        } => respond(
            session
                .examine(&specification, &expression)
                .unwrap_or_else(|message| message),
        ),
        ReplInput::Backtrace => respond(session.backtrace()),
        ReplInput::Frame => respond(session.frame_info()),
        ReplInput::Modules => respond(session.modules_text()),
        ReplInput::Source(_) => respond(
            "`source FILE` runs a command script in the native REPL; in the browser, use Run script"
                .to_string(),
        ),
        ReplInput::Layout => respond(
            "x86-63 always shows source and registers together: use the TUI (`x86-63 tui`) or the browser"
                .to_string(),
        ),
        ReplInput::Why => respond(session.last_explanation().to_string()),
        ReplInput::Help(topic) => respond(help_text(topic.as_deref())),
        ReplInput::Quit => ReplResponse {
            output: String::new(),
            result: None,
            quit: true,
        },
        ReplInput::Empty => respond(String::new()),
        ReplInput::Unknown(message) => respond(message),
    }
}

pub fn help_text(topic: Option<&str>) -> String {
    match topic {
        None => "\
running:   si|step  ni|next  back|rsi  c|continue|run  start|reset  quit
input:     input TEXT | stdin TEXT   submit one terminal line to a read
breaks:    break LINE|MODULE:LINE|LABEL|*ADDR  info breakpoints  delete
inspect:   info registers  p[/x|d|u|t|o|c] EXPR  x/NFU ADDR  bt  info frame
state:     memory  stack  output  modules  why
change:    set $REG = EXPR   (recorded in history; `back` undoes it)
scripts:   source FILE (native REPL)
Type `help print`, `help x`, `help break`, or `help set` for details."
            .to_string(),
        Some("print") | Some("p") => "\
print EXPR, p/x EXPR, p/d, p/u, p/t, p/o, p/c
EXPR may use $rax (any register name), $rip, &symbol (an address), a bare
data symbol (its value at the declared width), equates, integers, 'c', and
+ - * / % with parentheses. Example: p/x $rsp + 8"
            .to_string(),
        Some("x") => "\
x/NFU ADDR examines memory: N units, format F, unit size U.
F: x hex, d signed, u unsigned, t binary, o octal, c char, s string, i instruction
U: b byte, h 2 bytes, w 4 bytes (default), g 8 bytes
Examples: x/8xb &buff   x/4gx $rsp   x/s &msg   x/3i $pc"
            .to_string(),
        Some("break") | Some("b") => "\
break 12           toggle a breakpoint on line 12 of the current module
break helper.s:8   toggle a breakpoint in another module
break fact         toggle a breakpoint at a .text label
break *0x401005    toggle a breakpoint at an instruction address
continue stops before a breakpoint; delete removes them all"
            .to_string(),
        Some("set") => "\
set $rax = 10       set var $rdi = $rbx + 1       p $rcx = 0
The change is a history entry: `back` restores the old value, and the next
instruction has not run yet."
            .to_string(),
        Some(other) => format!("no help topic `{other}`; try `help`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceModule;

    fn session(source: &str) -> Session {
        Session::from_modules(vec![SourceModule::new("main.s", source)]).unwrap()
    }

    #[test]
    fn accepts_gdb_and_friendly_aliases() {
        assert_eq!(parse_repl_input("si"), ReplInput::Command(Command::Step));
        assert_eq!(parse_repl_input("step"), ReplInput::Command(Command::Step));
        assert_eq!(parse_repl_input("info registers"), ReplInput::Registers);
        assert_eq!(
            parse_repl_input("break helper.s:12"),
            ReplInput::Breakpoint("helper.s:12".to_string())
        );
        assert_eq!(parse_repl_input("info breakpoints"), ReplInput::Breakpoints);
        assert_eq!(
            parse_repl_input("p/x $rax"),
            ReplInput::Print {
                format: Some('x'),
                expression: "$rax".to_string()
            }
        );
        assert_eq!(
            parse_repl_input("x/8xb &buff"),
            ReplInput::Examine {
                specification: "8xb".to_string(),
                expression: "&buff".to_string()
            }
        );
        assert_eq!(
            parse_repl_input("set var $rax = 5"),
            ReplInput::Set {
                register: "$rax".to_string(),
                value: "5".to_string()
            }
        );
        assert_eq!(parse_repl_input("bt"), ReplInput::Backtrace);
        assert_eq!(
            parse_repl_input("stdin 42"),
            ReplInput::Input("42".to_string())
        );
    }

    #[test]
    fn print_examine_and_set_share_one_expression_language() {
        let mut session = session(
            ".data\nnum: .quad 200\nmsg: .asciz \"hi\\n\"\n.text\n.global _start\n_start:\n mov $30,%rax\n mov $-1,%ebx\n mov $60,%rax\n syscall\n",
        );
        run_repl_line(&mut session, "si");
        run_repl_line(&mut session, "si");
        assert_eq!(run_repl_line(&mut session, "p $rax").output, "$1 = 30");
        assert_eq!(run_repl_line(&mut session, "p/x $rax").output, "$2 = 0x1e");
        assert_eq!(run_repl_line(&mut session, "p/t $rax").output, "$3 = 11110");
        assert_eq!(run_repl_line(&mut session, "p/d $ebx").output, "$4 = -1");
        assert_eq!(
            run_repl_line(&mut session, "p/u $ebx").output,
            "$5 = 4294967295"
        );
        assert_eq!(run_repl_line(&mut session, "p num + 1").output, "$6 = 201");
        assert_eq!(
            run_repl_line(&mut session, "p/x &num").output,
            "$7 = 0x400000"
        );
        assert_eq!(
            run_repl_line(&mut session, "x/s &msg").output,
            "0x400008 <msg>:\t\"hi\\n\""
        );
        assert_eq!(
            run_repl_line(&mut session, "x/2xb &num").output,
            "0x400000 <num>:\t0xc8\t0x00"
        );
        assert_eq!(
            run_repl_line(&mut session, "x/1gx $rsp").output,
            "Cannot access memory at address 0x7fffffffe000"
        );
        assert!(
            run_repl_line(&mut session, "x/s msg")
                .output
                .contains("write `&msg` for its address")
        );
        assert!(
            run_repl_line(&mut session, "x/2i $pc")
                .output
                .starts_with("=> 0x401002 <_start+2>:\tmov $60,%rax")
        );

        let set = run_repl_line(&mut session, "set $rdi = $rax * 2 + 2");
        assert!(set.result.is_some());
        assert_eq!(run_repl_line(&mut session, "p $rdi").output, "$8 = 62");
        run_repl_line(&mut session, "back");
        assert_eq!(run_repl_line(&mut session, "p $rdi").output, "$9 = 0");
        assert!(
            run_repl_line(&mut session, "set $rip = 0")
                .output
                .contains("not supported")
        );
    }

    #[test]
    fn breakpoints_resolve_lines_labels_and_addresses() {
        let mut session =
            session(".text\n.global _start\n_start:\n mov $60,%rax\nend:\n syscall\n");
        assert_eq!(
            session.breakpoint_command("4"),
            Ok(Command::ToggleBreakpoint {
                module: "main.s".to_string(),
                line: 4
            })
        );
        assert_eq!(
            session.breakpoint_command("end"),
            Ok(Command::ToggleBreakpoint {
                module: "main.s".to_string(),
                line: 6
            })
        );
        assert_eq!(
            session.breakpoint_command("*0x401001"),
            Ok(Command::ToggleBreakpoint {
                module: "main.s".to_string(),
                line: 6
            })
        );
        run_repl_line(&mut session, "break end");
        let stopped = run_repl_line(&mut session, "c");
        assert!(stopped.output.contains("breakpoint at main.s:6"));
        assert!(
            run_repl_line(&mut session, "bt")
                .output
                .starts_with("#0  main.s:6")
        );
    }
}
