use cretonne::Context;
use cretonne::settings;
use cretonne::isa;
use cretonne::ir::{Ebb, FuncRef, JumpTable};
use cretonne::binemit::{RelocSink, Reloc, CodeOffset};
use wasm2cretonne::{TranslationResult, FunctionTranslation};
use std::mem::transmute;
use region::Protection;
use region::protect;

struct DummyRelocSink {}

impl RelocSink for DummyRelocSink {
    fn reloc_ebb(&mut self, _: CodeOffset, _: Reloc, _: Ebb) {
        // We do nothing
    }
    fn reloc_func(&mut self, _: CodeOffset, _: Reloc, _: FuncRef) {
        // We do nothing
    }
    fn reloc_jt(&mut self, _: CodeOffset, _: Reloc, _: JumpTable) {
        // We do nothing
    }
}

/// Executes a module that has been translated with the `StandaloneRuntime` runtime implementation.
pub fn execute_module(trans_result: &TranslationResult) -> Result<(), String> {
    let shared_builder = settings::builder();
    let shared_flags = settings::Flags::new(&shared_builder);
    let isa = match isa::lookup("intel") {
        None => {
            panic!() // The Intel target ISA is not available.
        }
        Some(isa_builder) => isa_builder.finish(shared_flags),
    };
    match trans_result.start_index {
        None => println!("No start function defined, aborting execution"),
        Some(index) => {
            let mut context = Context::new();
            context.func = match trans_result.functions[index] {
                FunctionTranslation::Import() => panic!("start function should not be an import"),
                FunctionTranslation::Code { ref il, .. } => il.clone(),
            };
            let code_size = context.compile(&*isa).unwrap() as usize;
            let mut code_buf: Vec<u8> = Vec::with_capacity(code_size);
            code_buf.resize(code_size, 0);
            let mut relocsink = DummyRelocSink {};
            context.emit_to_memory(code_buf.as_mut_ptr(), &mut relocsink, &*isa);
            execute(&mut code_buf);
        }
    }
    Ok(())
}

fn execute(code_buf: &mut Vec<u8>) {
    unsafe {
        protect(code_buf.as_ptr(),
                code_buf.len(),
                Protection::ReadWriteExecute)
                .unwrap();
        let start_func = transmute::<_, fn()>(code_buf.as_ptr());
        start_func()
    }
}
