extern crate wasm2cretonne;
extern crate wasmparser;
extern crate cretonne;
extern crate wasmtext;

use wasm2cretonne::module_translator::translate_module;
use cretonne::ir::Function;
use std::path::PathBuf;
use wasmparser::{Parser, ParserState, WasmDecoder, SectionCode};
use wasmtext::Writer;
use std::fs::File;
use std::io::{BufReader, Error, stdout, stdin};
use std::io::prelude::*;
use std::process::Command;

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}


fn main() {
    let files = vec!["tests/br_if.wast.0.wasm",
                     "tests/loop.wast.0.wasm",
                     "tests/br_table.wast.0.wasm",
                     "tests/block.wast.0.wasm",
                     "tests/call.wast.0.wasm",
                     "tests/br.wast.0.wasm"];


    for filename in files {
        let path = PathBuf::from(filename);
        println!("Reading: {:?}", path.as_os_str());
        let data = match read_wasm_file(path) {
            Ok(data) => data,
            Err(err) => {
                println!("Error: {}", err);
                return;
            }
        };
        let funcs = match translate_module(&data) {
            Ok(funcs) => funcs,
            Err(string) => {
                println!("Error : {}", string);
                return;
            }
        };
        let mut writer1 = stdout();
        let mut writer2 = stdout();
        match pretty_print_translation(&data, &funcs, &mut writer1, &mut writer2) {
            Err(error) => panic!(error),
            Ok(()) => {}
        };
    }
}

fn pretty_print_translation(data: &Vec<u8>,
                            funcs: &Vec<Function>,
                            writer_wast: &mut Write,
                            writer_cretonne: &mut Write)
                            -> Result<(), Error> {
    let mut parser = Parser::new(data.as_slice());
    let mut parser_writer = Writer::new(writer_wast);
    match parser.read() {
        s @ &ParserState::BeginWasm { .. } => parser_writer.write(&s)?,
        _ => panic!("modules should begin properly"),
    }
    loop {
        match parser.read() {
            s @ &ParserState::BeginSection { code: SectionCode::Code, .. } => {
                // The code section begins
                parser_writer.write(&s)?;
                break;
            }
            &ParserState::EndWasm => panic!("module ended with no code"),
            s @ _ => parser_writer.write(&s)?,
        }
    }
    let mut function_index = 0;
    loop {
        write!(writer_cretonne,
               "====== Begin function block ======\nWast ---------->\n")?;
        match parser.read() {
            s @ &ParserState::BeginFunctionBody { .. } => {
                parser_writer.write(&s)?;
            }
            s @ &ParserState::EndSection => {
                parser_writer.write(&s)?;
                break;
            }
            _ => panic!("wrong content in code section"),
        }
        {
            loop {
                match parser.read() {
                    s @ &ParserState::EndFunctionBody => {
                        parser_writer.write(&s)?;
                        break;
                    }
                    s @ _ => {
                        parser_writer.write(&s)?;
                    }
                };
            }
        }
        let mut function_string = format!("  {}", funcs[function_index].display(None));
        function_string.pop();
        let function_str = str::replace(function_string.as_str(), "\n", "\n  ");
        write!(writer_cretonne, "Cretonne IL --->\n{}\n", function_str)?;
        write!(writer_cretonne, "====== End function block ======\n")?;

        let mut input = String::new();
        stdin().read_line(&mut input)?;
        Command::new("clear").status()?;

        function_index += 1;
    }
    Ok(())
}
