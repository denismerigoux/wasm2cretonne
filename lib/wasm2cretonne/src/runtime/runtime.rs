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
    /// Declares a global to the runtime.
    fn declare_global(&mut self, global: Global);
    /// Declares a table to the runtime.
    fn declare_table(&mut self, table: Table);
    /// Fills a declared table with references to functions in the module.
    fn declare_table_elements(&mut self,
                              table_index: TableIndex,
                              offset: usize,
                              elements: &[FunctionIndex]);
    /// Declares a memory to the runtime
    fn declare_memory(&mut self, memory: Memory);
    /// Fills a declared memory with bytes at module instantiation.
    fn declare_data_initialization(&mut self,
                                   memory_index: MemoryIndex,
                                   offset: usize,
                                   data: &[u8])
                                   -> Result<(), String>;
    /// Call this function after having declared all the runtime elements but prior to the
    /// function body translation.
    fn begin_translation(&mut self);
    /// Call this function between each function body translation.
    fn next_function(&mut self);
    /// Translates a `get_global` wasm instruction.
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex)
                            -> Value;
    /// Translates a `set_global` wasm instruction.
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex,
                            val: Value);
    /// Translates a `grow_memory` wasm instruction.
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, val: Value) -> Value;
    /// Translates a `current_memory` wasm instruction.
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value;
    /// Returns the ase address of a wasm memory as a Cretonne `Value`.
    fn translate_memory_base_adress(&self,
                                    builder: &mut FunctionBuilder<Local>,
                                    index: MemoryIndex)
                                    -> Value;
    /// Translates a `call_indirect` wasm instruction.
    fn translate_call_indirect<'a>(&self,
                                   builder: &'a mut FunctionBuilder<Local>,
                                   sig_ref: SigRef,
                                   index_val: Value,
                                   call_args: &[Value])
                                   -> &'a [Value];
}
