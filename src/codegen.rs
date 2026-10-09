use std::path::PathBuf;
use crate::lex::TokenType;
use crate::parse::{Expression};
use inkwell::builder::{Builder};
use inkwell::context::Context;
use inkwell::AddressSpace;
use inkwell::module::Module;
use inkwell::targets::{Target, TargetMachine, InitializationConfig, CodeModel, RelocMode, FileType};
use inkwell::values::BasicValueEnum;
use inkwell::values::IntValue;
use inkwell::IntPredicate;

fn build_int_pow<'ctx>(
    lhs: IntValue<'ctx>,
    rhs: IntValue<'ctx>,
    context: &'ctx Context,
    builder: &Builder<'ctx>,
) -> Result<IntValue<'ctx>, String> {
    let i64_type = context.i64_type();

    // The block where the operands have already been computed.
    let preheader = builder
        .get_insert_block()
        .ok_or("Builder has no insertion point")?;

    let function = preheader
        .get_parent()
        .ok_or("Current block has no parent function")?;

    // Create the loop and exit blocks.
    let loop_block = context.append_basic_block(function, "pow.loop");
    let body_block = context.append_basic_block(function, "pow.body");
    let exit_block = context.append_basic_block(function, "pow.exit");

    let zero = i64_type.const_int(0, false);
    let one = i64_type.const_int(1, false);

    // Enter the loop.
    builder
        .build_unconditional_branch(loop_block)
        .map_err(|e| e.to_string())?;

    builder.position_at_end(loop_block);

    // i = iteration counter; acc = accumulated product.
    let i_phi = builder
        .build_phi(i64_type, "pow.i")
        .map_err(|e| e.to_string())?;

    let acc_phi = builder
        .build_phi(i64_type, "pow.acc")
        .map_err(|e| e.to_string())?;

    i_phi.add_incoming(&[(&zero, preheader)]);
    acc_phi.add_incoming(&[(&one, preheader)]);

    let i = i_phi.as_basic_value().into_int_value();
    let acc = acc_phi.as_basic_value().into_int_value();

    // Continue while i < exponent.
    let condition = builder
        .build_int_compare(IntPredicate::SLT, i, rhs, "pow.cond")
        .map_err(|e| e.to_string())?;

    builder
        .build_conditional_branch(condition, body_block, exit_block)
        .map_err(|e| e.to_string())?;

    // acc *= base; i += 1.
    builder.position_at_end(body_block);

    let next_acc = builder
        .build_int_mul(acc, lhs, "pow.mul")
        .map_err(|e| e.to_string())?;

    let next_i = builder
        .build_int_add(i, one, "pow.next")
        .map_err(|e| e.to_string())?;

    builder
        .build_unconditional_branch(loop_block)
        .map_err(|e| e.to_string())?;

    // Complete the loop-carried PHI values.
    i_phi.add_incoming(&[(&next_i, body_block)]);
    acc_phi.add_incoming(&[(&next_acc, body_block)]);

    // The accumulated product is the result.
    builder.position_at_end(exit_block);

    Ok(acc_phi.as_basic_value().into_int_value())
}

pub enum OutputType {
    Obj(PathBuf), IR(PathBuf)
}

fn build_expr<'ctx>(
    expr: &Expression,
    context: &'ctx Context,
    builder: &Builder<'ctx>,
) -> Result<BasicValueEnum<'ctx>, String> {
    match expr {
        Expression::Atom(token) => {
            match &token.token_type {
                TokenType::IntLiteral(n) => {
                    Ok(context
                        .i64_type()
                        .const_int(*n as u64, true)
                        .into())
                }

                TokenType::FloatLiteral(n) => {
                    Ok(context
                        .f64_type()
                        .const_float(*n)
                        .into())
                }

                _ => Err(format!(
                    "Unexpected token in atom: {:?}",
                    token.token_type
                )),
            }
        }

        Expression::Operation(token, expressions) => {
            // Recursively compile all operands.
            let values: Result<Vec<_>, _> = expressions
                .iter()
                .map(|expr| build_expr(expr, context, builder))
                .collect();

            let values = values?;

            match &token.token_type {
                TokenType::Plus
                | TokenType::Minus
                | TokenType::Asterisk
                | TokenType::Slash
                | TokenType::Caret => {
                    if values.len() != 2 {
                        return Err(format!(
                            "Expected 2 operands for {:?}, got {}",
                            token.token_type,
                            values.len()
                        ));
                    }

                    let lhs = values[0];
                    let rhs = values[1];

                    // Integer arithmetic
                    if lhs.is_int_value() && rhs.is_int_value() {
                        let lhs = lhs.into_int_value();
                        let rhs = rhs.into_int_value();

                        let result = match &token.token_type {
                            TokenType::Plus =>
                                builder.build_int_add(lhs, rhs, "add"),

                            TokenType::Minus =>
                                builder.build_int_sub(lhs, rhs, "sub"),

                            TokenType::Asterisk =>
                                builder.build_int_mul(lhs, rhs, "mul"),

                            TokenType::Slash =>
                                builder.build_int_signed_div(lhs, rhs, "div"),

                            TokenType::Caret => {
                                return Ok(
                                    build_int_pow(lhs, rhs, context, builder)?
                                        .into()
                                );
                            }

                            _ => unreachable!(),
                        };

                        Ok(result.map_err(|e| e.to_string())?.into())
                    }
                    // Floating-point arithmetic
                    else if lhs.is_float_value() && rhs.is_float_value() {
                        let lhs = lhs.into_float_value();
                        let rhs = rhs.into_float_value();

                        let result = match &token.token_type {
                            TokenType::Plus =>
                                builder.build_float_add(lhs, rhs, "add"),

                            TokenType::Minus =>
                                builder.build_float_sub(lhs, rhs, "sub"),

                            TokenType::Asterisk =>
                                builder.build_float_mul(lhs, rhs, "mul"),

                            TokenType::Slash =>
                                builder.build_float_div(lhs, rhs, "div"),

                            TokenType::Caret => {
                                return Err(
                                    "Floating-point exponentiation needs libm pow"
                                        .into()
                                );
                            }

                            _ => unreachable!(),
                        };

                        Ok(result.map_err(|e| e.to_string())?.into())
                    } else {
                        Err("Cannot mix integer and float operands".into())
                    }
                }

                _ => Err(format!(
                    "Unsupported operation: {:?}",
                    token.token_type
                )),
            }
        }

        Expression::Illegal => {
            Err("Got Illegal expression while generating code".into())
        }
    }
}

fn build<'ctx>(
    expr: Expression,
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
) -> Result<bool, String> {
    // Basic types
    let i32_type = context.i32_type();
    let ptr_type = context.ptr_type(AddressSpace::default());
    let zero = i32_type.const_int(0, false);

    // printf(ptr, ...) -> i32
    let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
    let printf_func = module.add_function("printf", printf_type, None);

    // format_str = "%d\n\0"
    let format_str = context.const_string(b"%lld\n", true);
    let global_str = module.add_global(format_str.get_type(), None, "format_str");
    global_str.set_initializer(&format_str);
    global_str.set_linkage(inkwell::module::Linkage::Private);
    global_str.set_unnamed_addr(true);

    // main() -> i32
    let main_type = i32_type.fn_type(&[], false);
    let main_func = module.add_function("main", main_type, None);
    // main() -> Base block "entry"
    let entry_block = context.append_basic_block(main_func, "entry");
    builder.position_at_end(entry_block);
    // main() -> Body
    
    let res = build_expr(&expr, context, builder)?;

    let format_ptr = global_str.as_pointer_value();

    // printf("%d\n", res);
    builder
    .build_call(
        printf_func,
        &[format_ptr.into(), res.into()],
        "call",
    )
    .map_err(|e| e.to_string())?;

    // return 0;
    builder.build_return(Some(&zero))
        .expect("Codegen: Return failed");

    // Verify that object structure is valid and return it
    Ok(main_func.verify(true))
}

pub fn compile_expr(expr: Expression, output: OutputType) {
    // Create context & module
    let context = Context::create();
    let module = context.create_module("my_rust_llvm_program");
    
    // Create builder
    let builder = context.create_builder();
    let result = build(expr, &context, &module, &builder);
    match result {
        Ok(_b) => {
        match output {
            OutputType::Obj(path) => {
                println!("Generating native object file...");
            
                Target::initialize_native(&InitializationConfig::default())
                    .expect("Failed to initialize native target");

                let triple = TargetMachine::get_default_triple();
                let target = Target::from_triple(&triple)
                    .expect("Failed to get target from triple");
                
                let cpu = TargetMachine::get_host_cpu_name().to_string();
                let features = TargetMachine::get_host_cpu_features().to_string();

                let target_machine = target
                    .create_target_machine(
                        &triple,
                        &cpu,
                        &features,
                        inkwell::OptimizationLevel::Default,
                        RelocMode::PIC,
                        CodeModel::Default,
                    )
                    .expect("Failed to create target machine");

                target_machine
                    .write_to_file(&module, FileType::Object, &path)
                    .expect("Failed to write object file");
                
                println!("Object file written to: {}", path.display());
            },
            OutputType::IR(path) => {
                module.print_to_file(&path).unwrap();
                println!("IR file written to: {}", path.display());
            },
        }
    },
    Err(e) => println!("Error when creating LLVM graph: {}", e),
    }
}