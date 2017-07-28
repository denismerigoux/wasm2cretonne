use cretonne::Context;
use cretonne::settings;
use cretonne::isa;
use cretonne::ir::{Ebb, FuncRef, JumpTable, Function};
use cretonne::binemit::{RelocSink, Reloc, CodeOffset};
use wasm2cretonne::{TranslationResult, FunctionTranslation, ImportMappings};
use std::mem::transmute;
use region::Protection;
use region::protect;
use std::collections::HashMap;
use std::ptr::write_unaligned;

type RelocRef = u16;

// Implementation of a relocation sink that just saves all the information for later
struct StandaloneRelocSink {
    ebbs: HashMap<RelocRef, (Ebb, CodeOffset)>,
    funcs: HashMap<RelocRef, (FuncRef, CodeOffset)>,
    jts: HashMap<RelocRef, (JumpTable, CodeOffset)>,
}

// Contains all the metadata necessary to perform relocations
enum FunctionMetaData {
    Import(),
    Local {
        relocs: StandaloneRelocSink,
        imports: ImportMappings,
        il_func: Function,
    },
}

impl RelocSink for StandaloneRelocSink {
    fn reloc_ebb(&mut self, offset: CodeOffset, reloc: Reloc, ebb: Ebb) {
        self.ebbs.insert(reloc.0, (ebb, offset));
    }
    fn reloc_func(&mut self, offset: CodeOffset, reloc: Reloc, func: FuncRef) {
        self.funcs.insert(reloc.0, (func, offset));
    }
    fn reloc_jt(&mut self, offset: CodeOffset, reloc: Reloc, jt: JumpTable) {
        self.jts.insert(reloc.0, (jt, offset));
    }
}

impl StandaloneRelocSink {
    fn new() -> StandaloneRelocSink {
        StandaloneRelocSink {
            ebbs: HashMap::new(),
            funcs: HashMap::new(),
            jts: HashMap::new(),
        }
    }
}

/// Executes a module that has been translated with the `StandaloneRuntime` runtime implementation.
/// Recognized ISAs are `"intel"`, `"riscv"`, `"arm32"`, `"arm64"`.
pub fn execute_module(trans_result: &TranslationResult, isa: &str) -> Result<(), String> {
    let shared_builder = settings::builder();
    let shared_flags = settings::Flags::new(&shared_builder);
    let isa = match isa::lookup(isa) {
        None => {
            panic!() // The Intel target ISA is not available.
        }
        Some(isa_builder) => isa_builder.finish(shared_flags),
    };
    let mut functions_metatada = Vec::new();
    let mut functions_code = Vec::new();
    for (function_index, function) in trans_result.functions.iter().enumerate() {
        let mut context = Context::new();
        let (il, imports) = match function {
            &FunctionTranslation::Import() => {
                if trans_result.start_index.is_some() &&
                   trans_result.start_index.unwrap() == function_index {
                    return Err(String::from("start function should not be an import"));
                } else {
                    functions_code.push(Vec::new());
                    functions_metatada.push(FunctionMetaData::Import());
                    continue;
                }
            }
            &FunctionTranslation::Code {
                ref il,
                ref imports,
                ..
            } => (il.clone(), imports.clone()),
        };
        context.func = il;
        let code_size = context.compile(&*isa).unwrap() as usize;
        if code_size == 0 {
            return Err(String::from("no code generated by Cretonne"));
        }
        let mut code_buf: Vec<u8> = Vec::with_capacity(code_size);
        code_buf.resize(code_size, 0);
        let mut relocsink = StandaloneRelocSink::new();
        context.emit_to_memory(code_buf.as_mut_ptr(), &mut relocsink, &*isa);
        functions_metatada.push(FunctionMetaData::Local {
                                    relocs: relocsink,
                                    imports: imports,
                                    il_func: context.func,
                                });
        functions_code.push(code_buf);
    }
    relocate(&functions_metatada, &mut functions_code);
    // After having emmitted the code to memory, we deal with relocations
    match trans_result.start_index {
        None => Err(String::from("No start function defined, aborting execution")),
        Some(index) => execute(&mut functions_code[index]),
    }
}

// Jumps to the code region of memory and execute the start function of the module.
fn execute(code_buf: &mut Vec<u8>) -> Result<(), String> {
    unsafe {
        match protect(code_buf.as_ptr(),
                      code_buf.len(),
                      Protection::ReadWriteExecute) {
            Ok(()) => (),
            Err(err) => {
                return Err(format!("failed to give executable permission to code: {}",
                                   err.description()))
            }
        };
        // Rather than writing inline assembly to jump to the code region, we use the fact that
        // the Rust ABI for calling a function with no arguments and no return matches the one of
        // the generated code.Thanks to this, we can transmute the code region into a first-class
        // Rust function and call it.
        // TODO: the Rust callee-saved registers will be overwritten by the executed code, inline
        // assembly spilling these registers to the stack and restoring them after the call is
        // needed.
        let start_func = transmute::<_, fn()>(code_buf.as_ptr());
        // The code below saves the Intel callee-saved registers. It is not activate because
        // inline ASM is not supported in the release version of the Rust compiler.
        /*asm!("push rax
              push rcx
              push rdx
              push rsi
              push rdi
              push r8
              push r9
              push r10
              push r11
        " :::: "intel", "volatile");*/
        start_func();
        /*asm!("pop r11
              pop r10
              pop r9
              pop r8
              pop rdi
              pop rsi
              pop rdx
              pop rcx
              pop rax
      " :::: "intel", "volatile");*/
        Ok(())
    }
}

/// Performs the relocations inside the function bytecode, provided the necessary metadata
fn relocate(functions_metatada: &Vec<FunctionMetaData>, functions_code: &mut Vec<Vec<u8>>) {
    // The relocations are relative to the relocation's address plus four bytes
    for (func_index, function_in_memory) in functions_metatada.iter().enumerate() {
        match function_in_memory {
            &FunctionMetaData::Import() => continue,
            &FunctionMetaData::Local {
                ref relocs,
                ref imports,
                ref il_func,
            } => {
                for (_, &(func_ref, offset)) in relocs.funcs.iter() {
                    let target_func_index = imports.functions[&func_ref];
                    let target_func_address: isize = functions_code[target_func_index].as_ptr() as
                                                     isize;
                    unsafe {
                        let reloc_address: isize = functions_code[func_index]
                            .as_mut_ptr()
                            .offset(offset as isize + 4) as
                                                   isize;
                        let reloc_delta_i32: i32 = (target_func_address - reloc_address) as i32;
                        write_unaligned(reloc_address as *mut i32, reloc_delta_i32);
                    }
                }
                for (_, &(ebb, offset)) in relocs.ebbs.iter() {
                    unsafe {
                        let reloc_address: isize = functions_code[func_index]
                            .as_mut_ptr()
                            .offset(offset as isize + 4) as
                                                   isize;
                        let target_ebb_address: isize =
                            functions_code[func_index]
                                .as_ptr()
                                .offset(il_func.offsets[ebb] as isize) as
                            isize;
                        let reloc_delta_i32: i32 = (target_ebb_address - reloc_address) as i32;
                        write_unaligned(reloc_address as *mut i32, reloc_delta_i32);
                    }
                }
                // TODO: deal with jumptable relocations
            }
        }
    }
}
