use cretonne::ir::{Function, Signature, Value, Type, InstBuilder, FunctionName, Ebb, FuncRef,
                   SigRef, ExtFuncData, Inst};
use cretonne::ir::types::*;
use cretonne::ir::immediates::{Ieee32, Ieee64};
use cretonne::verifier::verify_function;
use cretonne::ir::condcodes::{IntCC, FloatCC};
use cretonne::entity_ref::EntityRef;
use cretonne::ir::frontend::{ILBuilder, FunctionBuilder};
use wasmparser::{Parser, ParserState, Operator};
use translation_utils::{f32_translation, f64_translation, return_values_count};
use std::collections::HashMap;
use std::u32;

// An opaque reference to local variable in wasm.
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

enum ControlStackFrame {
    If {
        destination: Ebb,
        branch_inst: Inst,
        return_values_count: usize,
    },
    Block {
        destination: Ebb,
        return_values_count: usize,
    },
}

struct TranslationState {
    last_inst_return: bool,
    unreachable: bool,
}

/// Returns a well-formed Cretonne IL function from a wasm function body and a signature.
pub fn translate_function_body(parser: &mut Parser,
                               function_index: u32,
                               sig: Signature,
                               locals: &Vec<(u32, Type)>,
                               exports: &Option<HashMap<u32, String>>,
                               signatures: &Vec<Signature>,
                               functions: &Vec<u32>,
                               il_builder: &mut ILBuilder<Local>)
                               -> Result<Function, String> {
    let mut func = Function::new();
    let args_num: usize = sig.argument_types.len();
    let args_types: Vec<Type> = sig.argument_types
        .iter()
        .map(|arg| arg.value_type)
        .collect();
    func.signature = sig;
    match exports {
        &None => (),
        &Some(ref exports) => {
            match exports.get(&function_index) {
                None => (),
                Some(name) => func.name = FunctionName::new(name.clone()),
            }
        }
    }
    // Declare external functions references
    for signature in signatures {
        func.dfg.signatures.push(signature.clone());
    }
    for (func_index, sig_index) in functions.iter().enumerate() {
        func.dfg
            .ext_funcs
            .push(ExtFuncData {
                      name: match exports {
                          &None => FunctionName::new(""),
                          &Some(ref exports) => {
                              match exports.get(&(func_index as u32)) {
                                  None => FunctionName::new(""),
                                  Some(name) => FunctionName::new(name.clone()),
                              }
                          }
                      },
                      signature: SigRef::new(*sig_index as usize),
                  });
    }
    let mut stack: Vec<Value> = Vec::new();
    let mut control_stack: Vec<ControlStackFrame> = Vec::new();
    {
        let mut builder = FunctionBuilder::new(&mut func, il_builder);
        let first_ebb = builder.create_ebb();
        builder.switch_to_block(first_ebb);
        builder.seal_block(first_ebb);
        for i in 0..args_num {
            // First we declare the function arguments' as non-SSA vars because they will be
            // accessed by get_local
            let arg_value = builder.arg_value(i as usize);
            builder.declare_var(Local(i as u32), args_types[i]);
            builder.def_var(Local(i as u32), arg_value);
        }
        // We also declare and initialize to 0 the local variables
        let mut local_index = args_num;
        for &(loc_count, ty) in locals {
            let val = match ty {
                I32 => builder.ins().iconst(ty, 0),
                I64 => builder.ins().iconst(ty, 0),
                F32 => builder.ins().f32const(Ieee32::new(0.0)),
                F64 => builder.ins().f64const(Ieee64::new(0.0)),
                _ => panic!("should not happen"),
            };
            for _ in 0..loc_count {
                builder.declare_var(Local(local_index as u32), ty);
                builder.def_var(Local(local_index as u32), val);
                local_index += 1;
            }
        }
        let mut state = TranslationState {
            last_inst_return: false,
            unreachable: false,
        };
        loop {
            let parser_state = parser.read();

            match *parser_state {
                ParserState::CodeOperator(ref op) => {
                    if state.unreachable {
                        match *op {
                            Operator::End => {
                                translate_operator(op,
                                                   &mut builder,
                                                   &mut stack,
                                                   &mut control_stack,
                                                   &mut state,
                                                   &functions,
                                                   &signatures)
                            }
                            Operator::Else => {
                                translate_operator(op,
                                                   &mut builder,
                                                   &mut stack,
                                                   &mut control_stack,
                                                   &mut state,
                                                   &functions,
                                                   &signatures)
                            }
                            _ => {
                                // We don't translate because the code is unreachable
                            }
                        }
                    } else {
                        translate_operator(op,
                                           &mut builder,
                                           &mut stack,
                                           &mut control_stack,
                                           &mut state,
                                           &functions,
                                           &signatures)
                    }
                }
                ParserState::EndFunctionBody => break,
                _ => return Err(String::from("wrong content in function body")),
            }
        }
        if !state.last_inst_return {
            builder.ins().return_(stack.as_slice());
        }
    }
    // TODO: remove the verification in production
    match verify_function(&func, None) {
        Ok(()) => {println!("{}", func.display(None));Ok(func)}
        Err(err) => {
            println!("{}", func.display(None));
            //Err(format!("{}: {}", err.location, err.message))
            println!("{}: {}", err.location, err.message);
            Ok(func)
        }
    }
}

/// Translates wasm operators into Cretonne IL instructions. Returns `true` if it inserted
/// a return.
fn translate_operator(op: &Operator,
                      builder: &mut FunctionBuilder<Local>,
                      stack: &mut Vec<Value>,
                      control_stack: &mut Vec<ControlStackFrame>,
                      state: &mut TranslationState,
                      functions: &Vec<u32>,
                      signatures: &Vec<Signature>) {
    state.last_inst_return = false;
    match *op {
        Operator::GetLocal { local_index } => stack.push(builder.use_var(Local(local_index))),
        Operator::SetLocal { local_index } => {
            let val = stack.pop().unwrap();
            builder.def_var(Local(local_index), val);
        }
        Operator::I32Const { value } => stack.push(builder.ins().iconst(I32, value as i64)),
        Operator::I64Const { value } => stack.push(builder.ins().iconst(I64, value)),
        Operator::I32Add => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().iadd(arg1, arg2));
        }
        Operator::I64Add => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().iadd(arg1, arg2));
        }
        Operator::F32Add => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fadd(arg1, arg2));
        }
        Operator::F64Add => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fadd(arg1, arg2));
        }
        Operator::I32Sub => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().isub(arg1, arg2));
        }
        Operator::I64Sub => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().isub(arg1, arg2));
        }
        Operator::F32Sub => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fsub(arg1, arg2));
        }
        Operator::F64Sub => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fsub(arg1, arg2));
        }
        Operator::I32Mul => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().imul(arg1, arg2));
        }
        Operator::I64Mul => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().imul(arg1, arg2));
        }
        Operator::F32Mul => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fmul(arg1, arg2));
        }
        Operator::F64Mul => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            stack.push(builder.ins().fmul(arg1, arg2));
        }
        Operator::I32LtS => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().icmp(IntCC::SignedLessThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::I32LtU => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().icmp(IntCC::UnsignedLessThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::I64LtS => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().icmp(IntCC::SignedLessThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::I64LtU => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().icmp(IntCC::UnsignedLessThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::I32Eqz => {
            let arg = stack.pop().unwrap();
            let val = builder.ins().icmp_imm(IntCC::Equal, arg, 0);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::I64Eqz => {
            let arg = stack.pop().unwrap();
            let val = builder.ins().icmp_imm(IntCC::Equal, arg, 0);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::F32Neg => {
            let arg = stack.pop().unwrap();
            stack.push(builder.ins().fneg(arg));
        }
        Operator::F64Neg => {
            let arg = stack.pop().unwrap();
            stack.push(builder.ins().fneg(arg));
        }
        Operator::F32Gt => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().fcmp(FloatCC::GreaterThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::F64Gt => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().fcmp(FloatCC::GreaterThan, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::F32Ge => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::F64Ge => {
            let arg1 = stack.pop().unwrap();
            let arg2 = stack.pop().unwrap();
            let val = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, arg1, arg2);
            stack.push(builder.ins().bint(I32, val));
        }
        Operator::Drop => {
            stack.pop();
        }
        Operator::F32Const { value } => {
            stack.push(builder.ins().f32const(f32_translation(value)));
        }
        Operator::F64Const { value } => {
            stack.push(builder.ins().f64const(f64_translation(value)));
        }
        Operator::F64ConvertUI64 => {
            let val = stack.pop().unwrap();
            stack.push(builder.ins().fcvt_from_uint(F64, val));
        }
        Operator::F64ConvertUI32 => {
            let val = stack.pop().unwrap();
            stack.push(builder.ins().fcvt_from_uint(F64, val));
        }
        Operator::F64ConvertSI64 => {
            let val = stack.pop().unwrap();
            stack.push(builder.ins().fcvt_from_sint(F64, val));
        }
        Operator::F64PromoteF32 => {
            let val = stack.pop().unwrap();
            stack.push(builder.ins().fpromote(F64, val));
        }
        Operator::F64ConvertSI32 => {
            let val = stack.pop().unwrap();
            stack.push(builder.ins().fcvt_from_sint(F64, val));
        }
        Operator::I32Ctz => {
            let val = stack.pop().unwrap();
            let short_res = builder.ins().ctz(val);
            stack.push(builder.ins().sextend(I32, short_res));

        }
        Operator::I64Ctz => {
            let val = stack.pop().unwrap();
            let short_res = builder.ins().ctz(val);
            stack.push(builder.ins().sextend(I32, short_res));
        }
        Operator::Return => {
            builder.ins().return_(stack.as_slice());
            stack.clear();
            state.last_inst_return = true;
        }
        Operator::Block { ty } => {
            let next = builder.create_ebb();
            control_stack.push(ControlStackFrame::Block {
                                   destination: next,
                                   return_values_count: return_values_count(ty),
                               });
        }
        Operator::If { ty } => {
            let val = stack.pop().unwrap();
            let if_not = builder.create_ebb();
            let jump_inst = builder.ins().brz(val, if_not, &[]);
            control_stack.push(ControlStackFrame::If {
                                   destination: if_not,
                                   branch_inst: jump_inst,
                                   return_values_count: return_values_count(ty),
                               });
        }
        Operator::Else => {
            // We take the control frame pushed by the if, use its ebb as the else body
            // and push a new control frame with a new ebb for the code after the if/then/else
            let (destination, return_values_count, branch_inst) = match &control_stack[control_stack.len() -
                                                                         1] {
                &ControlStackFrame::If {
                    destination,
                    return_values_count,
                    branch_inst,
                } => (destination, return_values_count, branch_inst),
                _ => panic!("should not happen"),
            };
            // At the end of the then clause we jump to the destination
            if state.unreachable {
                state.unreachable = false;
            } else {
                let cut_index = stack.len() - return_values_count;
                let jump_args = stack.split_off(cut_index);
                builder.ins().jump(destination, jump_args.as_slice());
            }
            // We change the target of the branch instruction
            let else_ebb = builder.create_ebb();
            builder.change_jump_destination(branch_inst, else_ebb);
            builder.seal_block(else_ebb);
            builder.switch_to_block(else_ebb);
        }
        Operator::End => {
            let (destination, return_values_count) = match control_stack.pop().unwrap() {
                ControlStackFrame::If {
                    destination,
                    return_values_count,
                    ..
                } => (destination, return_values_count),
                ControlStackFrame::Block {
                    destination,
                    return_values_count,
                    ..
                } => (destination, return_values_count),
            };
            if state.unreachable {
                state.unreachable = false;
            } else {
                let cut_index = stack.len() - return_values_count;
                let jump_args = stack.split_off(cut_index);
                builder.ins().jump(destination, jump_args.as_slice());
            }
            builder.seal_block(destination);
            builder.switch_to_block(destination);
            stack.extend_from_slice(builder.ebb_args(destination));
        }
        Operator::Br { relative_depth } => {
            let (destination, return_values_count) =
                match control_stack[control_stack.len() - 1 - (relative_depth as usize)] {
                    ControlStackFrame::If {
                        destination,
                        return_values_count,
                        ..
                    } => (destination, return_values_count),
                    ControlStackFrame::Block {
                        destination,
                        return_values_count,
                        ..
                    } => (destination, return_values_count),
                };
            let cut_index = stack.len() - return_values_count;
            let jump_args = stack.split_off(cut_index);
            builder.ins().jump(destination, jump_args.as_slice());
            // We signal that all the code that follows until the next End is unreachable
            state.unreachable = true;
        }
        Operator::BrIf { relative_depth } => {
            let val = stack.pop().unwrap();
            let (destination, return_values_count) =
                match control_stack[control_stack.len() - 1 - (relative_depth as usize)] {
                    ControlStackFrame::If {
                        destination,
                        return_values_count,
                        ..
                    } => (destination, return_values_count),
                    ControlStackFrame::Block {
                        destination,
                        return_values_count,
                        ..
                    } => (destination, return_values_count),
                };
            let cut_index = stack.len() - return_values_count;
            let jump_args = stack.split_off(cut_index);
            builder.ins().brnz(val, destination, jump_args.as_slice());
        }
        Operator::BrTable { ref table } => {
            // TODO: deal with jump arguments by splitting edges
            let (depths, default) = table.read_table();
            let jt = builder.create_jump_table();
            for (index, depth) in depths.iter().enumerate() {
                let ebb = match control_stack[control_stack.len() - 1 - (*depth as usize)] {
                    ControlStackFrame::If { destination, .. } => destination,
                    ControlStackFrame::Block { destination, .. } => destination,
                };
                builder.insert_jump_table_entry(jt, index, ebb);
            }
            let val = stack.pop().unwrap();
            builder.ins().br_table(val, jt);
            let ebb = match control_stack[control_stack.len() - 1 - (default as usize)] {
                ControlStackFrame::If { destination, .. } => destination,
                ControlStackFrame::Block { destination, .. } => destination,
            };
            builder.ins().jump(ebb, &[]);
            state.unreachable = true;
        }
        Operator::Nop => {
            // We do nothing
        }
        Operator::Unreachable => {
            builder.ins().trap();
            state.unreachable = true;
        }
        Operator::Call { function_index } => {
            let args_num = args_count(function_index as usize, functions, signatures);
            let cut_index = stack.len() - args_num;
            let call_args = stack.split_off(cut_index);
            let call_inst = builder
                .ins()
                .call(FuncRef::new(function_index as usize), call_args.as_slice());
            let ret_values = builder.inst_results(call_inst);
            for val in ret_values {
                stack.push(*val);
            }
        }
        _ => unimplemented!(),
    }
}

fn args_count(index: usize, functions: &Vec<u32>, signatures: &Vec<Signature>) -> usize {
    signatures[functions[index] as usize].argument_types.len()
}
