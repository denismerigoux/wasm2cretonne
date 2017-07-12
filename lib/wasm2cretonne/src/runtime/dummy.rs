use runtime::{Global, Table, WasmRuntime};
use translation_utils::Local;
use cton_frontend::FunctionBuilder;
use cretonne::ir::{Value, InstBuilder};
use cretonne::ir::immediates::{Ieee32, Ieee64};
use cretonne::ir::types::*;


pub struct DummyRuntime {
    globals: Vec<Global>,
    tables: Vec<Table>,
}

impl DummyRuntime {
    /// Allocates the runtime data structures
    pub fn new() -> DummyRuntime {
        DummyRuntime {
            globals: Vec::new(),
            tables: Vec::new(),
        }
    }
}

impl WasmRuntime for DummyRuntime {
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: u32)
                            -> Value {
        let ref glob = self.globals.get(global_index as usize).unwrap();
        match glob.ty {
            I32 => builder.ins().iconst(glob.ty, -1),
            I64 => builder.ins().iconst(glob.ty, -1),
            F32 => builder.ins().f32const(Ieee32::new(-1.0)),
            F64 => builder.ins().f64const(Ieee64::new(-1.0)),
            _ => panic!("should not happen"),
        }
    }

    fn translate_set_global(&self, _: &mut FunctionBuilder<Local>, _: u32, _: Value) {
        // We do nothing
    }
    fn translate_grow_memory(&self, _: &mut FunctionBuilder<Local>, _: Value) {
        // We do nothing
    }
    fn translate_current_memory(&self, builder: &mut FunctionBuilder<Local>) -> Value {
        builder.ins().iconst(I32, -1)
    }
    fn declare_global(&mut self, global: Global) {
        self.globals.push(global);
    }

    fn declare_table(&mut self, table: Table) {
        self.tables.push(table);
    }
}
