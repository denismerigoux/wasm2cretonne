//! All the runtime support necessary for the wasm -> cretonne translation is formalized by the
//! trait `WasmRuntime`.
use cton_frontend::FunctionBuilder;
use cretonne::ir::{Value, Type, SigRef};
use translation_utils::{Local, FunctionIndex, TableIndex, GlobalIndex, MemoryIndex};


/// Struct that models Wasm globals
#[derive(Debug,Clone,Copy)]
pub struct Global {
    pub ty: Type,
    pub mutability: bool,
    pub initializer: GlobalInit,
}

#[derive(Debug,Clone,Copy)]
pub enum GlobalInit {
    I32Const(i32),
    I64Const(i64),
    F32Const(u32),
    F64Const(u64),
    Import(),
    ImportRef(usize),
}

/// Struct that models Wasm tables
#[derive(Debug,Clone,Copy)]
pub struct Table {
    pub ty: TableElementType,
    pub size: usize,
    pub maximum: Option<usize>,
}

#[derive(Debug,Clone,Copy)]
pub enum TableElementType {
    Val(Type),
    Func(),
}

/// Struct that models the Wasm linear memory
#[derive(Debug,Clone,Copy)]
pub struct Memory {
    pub pages_count: usize,
    pub maximum: Option<usize>,
}

pub trait WasmRuntime {
    fn declare_global(&mut self, global: Global);
    fn declare_table(&mut self, table: Table);
    fn declare_table_elements(&mut self,
                              table_index: TableIndex,
                              offset: usize,
                              elements: &[FunctionIndex]);
    fn declare_memory(&mut self, memory: Memory);
    fn declare_data_initialization(&mut self,
                                   memory_index: MemoryIndex,
                                   offset: usize,
                                   data: &[u8])
                                   -> Result<(), String>;
    fn begin_translation(&mut self);
    fn next_function(&mut self);
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex)
                            -> Value;
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex,
                            val: Value);
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, val: Value) -> Value;
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value;
    fn translate_memory_base_adress(&self,
                                    builder: &mut FunctionBuilder<Local>,
                                    index: MemoryIndex)
                                    -> Value;
    fn translate_call_indirect<'a>(&self,
                                   builder: &'a mut FunctionBuilder<Local>,
                                   sig_ref: SigRef,
                                   index_val: Value,
                                   call_args: &[Value])
                                   -> &'a [Value];
}
