extern crate wasm2cretonne;
extern crate wasmparser;

use wasm2cretonne::module_translator::translate_module;
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufReader, Error};
use std::io::prelude::*;

fn read_wasm_file(path: PathBuf) -> Result<Vec<u8>, Error> {
    let mut buf: Vec<u8> = Vec::new();
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(&mut buf)?;
    Ok(buf)
}


fn main() {
    let path = PathBuf::from("tests/if.wast.0.wasm");
    println!("Reading: {:?}", path.as_os_str());
    let data = match read_wasm_file(path) {
        Ok(data) => data,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };
    let funcs = match translate_module(data) {
        Ok(funcs) => funcs,
        Err(string) => {
            println!("{}", string);
            return;
        }
    };
    for func in funcs {
        println!("{}", func.display(None));
    }
}
