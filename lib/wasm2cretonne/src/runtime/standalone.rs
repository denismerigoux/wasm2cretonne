use runtime::runtime::{Global, GlobalInit, Table, Memory, WasmRuntime};
use translation_utils::{Local, FunctionIndex, GlobalIndex, TableIndex, RawByte, Address};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{MemFlags, Value, InstBuilder, SigRef};
use cretonne::ir::types::*;
use cretonne::ir::condcodes::IntCC;
use cretonne::ir::immediates::Offset32;
use byteorder::{LittleEndian, WriteBytesExt};
use std::mem::transmute;
use std::ptr::copy_nonoverlapping;

#[derive(Clone, Debug)]
enum TableElement {
    Trap(),
    Function(FunctionIndex),
}

const PAGE_SIZE: usize = 65536;

pub struct StandaloneRuntime {
    globals: Vec<Global>,
    globals_data: Vec<RawByte>,
    globals_offsets: Vec<usize>,
    tables: Vec<Table>,
    tables_data: Vec<Vec<Address>>,
    tables_elements: Vec<Vec<TableElement>>,
    memories: Vec<Memory>,
    memories_data: Vec<Vec<RawByte>>,
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
            tables_data: Vec::new(),
            tables_elements: Vec::new(),
            memories: Vec::new(),
            instantiated: false,
            memories_data: Vec::new(),
        }
    }
}

impl WasmRuntime for StandaloneRuntime {
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex)
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
                            global_index: GlobalIndex,
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
    fn translate_call_indirect<'a>(&self,
                                   builder: &'a mut FunctionBuilder<Local>,
                                   sig_ref: SigRef,
                                   index_val: Value,
                                   call_args: &[Value])
                                   -> &'a [Value] {
        let trap_ebb = builder.create_ebb();
        let continue_ebb = builder.create_ebb();
        let size_val = builder.ins().iconst(I32, self.tables[0].size as i64);
        let zero_val = builder.ins().iconst(I32, 0);
        builder
            .ins()
            .br_icmp(IntCC::UnsignedLessThan, index_val, zero_val, trap_ebb, &[]);
        builder
            .ins()
            .br_icmp(IntCC::UnsignedGreaterThanOrEqual,
                     index_val,
                     size_val,
                     trap_ebb,
                     &[]);
        builder.seal_block(trap_ebb);
        let offset_val = builder.ins().imul_imm(index_val, 4);
        let base_table_addr: i64 = unsafe { transmute(self.tables_data[0].as_ptr()) };
        let table_addr_val = builder.ins().iconst(I32, base_table_addr);
        let table_entry_addr_val = builder.ins().iadd(table_addr_val, offset_val);
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(0);
        let table_entry_val = builder
            .ins()
            .load(I32, memflags, table_entry_addr_val, memoffset);
        let call_inst = builder
            .ins()
            .call_indirect(sig_ref, table_entry_val, call_args);
        builder.ins().jump(continue_ebb, &[]);
        builder.seal_block(continue_ebb);
        builder.switch_to_block(trap_ebb, &[]);
        builder.ins().trap();
        builder.switch_to_block(continue_ebb, &[]);
        builder.inst_results(call_inst)
    }

    fn instantiate(&mut self) {
        debug_assert!(!self.instantiated);
        self.instantiated = true;
        // At instantiation, we allocate memory for the globals, the memories and the tables
        // First the globals
        let mut globals_data_size = 0;
        for global in self.globals.iter() {
            self.globals_offsets.push(globals_data_size);
            globals_data_size += global.ty.bytes() as usize;
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
        // Instantiating the tables
        for (i, table) in self.tables.iter().enumerate() {
            // TODO: link with the actual adresses of the functions
            self.tables_data[i].resize(table.size as usize, 0);
        }
        // Instantiating the memory
        for (i, memory) in self.memories.iter().enumerate() {
            self.memories_data[i].resize((memory.size as usize) * PAGE_SIZE, 0);
        }
    }
    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        self.globals.push(global);
    }
    fn declare_table(&mut self, table: Table) {
        debug_assert!(!self.instantiated);
        self.tables.push(table);
        let mut elements_vec = Vec::new();
        elements_vec.resize(table.size as usize, TableElement::Trap());
        self.tables_elements.push(elements_vec);
        self.tables_data
            .push(Vec::with_capacity(table.size as usize));
    }
    fn declare_table_elements(&mut self,
                              table_index: TableIndex,
                              offset: usize,
                              elements: &[FunctionIndex]) {
        debug_assert!(!self.instantiated);
        for (i, elt) in elements.iter().enumerate() {
            self.tables_elements[table_index as usize][offset as usize + i] =
                TableElement::Function(*elt);
        }
    }
    fn declare_memory(&mut self, memory: Memory) {
        debug_assert!(!self.instantiated);
        self.memories.push(memory);
        self.memories_data
            .push(Vec::with_capacity(memory.size as usize));
    }
}
