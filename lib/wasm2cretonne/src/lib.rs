extern crate wasmparser;
extern crate cton_frontend;
extern crate cretonne;

pub mod module_translator;

mod translation_utils;
mod code_translator;
mod sections_translator;

/// Version number of the cretonne crate.
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
