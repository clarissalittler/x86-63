use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::machine::Operation;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceModule {
    pub name: String,
    pub source: String,
}

impl SourceModule {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub module: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub operation: Operation,
    pub location: SourceLocation,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub(crate) modules: Vec<SourceModule>,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) labels: BTreeMap<String, usize>,
    pub(crate) entry: usize,
}

impl Program {
    pub fn view(&self) -> ProgramView {
        ProgramView {
            modules: self.modules.clone(),
            instructions: self
                .instructions
                .iter()
                .enumerate()
                .map(|(index, instruction)| InstructionView {
                    index,
                    location: instruction.location.clone(),
                    text: instruction.text.clone(),
                })
                .collect(),
            labels: self.labels.clone(),
            entry: self.entry,
        }
    }

    pub(crate) fn instruction(&self, pc: usize) -> Option<&Instruction> {
        self.instructions.get(pc)
    }

    pub(crate) fn entry(&self) -> usize {
        self.entry
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionView {
    pub index: usize,
    pub location: SourceLocation,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramView {
    pub modules: Vec<SourceModule>,
    pub instructions: Vec<InstructionView>,
    pub labels: BTreeMap<String, usize>,
    pub entry: usize,
}
