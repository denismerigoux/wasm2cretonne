extern crate cretonne;
extern crate wasm2cretonne;
extern crate cton_frontend;
extern crate region;

mod execution;
mod standalone;

pub use execution::execute_module;
pub use standalone::StandaloneRuntime;
