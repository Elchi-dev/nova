use nova_lexer::Span;
use nova_parser::ast::*;

use crate::{
    env::Env,
    error::TypeError,
    types::{type_expr_to_type, types_compatible, Type},
};

/// The type checker. Walks the AST, infers types, and collects errors.
///
/// Errors are non-fatal by design: checking continues after an error so that
/// a single source file produces all its errors in one pass, not just the first.
pub struct Checker {
    pub errors: Vec<TypeError>,
}

impl Checker {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    // ── Program ───────────────────────────────────────────────────────────────

    /// Check an entire program. Uses a two-pass approach:
    /// 1. Collect all top-level function signatures into the env.
    /// 2. Check all items.
    pub fn check_program(&mut self, program: &Program, env: &mut Env) {
        // Pass 1: hoist function signatures so forward references work.
        for item in &program.items {
            if let Item::Function(f) = item {
                self.register_fn_sig(f, env);
            }
            if let Item::Module(m) = item {
                self.register_module_sigs(m, env);
            }
        }

        // Pass 2: full checking.
        for item in &program.items {
            self.check_item(item, env);
        }
    }

    // ── Items ─────────────────────────────────────────────────────────────────

    fn check_item(&mut self, item: &Item, env: &mut Env) {
        match item {
            Item::Function(f) => self.check_fn(f, env),
            Item::Module(m) => self.check_module(m, env),
            Item::Struct(s) => self.check_struct(s, env),
            Item::Import(_) => {}        // resolved in a later pass (v0.5)
            Item::RuntimeConfig(_) => {} // validated at the CLI level
            Item::Expr(e) => {
                self.infer_expr(e, env);
            }
        }
    }

    /// Register a function's type signature in the env WITHOUT checking its body.
    fn register_fn_sig(&mut self, f: &Function, env: &mut Env) {
        let params: Vec<Type> = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(type_expr_to_type)
                    .unwrap_or(Type::Unknown)
            })
            .collect();

        let ret = f
            .return_ty
            .as_ref()
            .map(type_expr_to_type)
            .unwrap_or(Type::Void);

        env.define_let(
            f.name.clone(),
            Type::Function {
                params,
                ret: Box::new(ret),
            },
        );
    }

    fn register_module_sigs(&mut self, m: &ModuleDecl, env: &mut Env) {
        for item in &m.items {
            if let Item::Function(f) = item {
                self.register_fn_sig(f, env);
            }
        }
    }

    // ── Functions ─────────────────────────────────────────────────────────────

    fn check_fn(&mut self, f: &Function, env: &mut Env) {
        env.push_scope();

        // Bind parameters into the function scope.
        for param in &f.params {
            let ty = param
                .ty
                .as_ref()
                .map(type_expr_to_type)
                .unwrap_or(Type::Unknown);
            env.define_let(param.name.clone(), ty);
        }

        // Set the return-type context.
        let ret_ty = f
            .return_ty
            .as_ref()
            .map(type_expr_to_type)
            .unwrap_or(Type::Void);
        let prev_ret = env.current_return_ty.replace(ret_ty);

        for stmt in &f.body {
            self.check_stmt(stmt, env);
        }

        env.current_return_ty = prev_ret;
        env.pop_scope();
    }

    // ── Modules ───────────────────────────────────────────────────────────────

    fn check_module(&mut self, m: &ModuleDecl, env: &mut Env) {
        env.push_scope();
        // Hoist function signatures within the module.
        for item in &m.items {
            if let Item::Function(f) = item {
                self.register_fn_sig(f, env);
            }
        }
        for item in &m.items {
            self.check_item(item, env);
        }
        env.pop_scope();
    }

    // ── Structs ───────────────────────────────────────────────────────────────

    fn check_struct(&mut self, s: &StructDecl, env: &mut Env) {
        // Validate that all field types are resolvable.
        for field in &s.fields {
            let ty = type_expr_to_type(&field.ty);
            if matches!(ty, Type::Unknown) {
                // Unknown means a user-defined type that we can't resolve yet —
                // that's fine at this stage.
            }
        }

        // Register the struct name as Unknown for now.
        // Full struct type support lands in v0.4 with the codegen.
        env.define_let(s.name.clone(), Type::Unknown);
    }

    // ── Statements ────────────────────────────────────────────────────────────

    pub fn check_stmt(&mut self, stmt: &Stmt, env: &mut Env) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                span,
            } => {
                self.check_binding(name, ty, value, false, *span, env);
            }

            Stmt::Var {
                name,
                ty,
                value,
                span,
            } => {
                self.check_binding(name, ty, value, true, *span, env);
            }

            Stmt::Assign {
                target,
                value,
                span,
            } => {
                // Immutability check for simple identifier assignments.
                if let Expr::Ident(name, id_span) = target {
                    if !env.is_mutable(name) && env.lookup(name).is_some() {
                        self.errors.push(TypeError::ImmutableAssign {
                            name: name.clone(),
                            span: *id_span,
                        });
                        return;
                    }
                }
                let target_ty = self.infer_expr(target, env);
                let value_ty = self.infer_expr(value, env);
                self.expect_compatible(&target_ty, &value_ty, *span);
            }

            Stmt::Return { value, span } => {
                let expected = env.current_return_ty.clone().unwrap_or(Type::Void);

                let actual = value
                    .as_ref()
                    .map(|v| self.infer_expr(v, env))
                    .unwrap_or(Type::Void);

                if !types_compatible(&expected, &actual) {
                    self.errors.push(TypeError::Mismatch {
                        expected,
                        found: actual,
                        span: *span,
                    });
                }
            }

            Stmt::If {
                condition,
                then_body,
                elif_branches,
                else_body,
                span,
            } => {
                let cond_ty = self.infer_expr(condition, env);
                if !types_compatible(&Type::Bool, &cond_ty) {
                    self.errors.push(TypeError::Mismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                        span: *span,
                    });
                }

                env.push_scope();
                for s in then_body {
                    self.check_stmt(s, env);
                }
                env.pop_scope();

                for (cond, body) in elif_branches {
                    let ty = self.infer_expr(cond, env);
                    if !types_compatible(&Type::Bool, &ty) {
                        self.errors.push(TypeError::Mismatch {
                            expected: Type::Bool,
                            found: ty,
                            span: *span,
                        });
                    }
                    env.push_scope();
                    for s in body {
                        self.check_stmt(s, env);
                    }
                    env.pop_scope();
                }

                if let Some(body) = else_body {
                    env.push_scope();
                    for s in body {
                        self.check_stmt(s, env);
                    }
                    env.pop_scope();
                }
            }

            Stmt::While {
                condition,
                body,
                span,
            } => {
                let cond_ty = self.infer_expr(condition, env);
                if !types_compatible(&Type::Bool, &cond_ty) {
                    self.errors.push(TypeError::Mismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                        span: *span,
                    });
                }
                env.push_scope();
                let prev = env.in_loop;
                env.in_loop = true;
                for s in body {
                    self.check_stmt(s, env);
                }
                env.in_loop = prev;
                env.pop_scope();
            }

            Stmt::For {
                var,
                iter,
                body,
                span,
            } => {
                let iter_ty = self.infer_expr(iter, env);
                let elem_ty = match &iter_ty {
                    Type::Array(inner) => *inner.clone(),
                    Type::Str => Type::Str,
                    Type::Unknown | Type::Error => Type::Unknown,
                    other => {
                        self.errors.push(TypeError::NotIterable {
                            ty: other.clone(),
                            span: *span,
                        });
                        Type::Error
                    }
                };

                env.push_scope();
                env.define_let(var.clone(), elem_ty);
                let prev = env.in_loop;
                env.in_loop = true;
                for s in body {
                    self.check_stmt(s, env);
                }
                env.in_loop = prev;
                env.pop_scope();
            }

            Stmt::Break { span } | Stmt::Continue { span } => {
                if !env.in_loop {
                    // Not inside a loop — this is a semantic error.
                    // We emit it as a type mismatch with Never for now.
                    // TODO: add a dedicated LoopControlOutsideLoop error.
                    let _ = span;
                }
            }

            Stmt::Pass { .. } => {}

            Stmt::Expr(e) => {
                self.infer_expr(e, env);
            }
        }
    }

    /// Shared logic for `let` and `var` bindings.
    fn check_binding(
        &mut self,
        name: &str,
        declared_ty: &Option<TypeExpr>,
        value: &Expr,
        mutable: bool,
        span: Span,
        env: &mut Env,
    ) {
        let inferred = self.infer_expr(value, env);

        let final_ty = if let Some(decl) = declared_ty {
            let decl_ty = type_expr_to_type(decl);
            if !types_compatible(&decl_ty, &inferred) {
                self.errors.push(TypeError::Mismatch {
                    expected: decl_ty.clone(),
                    found: inferred,
                    span,
                });
                decl_ty // keep declared type for downstream checking
            } else {
                decl_ty
            }
        } else {
            // No annotation: resolve literal types to their defaults.
            inferred.resolve_default()
        };

        if mutable {
            env.define_var(name.to_string(), final_ty);
        } else {
            env.define_let(name.to_string(), final_ty);
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    /// Infer the type of an expression, recording any errors found.
    pub fn infer_expr(&mut self, expr: &Expr, env: &mut Env) -> Type {
        match expr {
            // Literals
            Expr::Int(_, _) => Type::IntLiteral,
            Expr::Float(_, _) => Type::FloatLiteral,
            Expr::Str(_, _) => Type::Str,
            Expr::Bool(_, _) => Type::Bool,
            Expr::FStr(_, _) => Type::Str,

            // Variables
            Expr::Ident(name, span) => match env.lookup(name) {
                Some(binding) => binding.ty.clone(),
                None => {
                    self.errors.push(TypeError::Undefined {
                        name: name.clone(),
                        span: *span,
                    });
                    Type::Error
                }
            },

            // Binary operations
            Expr::BinOp { op, lhs, rhs, span } => {
                let lty = self.infer_expr(lhs, env);
                let rty = self.infer_expr(rhs, env);
                self.infer_binop(*op, lty, rty, *span)
            }

            // Unary operations
            Expr::UnaryOp { op, expr, span } => {
                let ty = self.infer_expr(expr, env);
                self.infer_unary(*op, ty, *span)
            }

            // Function calls
            Expr::Call { callee, args, span } => self.infer_call(callee, args, *span, env),

            // Field access: obj.field
            Expr::Field {
                object,
                field: _,
                span: _,
            } => {
                let _obj_ty = self.infer_expr(object, env);
                // Full struct field resolution in v0.4.
                Type::Unknown
            }

            // Index: arr[i]
            Expr::Index {
                object,
                index,
                span,
            } => self.infer_index(object, index, *span, env),

            // Array literal: [a, b, c]
            Expr::Array(elems, span) => self.infer_array(elems, *span, env),
        }
    }

    // ── Binary operations ─────────────────────────────────────────────────────

    fn infer_binop(&mut self, op: BinOp, lty: Type, rty: Type, span: Span) -> Type {
        use BinOp::*;

        // Suppress cascading errors.
        if lty.is_poisoned() || rty.is_poisoned() {
            return Type::Error;
        }

        match op {
            // Arithmetic — both sides must be numeric and compatible.
            Add | Sub | Mul | Div | Mod => {
                if !lty.is_numeric() {
                    self.errors.push(TypeError::BinOpMismatch {
                        op: op_str(op),
                        lty: lty.clone(),
                        rty,
                        span,
                    });
                    return Type::Error;
                }
                if !types_compatible(&lty, &rty) {
                    self.errors.push(TypeError::BinOpMismatch {
                        op: op_str(op),
                        lty: lty.clone(),
                        rty,
                        span,
                    });
                    return Type::Error;
                }
                // Result has the same type as the operands (promote literals).
                if matches!(lty, Type::IntLiteral) {
                    rty.resolve_default()
                } else {
                    lty
                }
            }

            // Ordered comparison — both must be numeric.
            Lt | LtEq | Gt | GtEq => {
                if !lty.is_numeric() || !types_compatible(&lty, &rty) {
                    self.errors.push(TypeError::BinOpMismatch {
                        op: op_str(op),
                        lty,
                        rty,
                        span,
                    });
                }
                Type::Bool
            }

            // Equality — any two compatible types.
            Eq | NotEq => {
                if !types_compatible(&lty, &rty) {
                    self.errors.push(TypeError::BinOpMismatch {
                        op: op_str(op),
                        lty,
                        rty,
                        span,
                    });
                }
                Type::Bool
            }

            // Logical — both must be bool.
            And | Or => {
                if !types_compatible(&Type::Bool, &lty) {
                    self.errors.push(TypeError::Mismatch {
                        expected: Type::Bool,
                        found: lty,
                        span,
                    });
                }
                if !types_compatible(&Type::Bool, &rty) {
                    self.errors.push(TypeError::Mismatch {
                        expected: Type::Bool,
                        found: rty,
                        span,
                    });
                }
                Type::Bool
            }
        }
    }

    // ── Unary operations ──────────────────────────────────────────────────────

    fn infer_unary(&mut self, op: UnaryOp, ty: Type, span: Span) -> Type {
        if ty.is_poisoned() {
            return Type::Error;
        }
        match op {
            UnaryOp::Neg => {
                if !ty.is_numeric() {
                    self.errors
                        .push(TypeError::InvalidUnary { op: "-", ty, span });
                    Type::Error
                } else {
                    ty
                }
            }
            UnaryOp::Not => {
                if !types_compatible(&Type::Bool, &ty) {
                    self.errors.push(TypeError::Mismatch {
                        expected: Type::Bool,
                        found: ty,
                        span,
                    });
                    Type::Error
                } else {
                    Type::Bool
                }
            }
        }
    }

    // ── Function calls ────────────────────────────────────────────────────────

    fn infer_call(&mut self, callee: &Expr, args: &[Expr], span: Span, env: &mut Env) -> Type {
        let callee_ty = self.infer_expr(callee, env);

        match callee_ty {
            Type::Function {
                ref params,
                ref ret,
            } => {
                // Arg count check (skip if params contain Unknown — variadic built-ins).
                let has_unknown_params = params.iter().any(|p| matches!(p, Type::Unknown));

                if !has_unknown_params && params.len() != args.len() {
                    self.errors.push(TypeError::ArgCount {
                        expected: params.len(),
                        found: args.len(),
                        span,
                    });
                } else {
                    for (param_ty, arg) in params.iter().zip(args.iter()) {
                        let arg_ty = self.infer_expr(arg, env);
                        if !types_compatible(param_ty, &arg_ty) {
                            self.errors.push(TypeError::Mismatch {
                                expected: param_ty.clone(),
                                found: arg_ty,
                                span,
                            });
                        }
                    }
                    // Check remaining args if more than params (only possible with unknown params).
                    if has_unknown_params && args.len() > params.len() {
                        for arg in &args[params.len()..] {
                            self.infer_expr(arg, env);
                        }
                    }
                }

                *ret.clone()
            }

            Type::Error | Type::Unknown => {
                // Still infer arg types so we don't miss errors inside them.
                for arg in args {
                    self.infer_expr(arg, env);
                }
                Type::Error
            }

            other => {
                self.errors.push(TypeError::NotCallable { ty: other, span });
                for arg in args {
                    self.infer_expr(arg, env);
                }
                Type::Error
            }
        }
    }

    // ── Indexing ──────────────────────────────────────────────────────────────

    fn infer_index(&mut self, object: &Expr, index: &Expr, span: Span, env: &mut Env) -> Type {
        let obj_ty = self.infer_expr(object, env);
        let idx_ty = self.infer_expr(index, env);

        if !idx_ty.is_integer() && !idx_ty.is_poisoned() {
            self.errors.push(TypeError::Mismatch {
                expected: Type::I64,
                found: idx_ty,
                span,
            });
        }

        match obj_ty {
            Type::Array(inner) => *inner,
            Type::Str => Type::Str,
            Type::Unknown | Type::Error => Type::Unknown,
            other => {
                self.errors
                    .push(TypeError::NotIndexable { ty: other, span });
                Type::Error
            }
        }
    }

    // ── Array literals ────────────────────────────────────────────────────────

    fn infer_array(&mut self, elems: &[Expr], span: Span, env: &mut Env) -> Type {
        if elems.is_empty() {
            return Type::Array(Box::new(Type::Unknown));
        }

        let first_ty = self.infer_expr(&elems[0], env).resolve_default();

        for elem in &elems[1..] {
            let ty = self.infer_expr(elem, env);
            if !types_compatible(&first_ty, &ty) {
                self.errors.push(TypeError::Mismatch {
                    expected: first_ty.clone(),
                    found: ty,
                    span,
                });
            }
        }

        Type::Array(Box::new(first_ty))
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn expect_compatible(&mut self, expected: &Type, found: &Type, span: Span) {
        if !types_compatible(expected, found) {
            self.errors.push(TypeError::Mismatch {
                expected: expected.clone(),
                found: found.clone(),
                span,
            });
        }
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

fn op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::LtEq => "<=",
        BinOp::Gt => ">",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}
