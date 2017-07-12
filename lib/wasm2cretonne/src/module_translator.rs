use wasmparser::{ParserState, SectionCode, ParserInput, Parser, WasmDecoder};
use sections_translator::{SectionParsingError, parse_function_signatures, parse_import_section,
                          parse_function_section, parse_export_section, parse_memory_section,
                          parse_global_section, parse_table_section};
use translation_utils::{type_to_type, Import};
use cretonne::ir::{Function, Type};
use code_translator::translate_function_body;
use cton_frontend::ILBuilder;
use std::collections::HashMap;
use runtime::WasmRuntime;

pub fn translate_module(data: &Vec<u8>,
                        runtime: &mut WasmRuntime)
                        -> Result<Vec<Function>, String> {
    let mut parser = Parser::new(data.as_slice());
    match *parser.read() {
        ParserState::BeginWasm { .. } => {}
        ref s @ _ => panic!("modules should begin properly: {:?}", s),
    }
    let mut signatures = None;
    let mut functions: Option<Vec<u32>> = None;
    let mut exports: Option<HashMap<u32, String>> = None;
    let mut next_input = ParserInput::Default;
    let mut function_index: u32 = 0;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection { code: SectionCode::Type, .. } => {
                match parse_function_signatures(&mut parser) {
                    Ok(sigs) => signatures = Some(sigs),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the type section"))
                    }
                };
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Import, .. } => {
                match parse_import_section(&mut parser) {
                    Ok(imps) => {
                        for import in imps {
                            match import {
                                Import::Function { sig_index } => {
                                    functions = match functions {
                                        None => Some(vec![sig_index]),
                                        Some(mut funcs) => {
                                            funcs.push(sig_index);
                                            Some(funcs)
                                        }
                                    };
                                    function_index += 1;
                                }
                                Import::Memory(mem) => {
                                    runtime.declare_memory(mem);
                                }
                                Import::Global(glob) => {
                                    runtime.declare_global(glob);
                                }
                            }
                        }
                    }
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
                match parse_table_section(&mut parser, runtime) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the table section"))
                    }
                }
            }
            ParserState::BeginSection { code: SectionCode::Memory, .. } => {
                match parse_memory_section(&mut parser) {
                    Ok(mems) => {
                        for mem in mems {
                            runtime.declare_memory(mem);
                        }
                    }
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the memory section"))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Global, .. } => {
                match parse_global_section(&mut parser, runtime) {
                    Ok(()) => (),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the global section"))
                    }
                }
                next_input = ParserInput::Default;
            }
            ParserState::BeginSection { code: SectionCode::Export, .. } => {
                match parse_export_section(&mut parser) {
                    Ok(exps) => exports = Some(exps),
                    Err(SectionParsingError::WrongSectionContent()) => {
                        return Err(String::from("wrong content in the export section"))
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
            ParserState::BeginSection { code: SectionCode::Data, .. } => {
                // TODO: handle it with runtime
                next_input = ParserInput::SkipSection;
            }
            ParserState::BeginSection { code: SectionCode::Code, .. } => {
                // The code section begins
                break;
            }
            ParserState::EndSection => {
                next_input = ParserInput::Default;
            }
            ParserState::EndWasm => return Ok(Vec::new()),
            _ => return Err(String::from("wrong content in the preamble")),
        };
    }
    // At this point we've entered the code section
    // First we check that we have all that is necessary to translate a function.
    let signatures = match signatures {
        None => Vec::new(),
        Some(sigs) => sigs,
    };
    let functions = match functions {
        None => return Err(String::from("missing a function section")),
        Some(functions) => functions,
    };
    let mut il_functions: Vec<Function> = Vec::new();
    let mut il_builder = ILBuilder::new();
    runtime.instantiate();
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
                                      &mut il_builder,
                                      runtime) {
            Ok(il_func) => il_functions.push(il_func),
            Err(s) => return Err(s),
        }
        function_index += 1;
    }
    Ok(il_functions)
}
