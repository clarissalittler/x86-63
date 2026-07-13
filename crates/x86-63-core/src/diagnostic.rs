use serde::{Deserialize, Serialize};

use crate::SourceLocation;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub help: Option<String>,
    pub location: Option<SourceLocation>,
}

impl Diagnostic {
    pub(crate) fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            help: None,
            location,
        }
    }

    pub(crate) fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
