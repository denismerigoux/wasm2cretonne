use cretonne::ir::{Function, Signature};
use cretonne::entity_map::EntityRef;
use cretonne::ir::frontend::{ILBuilder, FunctionBuilder};
use wasmparser::{Parser, ParserState, Operator};
use sections_translator::Import;
use std::u32;

// An opaque reference to variable.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Local(u32);
impl EntityRef for Local {
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

pub fn translate_function_body(parser: &mut Parser,
                               sig: Signature,
                               imports: &Option<Vec<Import>>,
                               il_builder: &mut ILBuilder<Local>)
                               -> Result<Function, String> {
    let mut func = Function::new();
    func.signature = sig;
    {
        let mut builder = FunctionBuilder::new(&mut func, il_builder);
        loop {
            let state = parser.read();
            match *state {
                ParserState::CodeOperator(ref op) => translate_operator(op, &mut builder, imports),
                ParserState::EndFunctionBody => break,
                _ => return Err(String::from("wrong content in function body")),
            }
        }
    }
    Ok(func)
}

fn translate_operator(op: &Operator,
                      builder: &mut FunctionBuilder<Local>,
                      imports: &Option<Vec<Import>>) {
    unimplemented!()
}
