use translation_utils::{type_to_type, Memory};
use cretonne::ir::{Signature, ArgumentType};
use cretonne;
use wasmparser::{Parser, ParserState, FuncType, ImportSectionEntryType, ExternalKind, WasmDecoder};
use wasmparser;
use std::collections::HashMap;
use std::str::from_utf8;

pub enum SectionParsingError {
    WrongSectionContent(),
}

/// Reads the Type Section of the wasm module and returns the corresponding function signatures.
pub fn parse_function_signatures(parser: &mut Parser)
                                 -> Result<Vec<Signature>, SectionParsingError> {
    let mut signatures: Vec<Signature> = Vec::new();
    loop {
        match *parser.read() {
            ParserState::EndSection => break,
            ParserState::TypeSectionEntry(FuncType {
                                              form: wasmparser::Type::Func,
                                              ref params,
                                              ref returns,
                                          }) => {
                let mut sig = Signature::new();
                sig.argument_types
                    .extend(params
                                .iter()
                                .map(|ty| {
                        let cret_arg: cretonne::ir::Type = match type_to_type(ty) {
                            Ok(ty) => ty,
                            Err(()) => panic!("only numeric types are supported in\
                                      function signatures"),
                        };
                        ArgumentType::new(cret_arg)
                    }));
                sig.return_types
                    .extend(returns
                                .iter()
                                .map(|ty| {
                        let cret_arg: cretonne::ir::Type = match type_to_type(ty) {
                            Ok(ty) => ty,
                            Err(()) => panic!("only numeric types are supported in\
                                  function signatures"),
                        };
                        ArgumentType::new(cret_arg)
                    }));
                signatures.push(sig);
            }
            _ => return Err(SectionParsingError::WrongSectionContent()),
        }
    }
    Ok(signatures)
}

/// Retrieves the imports from the imports section of the binary.
pub fn parse_import_section(parser: &mut Parser) -> Result<Vec<u32>, SectionParsingError> {
    let mut imports = Vec::new();
    loop {
        match *parser.read() {
            ParserState::ImportSectionEntry {
                ty: ImportSectionEntryType::Function(sig), ..
            } => imports.push(sig),
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Table(..), .. } => {}
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Memory(..), .. } => {}
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Global(..), .. } => {}
            ParserState::EndSection => break,
            _ => return Err(SectionParsingError::WrongSectionContent()),
        };
    }
    Ok(imports)
}

/// Retrieves the correspondances between functions and signatures from the function section
pub fn parse_function_section(parser: &mut Parser) -> Result<Vec<u32>, SectionParsingError> {
    let mut funcs = Vec::new();
    loop {
        match *parser.read() {
            ParserState::FunctionSectionEntry(sigindex) => funcs.push(sigindex as u32),
            ParserState::EndSection => break,
            _ => return Err(SectionParsingError::WrongSectionContent()),
        };
    }
    Ok(funcs)
}

/// Retrieves the names of the functions from the export section
pub fn parse_export_section(parser: &mut Parser)
                            -> Result<HashMap<u32, String>, SectionParsingError> {
    let mut exports: HashMap<u32, String> = HashMap::new();
    loop {
        match *parser.read() {
            ParserState::ExportSectionEntry {
                field,
                kind: ExternalKind::Function,
                index,
            } => {
                exports.insert(index, String::from(from_utf8(field).unwrap()));
            }
            ParserState::EndSection => break,
            _ => return Err(SectionParsingError::WrongSectionContent()),
        };
    }
    Ok(exports)
}

/// Retrieves the size and maximum fields of memories from the memory section
pub fn parse_memory_section(parser: &mut Parser) -> Result<Vec<Memory>, SectionParsingError> {
    let mut memories: Vec<Memory> = Vec::new();
    loop {
        match *parser.read() {
            ParserState::MemorySectionEntry(ref ty) => {
                memories.push(Memory {
                                  size: ty.limits.initial,
                                  maximum: ty.limits.maximum,
                              })
            }
            ParserState::EndSection => break,
            _ => return Err(SectionParsingError::WrongSectionContent()),
        };
    }
    Ok(memories)
}
