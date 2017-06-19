use wasmparser::{ParserState, SectionCode, ParserInput, Parser};
use sections_translator::{SectionParsingError, parse_function_signatures, parse_import_section,
                          parse_function_section, Import};
use cretonne::ir::Function;
use code_translator::translate_function_body;
use cretonne::ir::frontend::ILBuilder;

pub fn translate_module(data: Vec<u8>) -> Result<Vec<Function>, String> {
    let mut parser = Parser::new(data.as_slice());
    match *parser.read() {
        ParserState::BeginWasm { .. } => {
            println!("====== Module");
        }
        _ => panic!("modules should begin properly"),
    }
    match *parser.read() {
        ParserState::BeginSection { code: SectionCode::Type, .. } => (),
        _ => return Err(String::from("no function signature in the module")),
    };
    let signatures = match parse_function_signatures(&mut parser) {
        Ok(signatures) => {
            println!("== Signatures\n{:?}", signatures);
            signatures
        }
        Err(SectionParsingError::WrongSectionContent()) => {
            return Err(String::from("wrong content in the type section"))
        }
    };
    let mut imports: Option<Vec<Import>> = None;
    let mut functions: Option<Vec<usize>> = None;
    let mut next_input = ParserInput::Default;
    loop {
        match *parser.read_with_input(next_input) {
            ParserState::BeginSection { code: SectionCode::Import, .. } => {
                match parse_import_section(&mut parser) {
                    Ok(imp) => {
                        imports = {
                            println!("== Imports\n{:?}", imp);
                            Some(imp)
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
                        functions = {
                            println!("== Functions' signature index\n{:?}", funcs);
                            Some(funcs)
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
                next_input = ParserInput::SkipSection;
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

    let mut function_index: usize = 0;
    let mut il_functions: Vec<Function> = Vec::new();
    let mut il_builder = ILBuilder::new();
    loop {
        match *parser.read() {
            ParserState::BeginFunctionBody { .. } => {
                let signature = signatures[functions[function_index]].clone();
                println!("-> function");
                match translate_function_body(&mut parser, signature, &imports, &mut il_builder) {
                    Ok(il_func) => il_functions.push(il_func),
                    Err(s) => return Err(s),
                }
            }
            ParserState::EndSection => break,
            _ => return Err(String::from(format!("wrong content in code section"))),
        }
        function_index += 1;
    }
    Ok(il_functions)
}
