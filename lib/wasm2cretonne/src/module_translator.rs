use wasmparser::{ParserState, SectionCode, ParserInput, Parser};
use sections_translator::{SectionParsingError, parse_function_signatures, parse_import_section,
                          parse_function_section, parse_export_section};
use translation_utils::type_to_type;
use cretonne::ir::{Function, Type};
use code_translator::translate_function_body;
use cretonne::ir::frontend::ILBuilder;
use std::collections::HashMap;

pub fn translate_module(data: Vec<u8>) -> Result<Vec<Function>, String> {
    let mut parser = Parser::new(data.as_slice());
    match *parser.read() {
        ParserState::BeginWasm { .. } => {}
        _ => panic!("modules should begin properly"),
    }
    match *parser.read() {
        ParserState::BeginSection { code: SectionCode::Type, .. } => (),
        _ => return Err(String::from("no function signature in the module")),
    };
    let signatures = match parse_function_signatures(&mut parser) {
        Ok(signatures) => signatures,
        Err(SectionParsingError::WrongSectionContent()) => {
            return Err(String::from("wrong content in the type section"))
        }
    };
    let mut functions: Option<Vec<u32>> = None;
    let mut exports: Option<HashMap<u32, String>> = None;
    let mut next_input = ParserInput::Default;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection { code: SectionCode::Import, .. } => {
                match parse_import_section(&mut parser) {
                    Ok(imp) => functions = Some(imp),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the import section"))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Function, .. } => {
                match parse_function_section(&mut parser) {
                    Ok(funcs) => {
                        match functions {
                            None => functions = Some(funcs),
                            Some(ref mut imps) => imps.extend(funcs),
                        }
                    }
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the function section"))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Table, .. } => {
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Memory, .. } => {
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Global, .. } => {
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Export, .. } => {
                match parse_export_section(&mut parser) {
                    Ok(exps) => exports = Some(exps),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the function section"))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Start, .. } => {
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Element, .. } => {
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Code, .. } => {
                // The code section begins
                break;
            }
            ParserState::EndSection => {
                next_input = ParserInput::Default;
            }
            ParserState::EndWasm => return Err(String::from("module ended with no code")),
            _ => return Err(String::from("wrong content in the preamble")),
        };
    }
    // At this point we've entered the code section
    // First we check that we have all that is necessary to translate a function.
    let functions = match functions {
        None => return Err(String::from("missing a function section")),
        Some(functions) => functions,
    };

    let mut function_index: u32 = 0;
    let mut il_functions: Vec<Function> = Vec::new();
    let mut il_builder = ILBuilder::new();
    loop {
        let locals: Vec<(u32, Type)> = match *parser.read() {
            ParserState::BeginFunctionBody { ref locals, .. } => {
                locals
                    .iter()
                    .map(|&(index, ref ty)| {
                             (index,
                              match type_to_type(ty) {
                                  Ok(ty) => ty,
                                  Err(()) => panic!("unsupported type for local variable"),
                              })
                         })
                    .collect()
            }
            ParserState::EndSection => break,
            _ => return Err(String::from(format!("wrong content in code section"))),
        };
        let signature = signatures[functions[function_index as usize] as usize].clone();
        match translate_function_body(&mut parser,
                                      function_index,
                                      signature,
                                      &locals,
                                      &exports,
                                      &signatures,
                                      &functions,
                                      &mut il_builder) {
            Ok(il_func) => il_functions.push(il_func),
            Err(s) => return Err(s),
        }
        function_index += 1;
    }
    Ok(il_functions)
}
