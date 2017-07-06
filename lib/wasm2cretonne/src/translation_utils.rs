use wasmparser;
use cretonne;
use std::mem;

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
