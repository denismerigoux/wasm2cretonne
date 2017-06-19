extern crate wasm2cretonne;
extern crate wasmparser;

use wasm2cretonne::wasm_reader::{parser_loop, read_wasm_file};
use wasm2cretonne::sections_parser::{SectionParsingError, parse_function_signatures};
use std::fs::read_dir;
use wasmparser::{ParserState, ParserInput};

fn main() {
    for entry in read_dir("tests").unwrap().take(5) {
        let mut data: Vec<u8> = Vec::new();
        let mut parser = read_wasm_file(entry.unwrap().path(), &mut data)
            .ok()
            .unwrap();
        match *parser.read() {
            ParserState::BeginWasm { .. } => {
                println!("====== Module");
            }
            _ => panic!("modules should begin properly"),
        }
        let signatures = match parse_function_signatures(&mut parser) {
            Ok(signatures) => {
                println!("== Signatures\n{:?}", signatures);
                signatures
            }
            Err(SectionParsingError::NonExistentSection()) => {
                println!("No signatures in the module, skipping it.");
                continue;
            }
            Err(SectionParsingError::WrongSectionContent()) => panic!("wrong section content !"),
        };
        parser_loop(&mut parser);
    }
}
