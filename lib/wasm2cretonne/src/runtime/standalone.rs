use runtime::runtime::{Global, GlobalInit, Table, Memory, WasmRuntime};
use translation_utils::Local;
use cton_frontend::FunctionBuilder;
use cretonne::ir::{MemFlags, Value, InstBuilder};
use cretonne::ir::types::*;
use cretonne::ir::immediates::Offset32;
use byteorder::{LittleEndian, WriteBytesExt};
use std::mem::transmute;
use std::ptr::copy_nonoverlapping;

pub struct StandaloneRuntime {
    globals: Vec<Global>,
    globals_data: Vec<u8>,
    globals_offsets: Vec<u32>,
    tables: Vec<Table>,
    memories: Vec<Memory>,
    memories_data: Vec<Vec<u8>>,
    instantiated: bool,
}

impl StandaloneRuntime {
    /// Allocates the runtime data structures
    pub fn new() -> StandaloneRuntime {
        StandaloneRuntime {
            globals: Vec::new(),
            globals_data: Vec::new(),
            globals_offsets: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            instantiated: false,
            memories_data: Vec::new(),
        }
    }
}

impl WasmRuntime for StandaloneRuntime {
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: u32)
                            -> Value {
        debug_assert!(self.instantiated);
        let ty = self.globals[global_index as usize].ty;
        let offset = self.globals_offsets[global_index as usize];
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals_data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().load(ty, memflags, addr_val, memoffset)
    }
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: u32,
                            val: Value) {
        let offset = self.globals_offsets[global_index as usize];
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals_data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().store(memflags, val, addr_val, memoffset);
    }
    fn translate_grow_memory(&self, _: &mut FunctionBuilder<Local>, _: Value) {
        debug_assert!(self.instantiated);
        unimplemented!()
    }
    fn translate_current_memory(&self, _: &mut FunctionBuilder<Local>) -> Value {
        debug_assert!(self.instantiated);
        unimplemented!()
    }
    fn instantiate(&mut self) {
        debug_assert!(!self.instantiated);
        self.instantiated = true;
        // At instantiation, we allocate memory for the globals and the memories
        let mut globals_data_size = 0;
        for global in self.globals.iter() {
            self.globals_offsets.push(globals_data_size);
            globals_data_size += global.ty.bytes();
        }
        self.globals_data.resize(globals_data_size as usize, 0);
        for (i, global) in self.globals.iter().enumerate() {
            let offset = self.globals_offsets[i];
            match global.initializer {
                GlobalInit::I32Const(val) => {
                    self.globals_data
                        .as_mut_slice()
                        .split_at_mut(offset as usize)
                        .1
                        .write_i32::<LittleEndian>(val)
                        .unwrap();
                }
                GlobalInit::I64Const(val) => {
                    self.globals_data
                        .as_mut_slice()
                        .split_at_mut(offset as usize)
                        .1
                        .write_i64::<LittleEndian>(val)
                        .unwrap();
                }
                GlobalInit::F32Const(val) => {
                    self.globals_data
                        .as_mut_slice()
                        .split_at_mut(offset as usize)
                        .1
                        .write_f32::<LittleEndian>(unsafe { transmute(val) })
                        .unwrap();
                }
                GlobalInit::F64Const(val) => {
                    self.globals_data
                        .as_mut_slice()
                        .split_at_mut(offset as usize)
                        .1
                        .write_f64::<LittleEndian>(unsafe { transmute(val) })
                        .unwrap();
                }
                GlobalInit::Import() => {
                    // We don't initialize, this is inter-module linking
                    // TODO: support inter-module imports
                }
                GlobalInit::ImportRef(index) => {
                    let ref_offset = self.globals_offsets[index];
                    let size = global.ty.bytes();
                    unsafe {
                        let dst = self.globals_data.as_mut_ptr().offset(offset as isize);
                        let src = self.globals_data.as_ptr().offset(ref_offset as isize);
                        copy_nonoverlapping(src, dst, size as usize)
                    }
                }
            }
        }
        println!("{:?}", self.globals_data);
    }
    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        self.globals.push(global);
    }
    fn declare_table(&mut self, table: Table) {
        debug_assert!(!self.instantiated);
        self.tables.push(table);
    }
    fn declare_memory(&mut self, memory: Memory) {
        debug_assert!(!self.instantiated);
        self.memories.push(memory);
        self.memories_data.push(Vec::new());
    }
}
