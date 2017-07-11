//! All the runtime support necessary for the wasm -> cretonne translation is formalized by the
//! trait `WasmRuntime`.
use cton_frontend::FunctionBuilder;
use cretonne::ir::{Value, Type};
use cretonne::entity_ref::EntityRef;
use std::hash::Hash;


/// Struct that models Wasm globals
#[derive(Debug,Clone,Copy)]
pub struct Global {
    pub ty: Type,
    pub mutability: bool,
}

pub trait WasmRuntime<Variable>
    where Variable: EntityRef + Hash + Default
{
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Variable>,
                            global_index: u32)
                            -> Value;
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Variable>,
                            global_index: u32,
                            val: Value);
    fn translate_grow_memory(&self, builder: &mut FunctionBuilder<Variable>, val: Value);
    fn translate_current_memory(&self, builder: &mut FunctionBuilder<Variable>) -> Value;

    fn declare_global(&mut self, global: Global);
}
