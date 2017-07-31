//! CLI tool to use the functions provided by crates [wasm2cretonne](../wasm2cretonne/index.html)
//! and [wasmstandalone](../wasmstandalone/index.html).
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cretonne
//! IL. Can also executes the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

extern crate wasm2cretonne;
extern crate wasmstandalone;
extern crate wasmparser;
extern crate cretonne;
extern crate wasmtext;
extern crate docopt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate term;
extern crate tempdir;

use wasm2cretonne::{translate_module, TranslationResult, FunctionTranslation, DummyRuntime,
                    WasmRuntime};
use wasmstandalone::{StandaloneRuntime, execute_module};
use std::path::PathBuf;
use wasmparser::{Parser, ParserState, WasmDecoder, SectionCode};
use wasmtext::Writer;
use std::fs::File;
use std::error::Error;
use std::io;
use std::io::{BufReader, stdout};
use std::io::prelude::*;
use docopt::Docopt;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempdir::TempDir;

const USAGE: &str = "
Wasm to Cretonne IL translation utility.
Takes a binary WebAssembly module and returns its functions in Cretonne IL format.
The translation is dependent on the runtime chosen.
The default is a dummy runtime that produces placeholder values.

Usage:
    cton-util [-v] file <file>...
    cton-util -e [-mv] file <file>...
    cton-util [-v] all
    cton-util -e [-mv] all
    cton-util --help | --version

Options:
    -v, --verbose       displays the module and translated functions
    -e, --execute       enable the standalone runtime and executes the start function of the module
    -m, --memory        interactive memory inspector after execution
    -h, --help          print this help message
    --version           print the Cretonne version
";

#[derive(Deserialize, Debug, Clone)]
struct Args {
    cmd_all: bool,
    arg_file: Vec<String>,
    flag_verbose: bool,
    flag_execute: bool,
    flag_memory: bool,
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

    if args.cmd_all {
        let mut paths: Vec<_> = fs::read_dir("testsuite")
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let total_files = paths.len();
        paths.sort_by_key(|dir| dir.path());

        let mut files_ok = 0;
        for path in paths {
            let path = path.path();
            let name = String::from(path.as_os_str().to_string_lossy());
            match handle_module(&args, path, name) {
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
    for filename in args.arg_file.iter() {
        let path = Path::new(&filename);
        let name = String::from(path.as_os_str().to_string_lossy());
        match handle_module(&args, path.to_path_buf(), name) {
            Ok(()) => {}
            Err(message) => println!("{}", message),
        }
    }
}

fn handle_module(args: &Args, path: PathBuf, name: String) -> Result<(), String> {
    let mut terminal = term::stdout().unwrap();
    terminal.fg(term::color::YELLOW).unwrap();
    print!("Translating: ");
    terminal.reset().unwrap();
    print!("\"{}\"", name);
    let data = match path.extension() {
        None => {
            terminal.fg(term::color::RED).unwrap();
            println!(" error");
            terminal.reset().unwrap();
            return Err(String::from("the file extension is not wasm or wast"));
        }
        Some(ext) => {
            match ext.to_str() {
                Some("wasm") => {
                    match read_wasm_file(path.clone()) {
                        Ok(data) => data,
                        Err(err) => {
                            terminal.fg(term::color::RED).unwrap();
                            println!(" error");
                            terminal.reset().unwrap();
                            return Err(String::from(err.description()));
                        }
                    }
                }
                Some("wast") => {
                    let tmp_dir = TempDir::new("wasm2cretonne").unwrap();
                    let file_path = tmp_dir.path().join("module.wasm");
                    File::create(file_path.clone()).unwrap();
                    Command::new("wast2wasm")
                        .arg(path.clone())
                        .arg("-o")
                        .arg(file_path.to_str().unwrap())
                        .output()
                        .or_else(|e| {
                            terminal.fg(term::color::RED).unwrap();
                            println!(" error");
                            terminal.reset().unwrap();
                            if let io::ErrorKind::NotFound = e.kind() {
                                return Err(String::from("wast2wasm not found"));
                            } else {
                                return Err(String::from(e.description()));
                            }
                        })
                        .unwrap();
                    match read_wasm_file(file_path) {
                        Ok(data) => data,
                        Err(err) => {
                            terminal.fg(term::color::RED).unwrap();
                            println!(" error");
                            terminal.reset().unwrap();
                            return Err(String::from(err.description()));
                        }
                    }
                }
                None | Some(&_) => {
                    terminal.fg(term::color::RED).unwrap();
                    println!(" error");
                    terminal.reset().unwrap();
                    return Err(String::from("the file extension is not wasm or wast"));
                }
            }
        }
    };
    let mut dummy_runtime = DummyRuntime::new();
    let mut standalone_runtime = StandaloneRuntime::new();
    let translation = {
        let mut runtime: &mut WasmRuntime = if args.flag_execute {
            &mut standalone_runtime
        } else {
            &mut dummy_runtime
        };
        match translate_module(&data, runtime) {
            Ok(x) => x,
            Err(string) => {
                terminal.fg(term::color::RED).unwrap();
                println!(" error");
                terminal.reset().unwrap();
                return Err(string);
            }
        }
    };
    if args.flag_verbose {
        println!();
        let mut writer1 = stdout();
        let mut writer2 = stdout();
        match pretty_print_translation(&name, &data, &translation, &mut writer1, &mut writer2) {
            Err(error) => return Err(String::from(error.description())),
            Ok(()) => {
                terminal.fg(term::color::GREEN).unwrap();
                println!("ok");
                terminal.reset().unwrap();
            }
        }
    } else {
        terminal.fg(term::color::GREEN).unwrap();
        println!(" ok");
        terminal.reset().unwrap();
    }
    if args.flag_execute {
        terminal.fg(term::color::YELLOW).unwrap();
        println!("Compiling and executing module...");
        terminal.reset().unwrap();
        match execute_module(&translation) {
            Ok(()) => {
                terminal.fg(term::color::GREEN).unwrap();
                println!("ok");
                terminal.reset().unwrap();
            }
            Err(s) => {
                terminal.fg(term::color::RED).unwrap();
                println!("error");
                terminal.reset().unwrap();
                return Err(s);
            }
        };
        if args.flag_memory {
            let mut input = String::new();
            terminal.fg(term::color::YELLOW).unwrap();
            println!("Inspecting memory");
            terminal.fg(term::color::MAGENTA).unwrap();
            println!("Type 'quit' to exit.");
            terminal.reset().unwrap();
            loop {
                input.clear();
                terminal.fg(term::color::YELLOW).unwrap();
                print!("Memory index, offset, length (e.g. 0,0,4): ");
                terminal.reset().unwrap();
                let _ = stdout().flush();
                match io::stdin().read_line(&mut input) {
                    Ok(_) => {
                        input.pop();
                        if input == "quit" {
                            break;
                        }
                        let split: Vec<&str> = input.split(",").collect();
                        if split.len() != 3 {
                            break;
                        }
                        let memory = standalone_runtime
                            .inspect_memory(str::parse(split[0]).unwrap(),
                                            str::parse(split[1]).unwrap(),
                                            str::parse(split[2]).unwrap());
                        let mut s = memory
                            .iter()
                            .fold(String::from("#"), |mut acc, byte| {
                                acc.push_str(format!("{:02x}_", byte).as_str());
                                acc
                            });
                        s.pop();
                        println!("{}", s);
                    }
                    Err(error) => return Err(String::from(error.description())),
                }
            }
        }
    }
    Ok(())
}

// Prints out a Wasm module, and for each function the corresponding translation in Cretonne IL.
fn pretty_print_translation(filename: &String,
                            data: &Vec<u8>,
                            translation: &TranslationResult,
                            writer_wast: &mut Write,
                            writer_cretonne: &mut Write)
                            -> Result<(), io::Error> {
    let mut terminal = term::stdout().unwrap();
    let mut parser = Parser::new(data.as_slice());
    let mut parser_writer = Writer::new(writer_wast);
    let imports_count = translation
        .functions
        .iter()
        .fold(0, |acc, &ref f| match f {
            &FunctionTranslation::Import() => acc + 1,
            &FunctionTranslation::Code { .. } => acc,
        });
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
        let mut function_string =
            format!("  {}",
                    match translation.functions[function_index + imports_count] {
                            FunctionTranslation::Code { ref il, .. } => il,
                            FunctionTranslation::Import() => panic!("should not happen"),
                        }
                        .display(None));
        function_string.pop();
        let function_str = str::replace(function_string.as_str(), "\n", "\n  ");
        terminal.fg(term::color::CYAN).unwrap();
        write!(writer_cretonne, "Cretonne IL --->\n")?;
        terminal.reset().unwrap();
        write!(writer_cretonne, "{}\n", function_str)?;
        function_index += 1;
    }
    loop {
        match parser.read() {
            &ParserState::EndWasm => return Ok(()),
            s @ _ => parser_writer.write(&s)?,
        }
    }
}
