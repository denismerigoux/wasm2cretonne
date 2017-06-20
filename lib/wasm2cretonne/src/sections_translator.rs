use cretonne::ir::{Signature, ArgumentType};
use cretonne;
use wasmparser::{Parser, ParserState, FuncType, ImportSectionEntryType, ExternalKind};
use wasmparser;
use std::collections::HashMap;
use std::str::from_utf8;

pub enum SectionParsingError {
    WrongSectionContent(),
}

#[derive(Debug)]
pub enum Import {
    Function {
        sig: u32,
        module: String,
        field: String,
    },
    Table(),
    Memory(),
    Global(),
}

// Helper function translating wasmparser types to Cretonne types when possible.
fn type_to_type(ty: &wasmparser::Type) -> Result<cretonne::ir::Type, ()> {
    match *ty {
        wasmparser::Type::I32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::I64 => Ok(cretonne::ir::types::I64),
        wasmparser::Type::F32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::F64 => Ok(cretonne::ir::types::F64),
        _ => Err(()),
    }
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
                                     Err(()) => panic!("only numeric types are supported in function signatures"),
                                 };
                                 ArgumentType::new(cret_arg)
                             }));
                sig.return_types
                    .extend(returns
                    .iter()
                    .map(|ty| {
                             let cret_arg: cretonne::ir::Type = match type_to_type(ty) {
                                 Ok(ty) => ty,
                                 Err(()) => panic!("only numeric types are supported in function signatures"),
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
pub fn parse_import_section(parser: &mut Parser) -> Result<Vec<Import>, SectionParsingError> {
    let mut imports = Vec::new();
    loop {
        match *parser.read() {
            ParserState::ImportSectionEntry {
                module,
                field,
                ty: ImportSectionEntryType::Function(sig),
            } => {
                imports.push(Import::Function {
                                 module: String::from_utf8(module.to_vec()).unwrap(),
                                 field: String::from_utf8(field.to_vec()).unwrap(),
                                 sig,
                             })
            }
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Table(..), .. } => {
                imports.push(Import::Table())
            }
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Memory(..), .. } => {
                imports.push(Import::Memory())
            }
            ParserState::ImportSectionEntry { ty: ImportSectionEntryType::Global(..), .. } => {
                imports.push(Import::Global())
            }
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
