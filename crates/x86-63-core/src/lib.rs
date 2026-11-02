mod diagnostic;
mod expression;
mod machine;
mod parser;
mod program;
mod protocol;
mod repl;
mod session;

pub use diagnostic::{Diagnostic, Severity};
pub use machine::{Flags, MachineStatus};
pub use parser::compile;
pub use program::{Program, ProgramView, SourceLocation, SourceModule, SymbolView};
pub use protocol::{
    Command, CommandResult, FlagsView, IoView, MachineView, MemoryView, PROTOCOL_VERSION,
    RegisterView, StackFrameView, StackSlotView, StackView, StepEvent,
};
pub use repl::{ReplInput, ReplResponse, help_text, parse_repl_input, run_repl_line};
pub use session::{BuildError, Session};
