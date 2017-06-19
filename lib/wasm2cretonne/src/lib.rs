extern crate wasmparser;
extern crate cretonne;

pub mod module_parser;

mod translations;
mod wasm_reader;
mod sections_parser;

/// Version number of the cretonne crate.
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
