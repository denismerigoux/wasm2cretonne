use cretonne::Context;
use cretonne::settings;
use cretonne::isa;
use cretonne::write_function;
use wasm2cretonne::translate_module;
use standalone::StandaloneRuntime;

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
        let code_size = context.compile(&*isa).unwrap();
        let mut buf = String::new();
        write_function(&mut buf, &context.func, Some(&*isa)).unwrap();
        println!("Code size: {}, code:\n{}", code_size, buf);
    }
    Ok(())
}
