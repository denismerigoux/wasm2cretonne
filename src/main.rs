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
    let files = vec!["tests/br_if.wast.0.wasm",
                     //"tests/loop.wast.0.wasm",
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
        let _ = match translate_module(data) {
            Ok(funcs) => funcs,
            Err(string) => {
                println!("Error : {}", string);
                return;
            }
        };
    }
    // for func in funcs {
    //     println!("{}", func.display(None));
    // }
}
