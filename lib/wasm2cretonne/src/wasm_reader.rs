use wasmparser::{Parser, ParserState};
use std::fs::File;
use std::path::PathBuf;
use std::io::{BufReader, Error};
use std::io::prelude::*;
use std::str::from_utf8;

pub fn read_wasm_file(path: PathBuf, buf: &mut Vec<u8>) -> Result<Parser, Error> {
    println!("Reading: {:?}", path.as_os_str());
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_end(buf)?;
    Ok(Parser::new(buf.as_slice()))
}

fn get_name(bytes: &[u8]) -> &str {
    from_utf8(bytes).ok().unwrap()
}

pub fn parser_loop(parser: &mut Parser) {
    loop {
        let state = parser.read();
        match *state {
            ParserState::BeginWasm { .. } => {
                println!("====== Module");
            }
            ParserState::ExportSectionEntry { field, ref kind, .. } => {
                println!("  Export {} {:?}", get_name(field), kind);
            }
            ParserState::ImportSectionEntry { module, field, .. } => {
                println!("  Import {}::{}", get_name(module), get_name(field))
            }
            ParserState::EndWasm => break,
            _ => println!(" Other {:?}", state),
        }
    }
}
