use cretonne::Context;
use cretonne::settings;
use cretonne::isa;
use cretonne::ir::{Ebb, FuncRef, JumpTable};
use cretonne::binemit::{RelocSink, Reloc, CodeOffset};
use wasm2cretonne::translate_module;
use standalone::StandaloneRuntime;
use std::mem::transmute;
use region::Protection;
use region::protect;

macro_rules! transmute_sig {
    ($addr: expr; [$($arg:ty),*] ;$ret:ty) => {
        transmute::< _,fn($($arg,)*) -> $ret>($addr)
    };
    ($addr: expr; [$($arg:ty),*]) => {
        transmute::< _,fn($($arg,)*)>($addr)
    }
}

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

pub fn translate_and_execute_module(data: &Vec<u8>) -> Result<(), String> {
    let mut runtime = StandaloneRuntime::new();
    let functions_and_import = match translate_module(data, &mut runtime) {
        Ok(funcs) => funcs,
        Err(string) => {
            return Err(string);
        }
    };
    let shared_builder = settings::builder();
    let shared_flags = settings::Flags::new(&shared_builder);
    let isa = match isa::lookup("intel") {
        None => {
            panic!() // The Intel target ISA is not available.
        }
        Some(isa_builder) => isa_builder.finish(shared_flags),
    };
    for (func, _) in functions_and_import {
        let mut context = Context::new();
        context.func = func;
        let code_size = context.compile(&*isa).unwrap() as usize;
        let mut code_buf: Vec<u8> = Vec::with_capacity(code_size);
        code_buf.resize(code_size, 0);
        let mut relocsink = DummyRelocSink {};
        context.emit_to_memory(code_buf.as_mut_ptr(), &mut relocsink, &*isa);
        execute(&mut code_buf);
    }
    Ok(())
}

fn execute(code_buf: &mut Vec<u8>) {
    unsafe {
        protect(code_buf.as_ptr(),
                code_buf.len(),
                Protection::ReadWriteExecute)
                .unwrap();
        let func = transmute_sig!(code_buf.as_ptr(); [i32, i32] ; i32);
        let arg1 = 5;
        let arg2 = 7;
        println!("Result of add({},{}): {}", arg1, arg2, func(arg1, arg2));
    }
}
