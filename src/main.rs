extern crate wasm2cretonne;
extern crate wasmparser;
extern crate cretonne;
extern crate wasmtext;
extern crate docopt;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use wasm2cretonne::module_translator::translate_module;
use cretonne::ir::Function;
use std::path::PathBuf;
use wasmparser::{Parser, ParserState, WasmDecoder, SectionCode};
use wasmtext::Writer;
use std::fs::File;
use std::io::{BufReader, Error, stdout, stdin};
use std::io::prelude::*;
use std::process::Command;
use docopt::Docopt;

const USAGE: &str = "
Wasm to Cretonne IL translation utility

Usage:
    cton-util [-i] file <file>...
    cton-util [-i] all
    cton-util --help | --version

Options:
    -i, --interactive   displays the translated functions
    -h, --help          print this help message
    --version           print the Cretonne version
";

#[derive(Deserialize, Debug)]
struct Args {
    cmd_all: bool,
    arg_file: Vec<String>,
    flag_interactive: bool,
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}


fn main() {
    let test_files = vec!["tests/br_if.wast.0.wasm",
                          "tests/loop.wast.0.wasm",
                          "tests/br_table.wast.0.wasm",
                          "tests/block.wast.0.wasm",
                          "tests/call.wast.0.wasm",
                          "tests/if.wast.0.wasm",
                          "tests/br.wast.0.wasm",
                          "tests/return.wast.0.wasm",
                          "tests/break-drop.wast.0.wasm",
                          "tests/unwind.wast.0.wasm",
                          "tests/unreachable.wast.0.wasm",
                          "tests/set_local.wast.0.wasm",
                          "tests/simple.wasm",
                          "tests/stack.wast.0.wasm",
                          "tests/forward.wast.0.wasm",
                          "tests/nop.wast.0.wasm"]
            .iter()
            .map(|&s| String::from(s))
            .collect();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.help(true).version(Some(format!("0.0.0"))).deserialize())
        .unwrap_or_else(|e| e.exit());

    let files: Vec<String>;
    if args.cmd_all || args.arg_file.len() == 0 {
        files = test_files;
    } else {
        files = args.arg_file;
    }


    for filename in files {
        let path = PathBuf::from(filename.clone());
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
        if args.flag_interactive {
            let mut writer1 = stdout();
            let mut writer2 = stdout();
            match pretty_print_translation(&filename, &data, &funcs, &mut writer1, &mut writer2) {
                Err(error) => panic!(error),
                Ok(()) => {}
            };
        }
    }
}

fn pretty_print_translation(filename: &String,
                            data: &Vec<u8>,
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
        match parser.read() {
            s @ &ParserState::BeginFunctionBody { .. } => {
                write!(writer_cretonne,
                       "====== Begin function block ({}) ======\nWast ---------->\n",
                       filename)?;
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
