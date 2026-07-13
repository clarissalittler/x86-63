mod diagnostic;
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
    RegisterView, StepEvent,
};
pub use repl::{ReplInput, parse_repl_input};
pub use session::{BuildError, Session};
