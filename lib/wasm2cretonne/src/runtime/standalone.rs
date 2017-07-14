use runtime::runtime::{Global, GlobalInit, Table, Memory, WasmRuntime};
use translation_utils::{Local, FunctionIndex, GlobalIndex, TableIndex, RawByte, Address};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{MemFlags, Value, InstBuilder, SigRef, FuncRef, ExtFuncData, FunctionName,
                   Signature, ArgumentType};
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

struct GlobalInfo {
    global: Global,
    offset: usize,
}

struct GlobalsData {
    data: Vec<RawByte>,
    info: Vec<GlobalInfo>,
}

struct TablesData {
    addresses: Vec<Address>,
    elements: Vec<TableElement>,
    info: Table,
}

struct MemoryData {
    data: Vec<RawByte>,
    info: Memory,
}

const PAGE_SIZE: usize = 65536;

pub struct StandaloneRuntime {
    globals: GlobalsData,
    tables: Vec<TablesData>,
    memories: Vec<MemoryData>,
    instantiated: bool,
    has_current_memory: Option<FuncRef>,
    has_grow_memory: Option<FuncRef>,
}

impl StandaloneRuntime {
    /// Allocates the runtime data structures
    pub fn new() -> StandaloneRuntime {
        StandaloneRuntime {
            globals: GlobalsData {
                data: Vec::new(),
                info: Vec::new(),
            },
            tables: Vec::new(),
            memories: Vec::new(),
            instantiated: false,
            has_current_memory: None,
            has_grow_memory: None,
        }
    }
}

impl WasmRuntime for StandaloneRuntime {
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex)
                            -> Value {
        debug_assert!(self.instantiated);
        let ty = self.globals.info[global_index as usize].global.ty;
        let offset = self.globals.info[global_index as usize].offset;
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals.data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().load(ty, memflags, addr_val, memoffset)
    }
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex,
                            val: Value) {
        let offset = self.globals.info[global_index as usize].offset;
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals.data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().store(memflags, val, addr_val, memoffset);
    }
    fn translate_grow_memory(&mut self,
                             builder: &mut FunctionBuilder<Local>,
                             pages: Value)
                             -> Value {
        debug_assert!(self.instantiated);
        let grow_mem_func = match self.has_grow_memory {
            Some(grow_mem_func) => grow_mem_func,
            None => {
                let sig_ref =
                    builder.import_signature(Signature {
                                                 argument_bytes: None,
                                                 argument_types: vec![ArgumentType::new(I32)],
                                                 return_types: vec![ArgumentType::new(I32)],
                                             });
                builder.import_function(ExtFuncData {
                                            name: FunctionName::new("current_memory"),
                                            signature: sig_ref,
                                        })
            }
        };
        self.has_grow_memory = Some(grow_mem_func);
        let call_inst = builder.ins().call(grow_mem_func, &[pages]);
        *builder.inst_results(call_inst).first().unwrap()
    }
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value {
        debug_assert!(self.instantiated);
        let cur_mem_func = match self.has_current_memory {
            Some(cur_mem_func) => cur_mem_func,
            None => {
                let sig_ref = builder.import_signature(Signature {
                                                           argument_bytes: None,
                                                           argument_types: Vec::new(),
                                                           return_types:
                                                               vec![ArgumentType::new(I32)],
                                                       });
                builder.import_function(ExtFuncData {
                                            name: FunctionName::new("current_memory"),
                                            signature: sig_ref,
                                        })
            }
        };
        self.has_current_memory = Some(cur_mem_func);
        let call_inst = builder.ins().call(cur_mem_func, &[]);
        *builder.inst_results(call_inst).first().unwrap()
    }
    fn translate_call_indirect<'a>(&self,
                                   builder: &'a mut FunctionBuilder<Local>,
                                   sig_ref: SigRef,
                                   index_val: Value,
                                   call_args: &[Value])
                                   -> &'a [Value] {
        let trap_ebb = builder.create_ebb();
        let continue_ebb = builder.create_ebb();
        let size_val = builder.ins().iconst(I32, self.tables[0].info.size as i64);
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
        let base_table_addr: i64 = unsafe { transmute(self.tables[0].addresses.as_ptr()) };
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
        for globalinfo in self.globals.info.iter_mut() {
            globalinfo.offset = globals_data_size;
            globals_data_size += globalinfo.global.ty.bytes() as usize;
        }
        self.globals.data.resize(globals_data_size as usize, 0);
        for globalinfo in self.globals.info.iter() {
            match globalinfo.global.initializer {
                GlobalInit::I32Const(val) => {
                    self.globals
                        .data
                        .as_mut_slice()
                        .split_at_mut(globalinfo.offset as usize)
                        .1
                        .write_i32::<LittleEndian>(val)
                        .unwrap();
                }
                GlobalInit::I64Const(val) => {
                    self.globals
                        .data
                        .as_mut_slice()
                        .split_at_mut(globalinfo.offset as usize)
                        .1
                        .write_i64::<LittleEndian>(val)
                        .unwrap();
                }
                GlobalInit::F32Const(val) => {
                    self.globals
                        .data
                        .as_mut_slice()
                        .split_at_mut(globalinfo.offset as usize)
                        .1
                        .write_f32::<LittleEndian>(unsafe { transmute(val) })
                        .unwrap();
                }
                GlobalInit::F64Const(val) => {
                    self.globals
                        .data
                        .as_mut_slice()
                        .split_at_mut(globalinfo.offset as usize)
                        .1
                        .write_f64::<LittleEndian>(unsafe { transmute(val) })
                        .unwrap();
                }
                GlobalInit::Import() => {
                    // We don't initialize, this is inter-module linking
                    // TODO: support inter-module imports
                }
                GlobalInit::ImportRef(index) => {
                    let ref_offset = self.globals.info[index].offset;
                    let size = globalinfo.global.ty.bytes();
                    unsafe {
                        let dst = self.globals
                            .data
                            .as_mut_ptr()
                            .offset(globalinfo.offset as isize);
                        let src = self.globals.data.as_ptr().offset(ref_offset as isize);
                        copy_nonoverlapping(src, dst, size as usize)
                    }
                }
            }
        }
        // Instantiating the tables
        for table in self.tables.iter_mut() {
            // TODO: link with the actual adresses of the functions
            table.addresses.resize(table.info.size as usize, 0);
        }
        // Instantiating the memory
        for memory in self.memories.iter_mut() {
            memory
                .data
                .resize((memory.info.size as usize) * PAGE_SIZE, 0);
        }
    }
    fn next_function(&mut self) {
        self.has_current_memory = None;
        self.has_grow_memory = None;
    }
    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        self.globals
            .info
            .push(GlobalInfo {
                      global: global,
                      offset: 0,
                  });
    }
    fn declare_table(&mut self, table: Table) {
        debug_assert!(!self.instantiated);
        let mut elements_vec = Vec::with_capacity(table.size as usize);
        elements_vec.resize(table.size as usize, TableElement::Trap());
        self.tables
            .push(TablesData {
                      info: table,
                      addresses: Vec::with_capacity(table.size as usize),
                      elements: elements_vec,
                  });
    }
    fn declare_table_elements(&mut self,
                              table_index: TableIndex,
                              offset: usize,
                              elements: &[FunctionIndex]) {
        debug_assert!(!self.instantiated);
        for (i, elt) in elements.iter().enumerate() {
            self.tables[table_index].elements[offset as usize + i] = TableElement::Function(*elt);
        }
    }
    fn declare_memory(&mut self, memory: Memory) {
        debug_assert!(!self.instantiated);
        self.memories
            .push(MemoryData {
                      info: memory,
                      data: Vec::with_capacity(memory.size as usize),
                  });
    }
}
