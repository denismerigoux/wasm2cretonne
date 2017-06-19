extern crate wasm2cretonne;
extern crate wasmparser;

use wasm2cretonne::module_parser::parse_module_preamble;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("tests/int_exprs.wast.0.wasm");
    match parse_module_preamble(path) {
        Ok(_) => (),
        Err(string) => println!("{}", string),
    }
}
