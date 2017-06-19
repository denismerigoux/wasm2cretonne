use cretonne::ir::{Signature, ArgumentType};
use cretonne;
use wasmparser::{Parser, ParserState, SectionCode, FuncType};
use wasmparser;
use translations::type_to_type;

pub enum SectionParsingError {
    NonExistentSection(),
    WrongSectionContent(),
}

/// Reads the Type Section of the wasm module. Expects that the first call to `parser.read()`
/// returns `ParserState::BeginSection` and will return a parser ready to read the next section.
pub fn parse_function_signatures<'a>(parser: &'a mut Parser)
                                     -> Result<Vec<Signature>, SectionParsingError> {
    match *parser.read() {
        ParserState::BeginSection { code: SectionCode::Type, .. } => (),
        _ => return Err(SectionParsingError::NonExistentSection()),
    };
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
