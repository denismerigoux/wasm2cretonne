use std::path::PathBuf;
use wasm_reader::{read_wasm_file, parser_loop};
use wasmparser::{ParserState, SectionCode, ParserInput};
use sections_parser::{SectionParsingError, parse_function_signatures, parse_import_section,
                      parse_function_section, Import};

pub fn parse_module_preamble(path: PathBuf) -> Result<(), String> {
    let mut data: Vec<u8> = Vec::new();
    let mut parser = read_wasm_file(path, &mut data).ok().unwrap();
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
    let mut functions: Option<Vec<u32>> = None;
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
            _ => return Err(String::from(format!("wrong content in the preamble"))),
        };
    }
    // At this point we've entered the code section
    parser_loop(&mut parser);
    Ok(())
}
