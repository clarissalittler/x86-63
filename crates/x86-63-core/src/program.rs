use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::machine::Operation;

pub(crate) const DATA_BASE: u64 = 0x0000_0000_0040_0000;

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
    pub(crate) data_symbols: BTreeMap<String, u64>,
    pub(crate) data_symbol_widths: BTreeMap<String, usize>,
    pub(crate) constants: BTreeMap<String, u64>,
    pub(crate) initial_data: Vec<u8>,
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
            symbols: self.symbol_views(),
            constants: self
                .constants
                .iter()
                .map(|(name, value)| (name.clone(), value.to_string()))
                .collect(),
            data_base: format!("0x{:016x}", DATA_BASE),
            data_size: self.initial_data.len(),
            entry: self.entry,
        }
    }

    pub(crate) fn instruction(&self, pc: usize) -> Option<&Instruction> {
        self.instructions.get(pc)
    }

    pub(crate) fn entry(&self) -> usize {
        self.entry
    }

    pub(crate) fn initial_data(&self) -> &[u8] {
        &self.initial_data
    }

    fn symbol_views(&self) -> Vec<SymbolView> {
        let data_end = DATA_BASE + self.initial_data.len() as u64;
        let mut symbols = self
            .data_symbols
            .iter()
            .map(|(name, address)| (name.clone(), *address))
            .collect::<Vec<_>>();
        symbols.sort_by_key(|(_, address)| *address);
        symbols
            .iter()
            .enumerate()
            .map(|(index, (name, address))| {
                let next = symbols
                    .get(index + 1)
                    .map_or(data_end, |(_, next_address)| *next_address);
                SymbolView {
                    name: name.clone(),
                    address: format!("0x{address:016x}"),
                    offset: (*address - DATA_BASE) as usize,
                    size: next.saturating_sub(*address) as usize,
                    element_width: self.data_symbol_widths.get(name).copied().unwrap_or(1),
                    section: ".data".to_string(),
                }
            })
            .collect()
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
    pub symbols: Vec<SymbolView>,
    pub constants: BTreeMap<String, String>,
    pub data_base: String,
    pub data_size: usize,
    pub entry: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolView {
    pub name: String,
    pub address: String,
    pub offset: usize,
    pub size: usize,
    pub element_width: usize,
    pub section: String,
}
