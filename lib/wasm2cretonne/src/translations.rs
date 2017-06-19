use wasmparser;
use cretonne;

pub fn type_to_type(ty: &wasmparser::Type) -> Result<cretonne::ir::Type, ()> {
    match *ty {
        wasmparser::Type::I32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::I64 => Ok(cretonne::ir::types::I64),
        wasmparser::Type::F32 => Ok(cretonne::ir::types::I32),
        wasmparser::Type::F64 => Ok(cretonne::ir::types::F64),
        _ => Err(()),
    }
}
