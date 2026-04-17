use crate::ast::*;

use super::env::TypeEnv;
use super::error::TypeError;
use super::types::{Effect, FunctionSig, StructType, Type};
use super::unify::{self, Substitution};

/// Result of type checking a program
pub struct CheckResult {
    pub errors: Vec<TypeError>,
    pub warnings: Vec<String>,
}

/// The Nova type checker
pub struct Checker {
    env: TypeEnv,
    subst: Substitution,
    errors: Vec<TypeError>,
    warnings: Vec<String>,
    in_loop: bool,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            subst: Substitution::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            in_loop: false,
        }
    }

    /// Type-check an entire program
    pub fn check_program(&mut self, program: &Program) -> CheckResult {
        // First pass: register all top-level type and function definitions
        for stmt in &program.statements {
            self.register_declaration(stmt);
        }

        // Second pass: type-check all statements
        for stmt in &program.statements {
            self.check_statement(stmt);
        }

        CheckResult {
            errors: std::mem::take(&mut self.errors),
            warnings: std::mem::take(&mut self.warnings),
        }
    }

    /// Record an error without stopping
    fn error(&mut self, err: TypeError) {
        self.errors.push(err);
    }

    #[allow(dead_code)]
    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    // ── Declaration registration (first pass) ────────────────

    fn register_declaration(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDef {
                name,
                params,
                return_type,
                effects,
                is_pub,
                decorators,
                ..
            } => {
                let param_types: Vec<(String, Type)> = params
                    .iter()
                    .map(|p| {
                        let ty = self.env.resolve_type_expr(&p.type_annotation);
                        (p.name.clone(), ty)
                    })
                    .collect();

                let ret_ty = return_type
                    .as_ref()
                    .map(|t| self.env.resolve_type_expr(t))
                    .unwrap_or(Type::None);

                let effect_list: Vec<Effect> =
                    effects.iter().map(|e| Effect::from_str(e)).collect();

                let is_pure = decorators.iter().any(|d| d.name == "pure");

                self.env.define_function(FunctionSig {
                    name: name.clone(),
                    params: param_types,
                    return_type: ret_ty,
                    effects: effect_list,
                    is_pure,
                    is_pub: *is_pub,
                });
            }
            Statement::StructDef {
                name,
                fields,
                is_pub,
            } => {
                let field_types: Vec<(String, Type)> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.env.resolve_type_expr(&f.type_annotation);
                        (f.name.clone(), ty)
                    })
                    .collect();

                let struct_type = Type::Struct(StructType {
                    name: name.clone(),
                    fields: field_types,
                    is_pub: *is_pub,
                });

                self.env.define_type(name, struct_type);
            }
            _ => {}
        }
    }

    // ── Statement checking (second pass) ─────────────────────

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDef {
                name: _,
                params,
                return_type,
                effects,
                body,
                decorators,
                ..
            } => {
                let ret_ty = return_type
                    .as_ref()
                    .map(|t| self.env.resolve_type_expr(t))
                    .unwrap_or(Type::None);

                let effect_list: Vec<Effect> =
                    effects.iter().map(|e| Effect::from_str(e)).collect();

                let is_pure = decorators.iter().any(|d| d.name == "pure");

                self.env
                    .push_function_scope(ret_ty.clone(), effect_list, is_pure);

                // Bind parameters
                for param in params {
                    let ty = self.env.resolve_type_expr(&param.type_annotation);
                    self.env.define_variable(&param.name, ty, false);
                }

                // Check body
                for body_stmt in body {
                    self.check_statement(body_stmt);
                }

                self.env.pop_scope();
            }

            Statement::LetBinding {
                name,
                type_annotation,
                value,
                mutable,
            } => {
                let inferred = self.infer_expression(value);

                let expected = type_annotation
                    .as_ref()
                    .map(|t| self.env.resolve_type_expr(t));

                let final_type = if let Some(ref expected_ty) = expected {
                    if let Err(e) = unify::unify(expected_ty, &inferred, &mut self.subst) {
                        self.error(e);
                    }
                    expected_ty.clone()
                } else {
                    inferred
                };

                self.env.define_variable(name, final_type, *mutable);
            }

            Statement::ConstBinding {
                name,
                type_annotation,
                value,
            } => {
                let inferred = self.infer_expression(value);

                if let Some(ann) = type_annotation {
                    let expected = self.env.resolve_type_expr(ann);
                    if let Err(e) = unify::unify(&expected, &inferred, &mut self.subst) {
                        self.error(e);
                    }
                }

                self.env.define_variable(name, inferred, false);
            }

            Statement::Assignment { target, value } => {
                let target_ty = self.infer_expression(target);
                let value_ty = self.infer_expression(value);

                // Check mutability
                if let Expression::Identifier(name) = target {
                    match self.env.is_mutable(name) {
                        Some(false) => {
                            self.error(TypeError::ImmutableAssignment {
                                name: name.clone(),
                            });
                        }
                        None => {
                            self.error(TypeError::UndefinedVariable {
                                name: name.clone(),
                            });
                        }
                        _ => {}
                    }
                }

                if let Err(e) = unify::unify(&target_ty, &value_ty, &mut self.subst) {
                    self.error(e);
                }
            }

            Statement::If {
                condition,
                body,
                elif_clauses,
                else_body,
            } => {
                let cond_ty = self.infer_expression(condition);
                if let Err(_) = unify::unify(&Type::Bool, &cond_ty, &mut self.subst) {
                    self.error(TypeError::NonBoolCondition { found: cond_ty });
                }

                self.env.push_scope();
                for s in body {
                    self.check_statement(s);
                }
                self.env.pop_scope();

                for (elif_cond, elif_body) in elif_clauses {
                    let elif_ty = self.infer_expression(elif_cond);
                    if let Err(_) = unify::unify(&Type::Bool, &elif_ty, &mut self.subst) {
                        self.error(TypeError::NonBoolCondition { found: elif_ty });
                    }
                    self.env.push_scope();
                    for s in elif_body {
                        self.check_statement(s);
                    }
                    self.env.pop_scope();
                }

                if let Some(else_stmts) = else_body {
                    self.env.push_scope();
                    for s in else_stmts {
                        self.check_statement(s);
                    }
                    self.env.pop_scope();
                }
            }

            Statement::WhileLoop { condition, body } => {
                let cond_ty = self.infer_expression(condition);
                if let Err(_) = unify::unify(&Type::Bool, &cond_ty, &mut self.subst) {
                    self.error(TypeError::NonBoolCondition { found: cond_ty });
                }

                let was_in_loop = self.in_loop;
                self.in_loop = true;
                self.env.push_scope();
                for s in body {
                    self.check_statement(s);
                }
                self.env.pop_scope();
                self.in_loop = was_in_loop;
            }

            Statement::ForLoop {
                variable,
                iterable,
                body,
            } => {
                let iter_ty = self.infer_expression(iterable);

                // Infer the element type from the iterable
                let elem_ty = match &iter_ty {
                    Type::List(inner) => *inner.clone(),
                    Type::Str => Type::Char,
                    _ => Type::fresh_var(),
                };

                let was_in_loop = self.in_loop;
                self.in_loop = true;
                self.env.push_scope();
                self.env.define_variable(variable, elem_ty, false);
                for s in body {
                    self.check_statement(s);
                }
                self.env.pop_scope();
                self.in_loop = was_in_loop;
            }

            Statement::Return(expr) => {
                let return_ty = if let Some(e) = expr {
                    self.infer_expression(e)
                } else {
                    Type::None
                };

                if let Some(expected) = self.env.expected_return_type().cloned() {
                    if let Err(e) = unify::unify(&expected, &return_ty, &mut self.subst) {
                        self.error(e);
                    }
                }
            }

            Statement::Break => {
                if !self.in_loop {
                    self.error(TypeError::BreakOutsideLoop);
                }
            }

            Statement::Continue => {
                if !self.in_loop {
                    self.error(TypeError::ContinueOutsideLoop);
                }
            }

            Statement::Expression(expr) => {
                self.infer_expression(expr);
            }

            // Declarations handled in first pass
            Statement::StructDef { .. }
            | Statement::EnumDef { .. }
            | Statement::TraitDef { .. }
            | Statement::ImplBlock { .. }
            | Statement::Import { .. }
            | Statement::ForeignImport { .. }
            | Statement::Match { .. } => {}
        }
    }

    // ── Expression type inference ────────────────────────────

    fn infer_expression(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLiteral(_) => Type::Int,
            Expression::FloatLiteral(_) => Type::Float,
            Expression::StringLiteral(_) => Type::Str,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::NoneLiteral => Type::None,
            Expression::FString(_) => Type::Str,

            Expression::Identifier(name) => {
                if let Some((ty, _)) = self.env.lookup_variable(name) {
                    ty.clone()
                } else if let Some(sig) = self.env.lookup_function(name) {
                    // Function used as a value (e.g. passed to pipe)
                    Type::Function {
                        params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                        return_type: Box::new(sig.return_type.clone()),
                        effects: sig.effects.clone(),
                    }
                } else {
                    self.error(TypeError::UndefinedVariable {
                        name: name.clone(),
                    });
                    Type::Error
                }
            }

            Expression::BinaryOp { left, op, right } => {
                let left_ty = self.infer_expression(left);
                let right_ty = self.infer_expression(right);
                self.check_binary_op(&left_ty, *op, &right_ty)
            }

            Expression::UnaryOp { op, operand } => {
                let operand_ty = self.infer_expression(operand);
                match op {
                    UnaryOperator::Neg => {
                        if operand_ty.is_numeric() {
                            operand_ty
                        } else {
                            self.error(TypeError::InvalidNegation {
                                ty: operand_ty.clone(),
                            });
                            Type::Error
                        }
                    }
                    UnaryOperator::Not => {
                        if let Err(_) = unify::unify(&Type::Bool, &operand_ty, &mut self.subst) {
                            self.error(TypeError::NonBoolCondition {
                                found: operand_ty,
                            });
                        }
                        Type::Bool
                    }
                }
            }

            Expression::Call { function, args } => {
                let func_ty = self.infer_expression(function);
                let arg_types: Vec<Type> =
                    args.iter().map(|a| self.infer_expression(a)).collect();

                match &func_ty {
                    Type::Function {
                        params,
                        return_type,
                        effects,
                    } => {
                        // Check arity
                        if params.len() != arg_types.len() {
                            let name = if let Expression::Identifier(n) = function.as_ref() {
                                n.clone()
                            } else {
                                "<anonymous>".to_string()
                            };
                            self.error(TypeError::ArityMismatch {
                                name,
                                expected: params.len(),
                                found: arg_types.len(),
                            });
                        } else {
                            // Unify each argument with its parameter
                            for (param_ty, arg_ty) in params.iter().zip(arg_types.iter()) {
                                if let Err(e) =
                                    unify::unify(param_ty, arg_ty, &mut self.subst)
                                {
                                    self.error(e);
                                }
                            }
                        }

                        // Check effects
                        if self.env.is_in_pure_function() && !effects.is_empty() {
                            let name = if let Expression::Identifier(n) = function.as_ref() {
                                n.clone()
                            } else {
                                "<anonymous>".to_string()
                            };
                            self.error(TypeError::PurityViolation { name });
                        }

                        *return_type.clone()
                    }
                    Type::Var(_) => {
                        // Unknown function type — create fresh return var
                        let ret = Type::fresh_var();
                        let expected = Type::Function {
                            params: arg_types,
                            return_type: Box::new(ret.clone()),
                            effects: vec![],
                        };
                        if let Err(e) = unify::unify(&func_ty, &expected, &mut self.subst) {
                            self.error(e);
                        }
                        ret
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(TypeError::NotCallable {
                            ty: func_ty.clone(),
                        });
                        Type::Error
                    }
                }
            }

            Expression::FieldAccess { object, field } => {
                let obj_ty = self.infer_expression(object);
                match &obj_ty {
                    Type::Struct(s) => {
                        if let Some((_, field_ty)) =
                            s.fields.iter().find(|(name, _)| name == field)
                        {
                            field_ty.clone()
                        } else {
                            self.error(TypeError::NoSuchField {
                                ty: obj_ty.clone(),
                                field: field.clone(),
                            });
                            Type::Error
                        }
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(TypeError::NoSuchField {
                            ty: obj_ty.clone(),
                            field: field.clone(),
                        });
                        Type::Error
                    }
                }
            }

            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_ty = self.infer_expression(object);
                let _arg_types: Vec<Type> =
                    args.iter().map(|a| self.infer_expression(a)).collect();

                // Built-in methods on common types
                match (&obj_ty, method.as_str()) {
                    (Type::Str, "len") => Type::Int,
                    (Type::Str, "upper" | "lower" | "trim" | "strip") => Type::Str,
                    (Type::Str, "split") => Type::List(Box::new(Type::Str)),
                    (Type::Str, "contains" | "starts_with" | "ends_with") => Type::Bool,
                    (Type::List(_), "len") => Type::Int,
                    (Type::List(inner), "pop") => *inner.clone(),
                    (Type::List(_), "push" | "append") => Type::None,
                    (Type::List(_), "contains") => Type::Bool,
                    (Type::Error, _) => Type::Error,
                    _ => {
                        // Could be a trait method — return fresh var for now
                        Type::fresh_var()
                    }
                }
            }

            Expression::Index { object, index } => {
                let obj_ty = self.infer_expression(object);
                let idx_ty = self.infer_expression(index);

                match &obj_ty {
                    Type::List(inner) => {
                        if let Err(e) = unify::unify(&Type::Int, &idx_ty, &mut self.subst) {
                            self.error(e);
                        }
                        *inner.clone()
                    }
                    Type::Dict(key, val) => {
                        if let Err(e) = unify::unify(key, &idx_ty, &mut self.subst) {
                            self.error(e);
                        }
                        *val.clone()
                    }
                    Type::Str => {
                        if let Err(e) = unify::unify(&Type::Int, &idx_ty, &mut self.subst) {
                            self.error(e);
                        }
                        Type::Char
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.error(TypeError::NotIndexable {
                            ty: obj_ty.clone(),
                        });
                        Type::Error
                    }
                }
            }

            Expression::Pipe { left, right } => {
                let input_ty = self.infer_expression(left);
                // The right side of a pipe is a function applied to the left
                // data |> transform is sugar for transform(data)
                let func_ty = self.infer_expression(right);

                match &func_ty {
                    Type::Function {
                        return_type,
                        ..
                    } => *return_type.clone(),
                    _ => {
                        // Treat as function call with input as arg
                        let ret = Type::fresh_var();
                        let expected = Type::Function {
                            params: vec![input_ty],
                            return_type: Box::new(ret.clone()),
                            effects: vec![],
                        };
                        let _ = unify::unify(&func_ty, &expected, &mut self.subst);
                        ret
                    }
                }
            }

            Expression::Lambda { params, body } => {
                self.env.push_scope();

                let param_types: Vec<Type> = params
                    .iter()
                    .map(|name| {
                        let ty = Type::fresh_var();
                        self.env.define_variable(name, ty.clone(), false);
                        ty
                    })
                    .collect();

                let body_ty = self.infer_expression(body);

                self.env.pop_scope();

                Type::Function {
                    params: param_types,
                    return_type: Box::new(body_ty),
                    effects: vec![],
                }
            }

            Expression::List(elements) => {
                if elements.is_empty() {
                    Type::List(Box::new(Type::fresh_var()))
                } else {
                    let first_ty = self.infer_expression(&elements[0]);
                    for elem in &elements[1..] {
                        let elem_ty = self.infer_expression(elem);
                        if let Err(e) = unify::unify(&first_ty, &elem_ty, &mut self.subst) {
                            self.error(e);
                        }
                    }
                    Type::List(Box::new(first_ty))
                }
            }

            Expression::Dict(entries) => {
                if entries.is_empty() {
                    Type::Dict(Box::new(Type::fresh_var()), Box::new(Type::fresh_var()))
                } else {
                    let first_key_ty = self.infer_expression(&entries[0].0);
                    let first_val_ty = self.infer_expression(&entries[0].1);
                    for (k, v) in &entries[1..] {
                        let k_ty = self.infer_expression(k);
                        let v_ty = self.infer_expression(v);
                        if let Err(e) = unify::unify(&first_key_ty, &k_ty, &mut self.subst) {
                            self.error(e);
                        }
                        if let Err(e) = unify::unify(&first_val_ty, &v_ty, &mut self.subst) {
                            self.error(e);
                        }
                    }
                    Type::Dict(Box::new(first_key_ty), Box::new(first_val_ty))
                }
            }

            Expression::StructInit { name, fields } => {
                if let Some(ty) = self.env.lookup_type(name).cloned() {
                    if let Type::Struct(ref struct_ty) = ty {
                        // Check each provided field
                        for (field_name, field_expr) in fields {
                            let field_ty = self.infer_expression(field_expr);
                            if let Some((_, expected_ty)) = struct_ty
                                .fields
                                .iter()
                                .find(|(n, _)| n == field_name)
                            {
                                if let Err(e) =
                                    unify::unify(expected_ty, &field_ty, &mut self.subst)
                                {
                                    self.error(e);
                                }
                            } else {
                                self.error(TypeError::NoSuchField {
                                    ty: ty.clone(),
                                    field: field_name.clone(),
                                });
                            }
                        }
                    }
                    ty
                } else {
                    self.error(TypeError::UndefinedType {
                        name: name.clone(),
                    });
                    Type::Error
                }
            }

            Expression::Await(inner) => {
                let _inner_ty = self.infer_expression(inner);
                // Future<T> → T (simplified for now)
                Type::fresh_var()
            }

            Expression::ResultExpr { value, .. } => {
                let val_ty = self.infer_expression(value);
                Type::Result(Box::new(val_ty), Box::new(Type::Named("Error".into())))
            }
        }
    }

    // ── Binary operator type checking ────────────────────────

    fn check_binary_op(
        &mut self,
        left: &Type,
        op: BinaryOperator,
        right: &Type,
    ) -> Type {
        match op {
            // Arithmetic: numeric × numeric → numeric
            BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::Mod
            | BinaryOperator::Power => {
                // String concatenation
                if matches!(op, BinaryOperator::Add)
                    && matches!(left, Type::Str)
                    && matches!(right, Type::Str)
                {
                    return Type::Str;
                }

                // If either side is a type variable, try to unify with the other
                if matches!(left, Type::Var(_)) && right.is_numeric() {
                    let _ = unify::unify(left, right, &mut self.subst);
                    return right.clone();
                }
                if matches!(right, Type::Var(_)) && left.is_numeric() {
                    let _ = unify::unify(right, left, &mut self.subst);
                    return left.clone();
                }
                if matches!(left, Type::Var(_)) && matches!(right, Type::Var(_)) {
                    let _ = unify::unify(left, right, &mut self.subst);
                    return left.clone();
                }

                if !left.is_numeric() || !right.is_numeric() {
                    self.error(TypeError::InvalidOperator {
                        op: format!("{:?}", op),
                        left: left.clone(),
                        right: right.clone(),
                    });
                    return Type::Error;
                }

                // Float wins in mixed operations
                if matches!(left, Type::Float) || matches!(right, Type::Float) {
                    Type::Float
                } else {
                    Type::Int
                }
            }

            BinaryOperator::IntDiv => {
                if !left.is_numeric() || !right.is_numeric() {
                    self.error(TypeError::InvalidOperator {
                        op: "//".to_string(),
                        left: left.clone(),
                        right: right.clone(),
                    });
                    return Type::Error;
                }
                Type::Int
            }

            // Comparison: same type → bool
            BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::Gt
            | BinaryOperator::LtEq
            | BinaryOperator::GtEq => {
                if let Err(e) = unify::unify(left, right, &mut self.subst) {
                    self.error(e);
                }
                Type::Bool
            }

            // Logical: bool × bool → bool
            BinaryOperator::And | BinaryOperator::Or => {
                if let Err(_) = unify::unify(&Type::Bool, left, &mut self.subst) {
                    self.error(TypeError::NonBoolCondition {
                        found: left.clone(),
                    });
                }
                if let Err(_) = unify::unify(&Type::Bool, right, &mut self.subst) {
                    self.error(TypeError::NonBoolCondition {
                        found: right.clone(),
                    });
                }
                Type::Bool
            }

            // Membership: x in collection → bool
            BinaryOperator::In => Type::Bool,

            // Identity: x is y → bool
            BinaryOperator::Is => Type::Bool,
        }
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}
