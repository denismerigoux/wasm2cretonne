extern crate wasmparser;
extern crate cton_frontend;
extern crate cretonne;

mod module_translator;
mod translation_utils;
mod code_translator;
mod runtime;
mod sections_translator;

pub use module_translator::{translate_module, TranslationResult};
pub use module_translator::FunctionTranslation;
pub use runtime::{WasmRuntime, DummyRuntime, Global, GlobalInit, Table, Memory};
pub use code_translator::FunctionImports;
pub use translation_utils::{Local, FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, RawByte,
                            Address};
