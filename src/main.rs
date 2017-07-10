extern crate wasm2cretonne;
extern crate wasmparser;
extern crate cretonne;
extern crate wasmtext;
extern crate docopt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate term;

use wasm2cretonne::module_translator::translate_module;
use cretonne::ir::Function;
use std::path::PathBuf;
use wasmparser::{Parser, ParserState, WasmDecoder, SectionCode};
use wasmtext::Writer;
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::{BufReader, stdout, stdin};
use std::io::prelude::*;
use docopt::Docopt;
use std::fs;
use std::path::Path;

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

#[derive(Deserialize, Debug, Clone)]
struct Args {
    cmd_all: bool,
    arg_file: Vec<String>,
    flag_interactive: bool,
}

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}


fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.help(true).version(Some(format!("0.0.0"))).deserialize())
        .unwrap_or_else(|e| e.exit());
    let mut terminal = term::stdout().unwrap();
    let mut paths: Vec<_> = fs::read_dir("tests").unwrap().map(|r| r.unwrap()).collect();
    let total_files = paths.len();
    paths.sort_by_key(|dir| dir.path());

    if args.cmd_all {
        let mut files_ok = 0;
        for path in paths {
            let path = path.path();
            let name = String::from(path.as_os_str().to_string_lossy());
            match handle_module(args.flag_interactive, path, name) {
                Ok(()) => files_ok +=1,
                Err(message) => println!("{}", message),
            };
        }
        terminal.fg(term::color::GREEN).unwrap();
        println!("Test files coverage: {}/{} ({:.0}%)",
                 files_ok,
                 total_files,
                 100.0 * (files_ok as f32) / (total_files as f32));
        terminal.reset().unwrap();
    }
    for filename in args.arg_file {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        match handle_module(args.flag_interactive, path.to_path_buf(), name) {
            Ok(()) => {}
            Err(message) => println!("{}", message),
        }
    }
}

fn handle_module(interactive: bool, path: PathBuf, name: String) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    terminal.fg(term::color::YELLOW).unwrap();
    print!("Testing: ");
    terminal.reset().unwrap();
    print!("\"{}\"", name);
    let data = match read_wasm_file(path) {
        Ok(data) => data,
        Err(err) => {
            println!("Error: {}", err);
            return Err(String::from(err.description()));
        }
    };
    let funcs = match translate_module(&data) {
        Ok(funcs) => funcs,
        Err(string) => {
            terminal.fg(term::color::RED).unwrap();
            println!(" error");
            terminal.reset().unwrap();
            return Err(string);
        }
    };
    if interactive {
        println!();
        let mut writer1 = stdout();
        let mut writer2 = stdout();
        match pretty_print_translation(&name, &data, &funcs, &mut writer1, &mut writer2) {
            Err(error) => return Err(String::from(error.description())),
            Ok(()) => Ok(()),
        }
    } else {
        terminal.fg(term::color::GREEN).unwrap();
        println!(" ok");
        terminal.reset().unwrap();
        Ok(())
    }
}

fn pretty_print_translation(filename: &String,
                            data: &Vec<u8>,
                            funcs: &Vec<Function>,
                            writer_wast: &mut Write,
                            writer_cretonne: &mut Write)
                            -> Result<(), io::Error> {
    let mut terminal = term::stdout().unwrap();
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
            &ParserState::EndWasm => return Ok(()),
            s @ _ => parser_writer.write(&s)?,
        }
    }
    let mut function_index = 0;
    loop {
        match parser.read() {
            s @ &ParserState::BeginFunctionBody { .. } => {
                terminal.fg(term::color::BLUE).unwrap();
                write!(writer_cretonne,
                       "====== Function No. {} of module \"{}\" ======\n",
                       function_index,
                       filename)?;
                terminal.fg(term::color::CYAN).unwrap();
                write!(writer_cretonne, "Wast ---------->\n")?;
                terminal.reset().unwrap();
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
        terminal.fg(term::color::CYAN).unwrap();
        write!(writer_cretonne, "Cretonne IL --->\n")?;
        terminal.reset().unwrap();
        write!(writer_cretonne, "{}\n", function_str)?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        function_index += 1;
    }
    Ok(())
}
