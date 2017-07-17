extern crate wasmparser;
extern crate cton_frontend;
extern crate cretonne;
extern crate byteorder;

mod module_translator;
mod translation_utils;
mod code_translator;
mod runtime;
mod sections_translator;

pub use module_translator::translate_module;
pub use runtime::{WasmRuntime, DummyRuntime, StandaloneRuntime};
pub use code_translator::FunctionImports;

/// Version number of the cretonne crate.
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
