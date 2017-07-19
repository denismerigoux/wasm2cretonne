extern crate cretonne;
extern crate wasm2cretonne;
extern crate cton_frontend;
extern crate byteorder;
extern crate region;

mod execution;
mod standalone;

pub use execution::translate_and_execute_module;
pub use standalone::StandaloneRuntime;
