use std::collections::HashMap;

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel,
    builder::Builder,
    context::Context,
    module::Module,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue},
};
use nova_compiler::ast::{
    BinaryOperator, Expression, FStringPart, Program, Statement, TypeExpr, UnaryOperator,
};

use crate::error::{CodegenError, CodegenResult};

/// One scope level — maps variable name → (alloca pointer, LLVM type)
type Scope<'ctx> = HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>;

/// The LLVM code generator for Nova programs.
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,

    /// Stack of variable scopes (innermost last)
    scopes: Vec<Scope<'ctx>>,

    /// Top-level functions registered so far
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        Self {
            context,
            module: context.create_module(module_name),
            builder: context.create_builder(),
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    // ── Scope helpers ────────────────────────────────────────────────────────

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ptr: PointerValue<'ctx>, ty: BasicTypeEnum<'ctx>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ptr, ty));
        }
    }

    fn lookup_var(&self, name: &str) -> Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(*v);
            }
        }
        None
    }

    // ── Type helpers ─────────────────────────────────────────────────────────

    fn llvm_type(&self, ty: &TypeExpr) -> CodegenResult<BasicTypeEnum<'ctx>> {
        match ty {
            TypeExpr::Named(n) => match n.as_str() {
                "int" | "i64" => Ok(self.context.i64_type().into()),
                "i32" => Ok(self.context.i32_type().into()),
                "bool" => Ok(self.context.bool_type().into()),
                "float" | "f64" => Ok(self.context.f64_type().into()),
                "f32" => Ok(self.context.f32_type().into()),
                // str / none represented as i8* (opaque pointer for now)
                "str" | "none" | "Self" | "any" => {
                    Ok(self.context.ptr_type(AddressSpace::default()).into())
                }
                other => Err(CodegenError::Unsupported(format!("type '{other}'"))),
            },
            TypeExpr::Generic(base, _) => {
                // Generic types: desugar to pointer for now
                match base.as_str() {
                    "list" | "dict" | "Option" | "Result" => {
                        Ok(self.context.ptr_type(AddressSpace::default()).into())
                    }
                    other => Err(CodegenError::Unsupported(format!("generic type '{other}'"))),
                }
            }
            TypeExpr::Tuple(_)
            | TypeExpr::Function(_, _)
            | TypeExpr::Optional(_)
            | TypeExpr::Result(_, _) => Ok(self.context.ptr_type(AddressSpace::default()).into()),
        }
    }

    #[allow(dead_code)]
    fn default_value(&self, ty: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        match ty {
            BasicTypeEnum::IntType(t) => t.const_zero().into(),
            BasicTypeEnum::FloatType(t) => t.const_zero().into(),
            BasicTypeEnum::PointerType(t) => t.const_null().into(),
            BasicTypeEnum::ArrayType(t) => t.const_zero().into(),
            BasicTypeEnum::StructType(t) => t.const_zero().into(),
            BasicTypeEnum::VectorType(t) => t.const_zero().into(),
            BasicTypeEnum::ScalableVectorType(t) => t.const_zero().into(),
        }
    }

    // ── Alloca in the entry block (standard pattern) ─────────────────────────

    fn alloca_entry(
        &self,
        func: FunctionValue<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        let entry = func.get_first_basic_block().unwrap();
        let saved = self.builder.get_insert_block();

        // Insert alloca before the first non-alloca instruction
        match entry.get_first_instruction() {
            Some(first) => self.builder.position_before(&first),
            None => self.builder.position_at_end(entry),
        }

        let ptr = self.builder.build_alloca(ty, name).unwrap();

        if let Some(block) = saved {
            self.builder.position_at_end(block);
        }
        ptr
    }

    // ── Program entry ────────────────────────────────────────────────────────

    /// Compile a full Nova program. Returns the LLVM IR as a string.
    pub fn compile_program(&mut self, program: &Program) -> CodegenResult<String> {
        // First pass: register all top-level function signatures
        for stmt in &program.statements {
            if let Statement::FunctionDef {
                name,
                params,
                return_type,
                ..
            } = stmt
            {
                self.register_function(name, params.as_slice(), return_type.as_ref())?;
            }
        }

        // Declare external `puts` / `printf` for print() built-in
        self.declare_builtins();

        // Second pass: compile function bodies + top-level code into main
        let i32_ty = self.context.i32_type();
        let main_ty = i32_ty.fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_ty, None);
        let entry_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry_block);
        self.functions.insert("main".into(), main_fn);

        for stmt in &program.statements {
            match stmt {
                Statement::FunctionDef { .. } => {
                    self.compile_function(stmt)?;
                    // Restore builder to main's entry block after each function
                    self.builder.position_at_end(entry_block);
                }
                other => self.compile_statement(other, main_fn)?,
            }
        }

        // Implicit `ret i32 0` if main doesn't already end with a terminator
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder
                .build_return(Some(&i32_ty.const_int(0, false)))
                .unwrap();
        }

        // Verify the module
        self.module.verify().map_err(|e| {
            CodegenError::Llvm(format!("module verification failed: {}", e.to_string()))
        })?;

        Ok(self.module.print_to_string().to_string())
    }

    // ── Built-in declarations ────────────────────────────────────────────────

    fn declare_builtins(&mut self) {
        let ptr_ty: BasicMetadataTypeEnum = self.context.ptr_type(AddressSpace::default()).into();

        // int puts(const char*)
        let puts_ty = self.context.i32_type().fn_type(&[ptr_ty], false);
        let puts = self.module.add_function("puts", puts_ty, None);
        self.functions.insert("__puts".into(), puts);

        // int printf(const char*, ...)
        let printf_ty = self.context.i32_type().fn_type(&[ptr_ty], true);
        let printf = self.module.add_function("printf", printf_ty, None);
        self.functions.insert("__printf".into(), printf);
    }

    // ── Function registration (first pass) ───────────────────────────────────

    fn register_function(
        &mut self,
        name: &str,
        params: &[nova_compiler::ast::Parameter],
        return_type: Option<&TypeExpr>,
    ) -> CodegenResult<()> {
        if self.module.get_function(name).is_some() {
            return Ok(());
        }

        let mut param_types: Vec<BasicMetadataTypeEnum> = Vec::new();
        for p in params {
            if p.name == "self" {
                param_types.push(self.context.ptr_type(AddressSpace::default()).into());
            } else {
                param_types.push(self.llvm_type(&p.type_annotation)?.into());
            }
        }

        let fn_val = match return_type {
            None => {
                // No return type annotation → default to void
                let ty = self.context.void_type().fn_type(&param_types, false);
                self.module.add_function(name, ty, None)
            }
            Some(TypeExpr::Named(n)) if matches!(n.as_str(), "none" | "void") => {
                let ty = self.context.void_type().fn_type(&param_types, false);
                self.module.add_function(name, ty, None)
            }
            Some(rt) => {
                let ret_ty = self.llvm_type(rt)?;
                let ty = ret_ty.fn_type(&param_types, false);
                self.module.add_function(name, ty, None)
            }
        };

        self.functions.insert(name.to_string(), fn_val);
        Ok(())
    }

    // ── Function compilation (second pass) ───────────────────────────────────

    fn compile_function(&mut self, stmt: &Statement) -> CodegenResult<()> {
        let Statement::FunctionDef {
            name,
            params,
            return_type,
            body,
            ..
        } = stmt
        else {
            return Ok(());
        };

        let fn_val = match self.module.get_function(name) {
            Some(f) => f,
            None => {
                self.register_function(name, params, return_type.as_ref())?;
                self.module.get_function(name).unwrap()
            }
        };

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        self.push_scope();

        // Bind parameters to alloca slots
        for (i, param) in params.iter().enumerate() {
            let llvm_param = fn_val.get_nth_param(i as u32).unwrap();
            let ty = llvm_param.get_type();
            let ptr = self.alloca_entry(fn_val, &param.name, ty);
            self.builder.build_store(ptr, llvm_param).unwrap();
            self.declare_var(&param.name, ptr, ty);
        }

        // Compile body statements
        let mut returned = false;
        for s in body {
            if returned {
                break;
            }
            if matches!(s, Statement::Return(_)) {
                returned = true;
            }
            self.compile_statement(s, fn_val)?;
        }

        // Implicit return for void functions
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            if fn_val.get_type().get_return_type().is_none() {
                self.builder.build_return(None).unwrap();
            } else {
                let zero = self.context.i64_type().const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            }
        }

        self.pop_scope();
        Ok(())
    }

    // ── Statement compilation ────────────────────────────────────────────────

    fn compile_statement(
        &mut self,
        stmt: &Statement,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        match stmt {
            Statement::LetBinding {
                name,
                type_annotation: _,
                value,
                ..
            } => {
                let val = self.compile_expr(value, func)?;
                let ty = val.get_type();
                let ptr = self.alloca_entry(func, name, ty);
                self.builder.build_store(ptr, val).unwrap();
                self.declare_var(name, ptr, ty);
            }

            Statement::Assignment { target, value } => {
                let val = self.compile_expr(value, func)?;
                if let Expression::Identifier(name) = target {
                    let (ptr, _) = self
                        .lookup_var(name)
                        .ok_or_else(|| CodegenError::UndefinedVariable(name.clone()))?;
                    self.builder.build_store(ptr, val).unwrap();
                }
            }

            Statement::Expression(expr) => {
                self.compile_expr(expr, func)?;
            }

            Statement::Return(expr) => match expr {
                Some(e) => {
                    let val = self.compile_expr(e, func)?;
                    self.builder.build_return(Some(&val)).unwrap();
                }
                None => {
                    self.builder.build_return(None).unwrap();
                }
            },

            Statement::If {
                condition,
                body,
                else_body,
                ..
            } => {
                self.compile_if(condition, body, else_body.as_deref(), func)?;
            }

            Statement::WhileLoop { condition, body } => {
                self.compile_while(condition, body, func)?;
            }

            Statement::ForLoop {
                variable,
                iterable,
                body,
            } => {
                self.compile_for(variable, iterable, body, func)?;
            }

            Statement::FunctionDef { .. } => {
                // Nested functions: compile as a top-level function
                self.compile_function(stmt)?;
            }

            // Require/ensure: runtime assertion (just evaluate condition for now)
            Statement::Require(expr) | Statement::Ensure(expr) => {
                self.compile_expr(expr, func)?;
            }

            // Declarations handled in first pass or not yet implemented
            Statement::StructDef { .. }
            | Statement::EnumDef { .. }
            | Statement::TraitDef { .. }
            | Statement::ImplBlock { .. }
            | Statement::Import { .. }
            | Statement::ForeignImport { .. }
            | Statement::ConstBinding { .. }
            | Statement::Break
            | Statement::Continue
            | Statement::Match { .. } => {
                // TODO in subsequent passes
            }
        }
        Ok(())
    }

    // ── If / while / for ─────────────────────────────────────────────────────

    fn compile_if(
        &mut self,
        condition: &Expression,
        then_block: &[Statement],
        else_block: Option<&[Statement]>,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        let cond_val = self.compile_expr(condition, func)?;
        let cond_bool = self.val_to_bool(cond_val)?;

        let then_bb = self.context.append_basic_block(func, "if.then");
        let else_bb = self.context.append_basic_block(func, "if.else");
        let merge_bb = self.context.append_basic_block(func, "if.merge");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .unwrap();

        // then
        self.builder.position_at_end(then_bb);
        self.push_scope();
        for s in then_block {
            self.compile_statement(s, func)?;
        }
        self.pop_scope();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        // else
        self.builder.position_at_end(else_bb);
        if let Some(else_stmts) = else_block {
            self.push_scope();
            for s in else_stmts {
                self.compile_statement(s, func)?;
            }
            self.pop_scope();
        }
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    fn compile_while(
        &mut self,
        condition: &Expression,
        body: &[Statement],
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let exit_bb = self.context.append_basic_block(func, "while.exit");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(condition, func)?;
        let cond_bool = self.val_to_bool(cond_val)?;
        self.builder
            .build_conditional_branch(cond_bool, body_bb, exit_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        for s in body {
            self.compile_statement(s, func)?;
        }
        self.pop_scope();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(exit_bb);
        Ok(())
    }

    fn compile_for(
        &mut self,
        variable: &str,
        iterable: &Expression,
        body: &[Statement],
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        // For now: support `for i in range(n)` — compile as an i64 counter loop.
        // Full iterator protocol comes later.
        let limit = match iterable {
            Expression::Call { function, args } => {
                if let Expression::Identifier(name) = function.as_ref() {
                    if name == "range" && args.len() == 1 {
                        self.compile_expr(&args[0], func)?
                    } else {
                        return Err(CodegenError::Unsupported(
                            "for loop over non-range iterables".into(),
                        ));
                    }
                } else {
                    return Err(CodegenError::Unsupported("for loop iterator".into()));
                }
            }
            other => self.compile_expr(other, func)?,
        };

        let i64_ty = self.context.i64_type();
        let counter_ptr = self.alloca_entry(func, variable, i64_ty.into());
        self.builder
            .build_store(counter_ptr, i64_ty.const_int(0, false))
            .unwrap();
        self.declare_var(variable, counter_ptr, i64_ty.into());

        let cond_bb = self.context.append_basic_block(func, "for.cond");
        let body_bb = self.context.append_basic_block(func, "for.body");
        let exit_bb = self.context.append_basic_block(func, "for.exit");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let cur = self
            .builder
            .build_load(i64_ty, counter_ptr, "i")
            .unwrap()
            .into_int_value();
        let limit_int = match limit {
            BasicValueEnum::IntValue(v) => v,
            _ => return Err(CodegenError::Unsupported("non-integer range limit".into())),
        };
        let cmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, limit_int, "for.cmp")
            .unwrap();
        self.builder
            .build_conditional_branch(cmp, body_bb, exit_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        for s in body {
            self.compile_statement(s, func)?;
        }
        self.pop_scope();
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            // Increment counter
            let cur2 = self
                .builder
                .build_load(i64_ty, counter_ptr, "i.next")
                .unwrap()
                .into_int_value();
            let next = self
                .builder
                .build_int_add(cur2, i64_ty.const_int(1, false), "i.inc")
                .unwrap();
            self.builder.build_store(counter_ptr, next).unwrap();
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(exit_bb);
        Ok(())
    }

    // ── Expression compilation ───────────────────────────────────────────────

    fn compile_expr(
        &mut self,
        expr: &Expression,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match expr {
            Expression::IntLiteral(n) => {
                Ok(self.context.i64_type().const_int(*n as u64, *n < 0).into())
            }

            Expression::FloatLiteral(f) => Ok(self.context.f64_type().const_float(*f).into()),

            Expression::BoolLiteral(b) => {
                Ok(self.context.bool_type().const_int(*b as u64, false).into())
            }

            Expression::NoneLiteral => Ok(self
                .context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into()),

            Expression::StringLiteral(s) => {
                let global = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(global.as_pointer_value().into())
            }

            Expression::FString(parts) => self.compile_fstring(parts, func),

            Expression::Identifier(name) => {
                let (ptr, ty) = self
                    .lookup_var(name)
                    .ok_or_else(|| CodegenError::UndefinedVariable(name.clone()))?;
                Ok(self.builder.build_load(ty, ptr, name).unwrap())
            }

            Expression::BinaryOp { op, left, right } => self.compile_binop(op, left, right, func),

            Expression::UnaryOp { op, operand } => self.compile_unaryop(op, operand, func),

            Expression::Call { function, args } => self.compile_call(function, args, func),

            Expression::FieldAccess { object, field } => {
                // For now: evaluate object (side effects), return 0
                self.compile_expr(object, func)?;
                Err(CodegenError::Unsupported(format!(
                    "field access '.{field}' (structs not yet in codegen)"
                )))
            }

            Expression::Lambda { params: _, body: _ } => {
                Err(CodegenError::Unsupported("lambda expressions".into()))
            }

            Expression::Pipe { left, right } => {
                // `left |> right` = `right(left)`
                let lval = self.compile_expr(left, func)?;
                if let Expression::Identifier(fn_name) = right.as_ref()
                    && let Some(fn_val) = self.functions.get(fn_name).copied()
                {
                    let result = self
                        .builder
                        .build_call(fn_val, &[lval.into()], "pipe")
                        .unwrap();
                    return Ok(result.try_as_basic_value().basic().unwrap_or_else(|| {
                        self.context
                            .ptr_type(AddressSpace::default())
                            .const_null()
                            .into()
                    }));
                }
                Err(CodegenError::Unsupported("pipe to non-identifier".into()))
            }

            Expression::Index {
                object: _,
                index: _,
            } => Err(CodegenError::Unsupported("index expressions".into())),

            Expression::List(_) | Expression::Dict(_) => {
                Err(CodegenError::Unsupported("collection literals".into()))
            }

            Expression::StructInit { .. } => {
                Err(CodegenError::Unsupported("struct init expressions".into()))
            }

            Expression::ResultExpr { .. } | Expression::Await(_) => {
                Err(CodegenError::Unsupported("result/await expressions".into()))
            }

            Expression::MethodCall { method, .. } => Err(CodegenError::Unsupported(format!(
                "method calls (.{method}())"
            ))),
        }
    }

    fn compile_fstring(
        &mut self,
        parts: &[FStringPart],
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        // Simplest correct implementation: print each part via printf,
        // return a null pointer as the "string value" (full string concat comes in v0.2.0 stdlib).
        let fmt_int = self
            .builder
            .build_global_string_ptr("%lld", "fmt_int")
            .unwrap()
            .as_pointer_value();
        let fmt_flt = self
            .builder
            .build_global_string_ptr("%g", "fmt_flt")
            .unwrap()
            .as_pointer_value();
        let fmt_str = self
            .builder
            .build_global_string_ptr("%s", "fmt_str")
            .unwrap()
            .as_pointer_value();

        let printf = self.functions["__printf"];

        for part in parts {
            match part {
                FStringPart::Literal(s) => {
                    let ptr = self
                        .builder
                        .build_global_string_ptr(s, "fstr_lit")
                        .unwrap()
                        .as_pointer_value();
                    self.builder
                        .build_call(printf, &[fmt_str.into(), ptr.into()], "")
                        .unwrap();
                }
                FStringPart::Expression(e) => {
                    let val = self.compile_expr(e, func)?;
                    let fmt = match val {
                        BasicValueEnum::IntValue(_) => fmt_int,
                        BasicValueEnum::FloatValue(_) => fmt_flt,
                        _ => fmt_str,
                    };
                    self.builder
                        .build_call(printf, &[fmt.into(), val.into()], "")
                        .unwrap();
                }
            }
        }

        Ok(self
            .context
            .ptr_type(AddressSpace::default())
            .const_null()
            .into())
    }

    #[allow(dead_code)]
    fn compile_if_expr(
        &mut self,
        condition: &Expression,
        then_expr: &Expression,
        else_expr: &Expression,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let cond_val = self.compile_expr(condition, func)?;
        let cond_bool = self.val_to_bool(cond_val)?;

        let then_bb = self.context.append_basic_block(func, "ternary.then");
        let else_bb = self.context.append_basic_block(func, "ternary.else");
        let merge_bb = self.context.append_basic_block(func, "ternary.merge");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .unwrap();

        self.builder.position_at_end(then_bb);
        let then_val = self.compile_expr(then_expr, func)?;
        let then_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(else_bb);
        let else_val = self.compile_expr(else_expr, func)?;
        let else_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(then_val.get_type(), "ternary.result")
            .unwrap();
        phi.add_incoming(&[(&then_val, then_end), (&else_val, else_end)]);
        Ok(phi.as_basic_value())
    }

    // ── Binary operations ────────────────────────────────────────────────────

    fn compile_binop(
        &mut self,
        op: &BinaryOperator,
        left: &Expression,
        right: &Expression,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        // Short-circuit for logical operators
        if matches!(op, BinaryOperator::And | BinaryOperator::Or) {
            return self.compile_logical(op, left, right, func);
        }

        let lval = self.compile_expr(left, func)?;
        let rval = self.compile_expr(right, func)?;

        match (lval, rval) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => Ok(match op {
                BinaryOperator::Add => self.builder.build_int_add(l, r, "add").unwrap().into(),
                BinaryOperator::Sub => self.builder.build_int_sub(l, r, "sub").unwrap().into(),
                BinaryOperator::Mul => self.builder.build_int_mul(l, r, "mul").unwrap().into(),
                BinaryOperator::Div | BinaryOperator::IntDiv => self
                    .builder
                    .build_int_signed_div(l, r, "div")
                    .unwrap()
                    .into(),
                BinaryOperator::Mod => self
                    .builder
                    .build_int_signed_rem(l, r, "rem")
                    .unwrap()
                    .into(),
                BinaryOperator::Power => {
                    return Err(CodegenError::Unsupported("** for ints".into()));
                }
                BinaryOperator::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .unwrap()
                    .into(),
                BinaryOperator::NotEq => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .unwrap()
                    .into(),
                BinaryOperator::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .unwrap()
                    .into(),
                BinaryOperator::LtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .unwrap()
                    .into(),
                BinaryOperator::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .unwrap()
                    .into(),
                BinaryOperator::GtEq => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .unwrap()
                    .into(),
                BinaryOperator::In
                | BinaryOperator::Is
                | BinaryOperator::And
                | BinaryOperator::Or => {
                    return Err(CodegenError::Unsupported(format!("op {op:?} on ints")));
                }
            }),
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => Ok(match op {
                BinaryOperator::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinaryOperator::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinaryOperator::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinaryOperator::Div | BinaryOperator::IntDiv => {
                    self.builder.build_float_div(l, r, "fdiv").unwrap().into()
                }
                BinaryOperator::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinaryOperator::NotEq => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinaryOperator::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinaryOperator::LtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinaryOperator::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinaryOperator::GtEq => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
                    .into(),
                _ => return Err(CodegenError::Unsupported(format!("float op {op:?}"))),
            }),
            _ => Err(CodegenError::Unsupported(format!(
                "binary op {op:?} for these types"
            ))),
        }
    }

    fn compile_logical(
        &mut self,
        op: &BinaryOperator,
        left: &Expression,
        right: &Expression,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let bool_ty = self.context.bool_type();
        let lval = self.compile_expr(left, func)?;
        let lcond = self.val_to_bool(lval)?;

        let right_bb = self.context.append_basic_block(func, "logic.right");
        let merge_bb = self.context.append_basic_block(func, "logic.merge");
        let short_bb = self.context.append_basic_block(func, "logic.short");

        // For `and`: short-circuit on false. For `or`: short-circuit on true.
        if matches!(op, BinaryOperator::And) {
            self.builder
                .build_conditional_branch(lcond, right_bb, short_bb)
                .unwrap();
        } else {
            self.builder
                .build_conditional_branch(lcond, short_bb, right_bb)
                .unwrap();
        }

        self.builder.position_at_end(short_bb);
        let short_val = if matches!(op, BinaryOperator::And) {
            bool_ty.const_int(0, false)
        } else {
            bool_ty.const_int(1, false)
        };
        let short_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(right_bb);
        let rval = self.compile_expr(right, func)?;
        let rcond = self.val_to_bool(rval)?;
        let right_end = self.builder.get_insert_block().unwrap();
        self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self.builder.build_phi(bool_ty, "logic.result").unwrap();
        phi.add_incoming(&[
            (&short_val as &dyn inkwell::values::BasicValue, short_end),
            (&rcond as &dyn inkwell::values::BasicValue, right_end),
        ]);
        Ok(phi.as_basic_value())
    }

    fn compile_unaryop(
        &mut self,
        op: &UnaryOperator,
        operand: &Expression,
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let val = self.compile_expr(operand, func)?;
        match (op, val) {
            (UnaryOperator::Neg, BasicValueEnum::IntValue(v)) => {
                Ok(self.builder.build_int_neg(v, "neg").unwrap().into())
            }
            (UnaryOperator::Neg, BasicValueEnum::FloatValue(v)) => {
                Ok(self.builder.build_float_neg(v, "fneg").unwrap().into())
            }
            (UnaryOperator::Not, v) => {
                let b = self.val_to_bool(v)?;
                Ok(self.builder.build_not(b, "not").unwrap().into())
            }
            _ => Err(CodegenError::Unsupported(format!("unary op {op:?}"))),
        }
    }

    // ── Function calls ───────────────────────────────────────────────────────

    fn compile_call(
        &mut self,
        function: &Expression,
        args: &[Expression],
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        // Compile arguments first
        let mut compiled_args: Vec<BasicMetadataValueEnum> = Vec::new();
        for a in args {
            compiled_args.push(self.compile_expr(a, func)?.into());
        }

        match function {
            Expression::Identifier(name) => {
                // Built-in: print(value)
                if name == "print" || name == "println" {
                    return self.compile_print(args, func);
                }

                let fn_val = self
                    .functions
                    .get(name)
                    .copied()
                    .ok_or_else(|| CodegenError::UndefinedFunction(name.clone()))?;

                let call = self
                    .builder
                    .build_call(fn_val, &compiled_args, "call")
                    .unwrap();
                Ok(call.try_as_basic_value().basic().unwrap_or_else(|| {
                    self.context
                        .ptr_type(AddressSpace::default())
                        .const_null()
                        .into()
                }))
            }
            other => Err(CodegenError::Unsupported(format!(
                "call to complex expression: {other:?}"
            ))),
        }
    }

    fn compile_print(
        &mut self,
        args: &[Expression],
        func: FunctionValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let printf = self.functions["__printf"];
        let puts = self.functions["__puts"];

        for arg in args {
            let val = self.compile_expr(arg, func)?;
            match val {
                BasicValueEnum::IntValue(v) => {
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%lld\n", "fmt_int_nl")
                        .unwrap()
                        .as_pointer_value();
                    self.builder
                        .build_call(printf, &[fmt.into(), v.into()], "")
                        .unwrap();
                }
                BasicValueEnum::FloatValue(v) => {
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%g\n", "fmt_flt_nl")
                        .unwrap()
                        .as_pointer_value();
                    self.builder
                        .build_call(printf, &[fmt.into(), v.into()], "")
                        .unwrap();
                }
                BasicValueEnum::PointerValue(v) => {
                    // Assume it's a char* (string)
                    self.builder.build_call(puts, &[v.into()], "").unwrap();
                }
                _ => {
                    // bool, etc: print as int
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d\n", "fmt_bool_nl")
                        .unwrap()
                        .as_pointer_value();
                    self.builder
                        .build_call(printf, &[fmt.into(), val.into()], "")
                        .unwrap();
                }
            }
        }

        Ok(self
            .context
            .ptr_type(AddressSpace::default())
            .const_null()
            .into())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn val_to_bool(
        &self,
        val: BasicValueEnum<'ctx>,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        match val {
            BasicValueEnum::IntValue(v) => {
                if v.get_type().get_bit_width() == 1 {
                    Ok(v)
                } else {
                    // Non-zero is truthy
                    Ok(self
                        .builder
                        .build_int_compare(IntPredicate::NE, v, v.get_type().const_zero(), "tobool")
                        .unwrap())
                }
            }
            BasicValueEnum::FloatValue(v) => Ok(self
                .builder
                .build_float_compare(FloatPredicate::ONE, v, v.get_type().const_zero(), "ftobool")
                .unwrap()),
            BasicValueEnum::PointerValue(v) => {
                Ok(self.builder.build_is_not_null(v, "ptobool").unwrap())
            }
            _ => Err(CodegenError::Unsupported(
                "bool conversion for this type".into(),
            )),
        }
    }

    // ── Binary output ────────────────────────────────────────────────────────

    /// Emit a native object file for the current module.
    pub fn emit_object_file(&self, path: &str) -> CodegenResult<()> {
        Target::initialize_all(&InitializationConfig::default());

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple).map_err(|e| CodegenError::Llvm(e.to_string()))?;

        let cpu = TargetMachine::get_host_cpu_name();
        let features = TargetMachine::get_host_cpu_features();

        let machine = target
            .create_target_machine(
                &triple,
                cpu.to_str().unwrap(),
                features.to_str().unwrap(),
                OptimizationLevel::Default,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| CodegenError::Llvm("failed to create target machine".into()))?;

        machine
            .write_to_file(&self.module, FileType::Object, std::path::Path::new(path))
            .map_err(|e| CodegenError::Llvm(e.to_string()))?;

        Ok(())
    }
}
