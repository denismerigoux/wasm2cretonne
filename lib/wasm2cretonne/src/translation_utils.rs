use wasmparser;
use cretonne;
use std::mem;
use std::u32;
use runtime::{Global, Memory};

/// Struct that models Wasm imports
#[derive(Debug,Clone,Copy)]
pub enum Import {
    Function { sig_index: u32 },
    Memory(Memory),
    Global(Global),
}

// An opaque reference to local variable in wasm.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Local(pub u32);
impl cretonne::entity_ref::EntityRef for Local {
    fn new(index: usize) -> Self {
        assert!(index < (u32::MAX as usize));
        Local(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}
impl Default for Local {
    fn default() -> Local {
        Local(u32::MAX)
    }
}

/// Helper function translating wasmparser types to Cretonne types when possible.
pub fn type_to_type(ty: &wasmparser::Type) -> Result<cretonne::ir::Type, ()> {
    match *ty {
        wasmparser::Type::I32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::I64 => Ok(cretonne::ir::types::I64),
        wasmparser::Type::F32 => Ok(cretonne::ir::types::F32),
        wasmparser::Type::F64 => Ok(cretonne::ir::types::F64),
        _ => Err(()),
    }
}

/// Converts between the two types
pub fn f32_translation(x: wasmparser::Ieee32) -> cretonne::ir::immediates::Ieee32 {
    cretonne::ir::immediates::Ieee32::new(unsafe { mem::transmute(x.bits()) })
}

pub fn f64_translation(x: wasmparser::Ieee64) -> cretonne::ir::immediates::Ieee64 {
    cretonne::ir::immediates::Ieee64::new(unsafe { mem::transmute(x.bits()) })
}

pub fn return_values_types(ty: wasmparser::Type) -> Result<Vec<cretonne::ir::Type>, ()> {
    match ty {
        wasmparser::Type::EmptyBlockType => Ok(Vec::new()),
        wasmparser::Type::I32 => Ok(vec![cretonne::ir::types::I32]),
        wasmparser::Type::F32 => Ok(vec![cretonne::ir::types::F32]),
        wasmparser::Type::I64 => Ok(vec![cretonne::ir::types::I64]),
        wasmparser::Type::F64 => Ok(vec![cretonne::ir::types::F64]),
        _ => panic!("unsupported return value type"),
    }
}
