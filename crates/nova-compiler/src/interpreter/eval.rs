use std::collections::HashMap;

use crate::ast::*;

use super::env::RuntimeEnv;
use super::value::{NovaFunction, RuntimeError, Value};

/// The Nova tree-walking interpreter
pub struct Interpreter {
    env: RuntimeEnv,
    /// Output buffer (captured for testing, printed in CLI)
    pub output: Vec<String>,
}

type IResult = Result<Value, RuntimeError>;

impl Interpreter {
    pub fn new() -> Self {
        let mut interp = Self {
            env: RuntimeEnv::new(),
            output: Vec::new(),
        };
        interp.register_builtins();
        interp
    }

    /// Execute an entire program
    pub fn execute(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // First pass: register all functions and type definitions
        for stmt in &program.statements {
            self.register_declaration(stmt);
        }

        // Second pass: execute top-level statements
        for stmt in &program.statements {
            match stmt {
                // Skip declarations (already registered)
                Statement::FunctionDef { .. }
                | Statement::StructDef { .. }
                | Statement::EnumDef { .. }
                | Statement::TraitDef { .. }
                | Statement::ImplBlock { .. } => {}
                _ => {
                    self.exec_statement(stmt)?;
                }
            }
        }

        // If there's a main function, call it
        if let Some(main_val) = self.env.get("main").cloned() {
            if let Value::Function(func) = main_val {
                self.call_function(&func, vec![])?;
            }
        }

        Ok(())
    }

    // ── Built-in functions ───────────────────────────────────

    fn register_builtins(&mut self) {
        // print
        self.env.define(
            "print",
            Value::Function(NovaFunction::Builtin {
                name: "print".to_string(),
                func: |_args| Ok(Value::None), // actual printing handled in call_function
            }),
            false,
        );

        // len
        self.env.define(
            "len",
            Value::Function(NovaFunction::Builtin {
                name: "len".to_string(),
                func: |args| match args.first() {
                    Some(Value::List(items)) => Ok(Value::Int(items.len() as i64)),
                    Some(Value::Str(s)) => Ok(Value::Int(s.len() as i64)),
                    Some(Value::Dict(entries)) => Ok(Value::Int(entries.len() as i64)),
                    _ => Err(RuntimeError::TypeError {
                        message: "len() requires a list, string, or dict".to_string(),
                    }),
                },
            }),
            false,
        );

        // range
        self.env.define(
            "range",
            Value::Function(NovaFunction::Builtin {
                name: "range".to_string(),
                func: |args| match args.as_slice() {
                    [Value::Int(n)] => {
                        Ok(Value::List((0..*n).map(Value::Int).collect()))
                    }
                    [Value::Int(start), Value::Int(end)] => {
                        Ok(Value::List((*start..*end).map(Value::Int).collect()))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "range() requires 1 or 2 integer arguments".to_string(),
                    }),
                },
            }),
            false,
        );

        // str (conversion)
        self.env.define(
            "str",
            Value::Function(NovaFunction::Builtin {
                name: "str".to_string(),
                func: |args| match args.first() {
                    Some(val) => Ok(Value::Str(val.to_string())),
                    None => Ok(Value::Str(String::new())),
                },
            }),
            false,
        );

        // type
        self.env.define(
            "type",
            Value::Function(NovaFunction::Builtin {
                name: "type".to_string(),
                func: |args| match args.first() {
                    Some(Value::Int(_)) => Ok(Value::Str("int".to_string())),
                    Some(Value::Float(_)) => Ok(Value::Str("float".to_string())),
                    Some(Value::Bool(_)) => Ok(Value::Str("bool".to_string())),
                    Some(Value::Str(_)) => Ok(Value::Str("str".to_string())),
                    Some(Value::List(_)) => Ok(Value::Str("list".to_string())),
                    Some(Value::Dict(_)) => Ok(Value::Str("dict".to_string())),
                    Some(Value::Struct { type_name, .. }) => {
                        Ok(Value::Str(type_name.clone()))
                    }
                    Some(Value::Function(_)) => Ok(Value::Str("function".to_string())),
                    Some(Value::None) => Ok(Value::Str("none".to_string())),
                    _ => Ok(Value::Str("unknown".to_string())),
                },
            }),
            false,
        );

        // abs
        self.env.define(
            "abs",
            Value::Function(NovaFunction::Builtin {
                name: "abs".to_string(),
                func: |args| match args.first() {
                    Some(Value::Int(n)) => Ok(Value::Int(n.abs())),
                    Some(Value::Float(n)) => Ok(Value::Float(n.abs())),
                    _ => Err(RuntimeError::TypeError {
                        message: "abs() requires a number".to_string(),
                    }),
                },
            }),
            false,
        );

        // min / max
        self.env.define(
            "min",
            Value::Function(NovaFunction::Builtin {
                name: "min".to_string(),
                func: |args| match args.as_slice() {
                    [Value::Int(a), Value::Int(b)] => Ok(Value::Int(*a.min(b))),
                    [Value::Float(a), Value::Float(b)] => Ok(Value::Float(a.min(*b))),
                    [Value::List(items)] => {
                        items.iter().cloned().reduce(|a, b| {
                            match (&a, &b) {
                                (Value::Int(x), Value::Int(y)) => if x < y { a } else { b },
                                _ => a,
                            }
                        }).ok_or(RuntimeError::Error("min() of empty list".to_string()))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "min() requires numbers or a list".to_string(),
                    }),
                },
            }),
            false,
        );

        self.env.define(
            "max",
            Value::Function(NovaFunction::Builtin {
                name: "max".to_string(),
                func: |args| match args.as_slice() {
                    [Value::Int(a), Value::Int(b)] => Ok(Value::Int(*a.max(b))),
                    [Value::Float(a), Value::Float(b)] => Ok(Value::Float(a.max(*b))),
                    [Value::List(items)] => {
                        items.iter().cloned().reduce(|a, b| {
                            match (&a, &b) {
                                (Value::Int(x), Value::Int(y)) => if x > y { a } else { b },
                                _ => a,
                            }
                        }).ok_or(RuntimeError::Error("max() of empty list".to_string()))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "max() requires numbers or a list".to_string(),
                    }),
                },
            }),
            false,
        );

        // sum
        self.env.define(
            "sum",
            Value::Function(NovaFunction::Builtin {
                name: "sum".to_string(),
                func: |args| match args.first() {
                    Some(Value::List(items)) => {
                        let mut total: i64 = 0;
                        for item in items {
                            match item {
                                Value::Int(n) => total += n,
                                _ => {
                                    return Err(RuntimeError::TypeError {
                                        message: "sum() requires a list of numbers".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(Value::Int(total))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "sum() requires a list".to_string(),
                    }),
                },
            }),
            false,
        );

        // filter
        self.env.define("filter", Value::Function(NovaFunction::Builtin {
            name: "filter".to_string(),
            func: |_| Ok(Value::None), // handled specially in pipe evaluation
        }), false);

        // map
        self.env.define("map", Value::Function(NovaFunction::Builtin {
            name: "map".to_string(),
            func: |_| Ok(Value::None), // handled specially in pipe evaluation
        }), false);

        // sort
        self.env.define("sort", Value::Function(NovaFunction::Builtin {
            name: "sort".to_string(),
            func: |args| match args.first() {
                Some(Value::List(items)) => {
                    let mut sorted = items.clone();
                    sorted.sort_by(|a, b| {
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => x.cmp(y),
                            (Value::Str(x), Value::Str(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    Ok(Value::List(sorted))
                }
                _ => Err(RuntimeError::TypeError {
                    message: "sort() requires a list".to_string(),
                }),
            },
        }), false);

        // reverse
        self.env.define("reverse", Value::Function(NovaFunction::Builtin {
            name: "reverse".to_string(),
            func: |args| match args.first() {
                Some(Value::List(items)) => {
                    let mut reversed = items.clone();
                    reversed.reverse();
                    Ok(Value::List(reversed))
                }
                _ => Err(RuntimeError::TypeError {
                    message: "reverse() requires a list".to_string(),
                }),
            },
        }), false);
    }

    // ── Declaration registration ─────────────────────────────

    fn register_declaration(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDef {
                name,
                params,
                body,
                ..
            } => {
                let func = Value::Function(NovaFunction::UserDefined {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure_env: None,
                });
                self.env.define(name, func, false);
            }
            Statement::StructDef { name, fields, .. } => {
                // Register struct as a callable constructor
                let field_names: Vec<String> =
                    fields.iter().map(|f| f.name.clone()).collect();
                let struct_name = name.clone();
                self.env.define(
                    name,
                    Value::Function(NovaFunction::Builtin {
                        name: struct_name,
                        func: |_| Ok(Value::None), // handled in StructInit eval
                    }),
                    false,
                );
                // Store field names for validation
                let _ = field_names; // used during StructInit evaluation
            }
            _ => {}
        }
    }

    // ── Statement execution ──────────────────────────────────

    fn exec_statement(&mut self, stmt: &Statement) -> IResult {
        match stmt {
            Statement::Expression(expr) => self.eval_expression(expr),

            Statement::LetBinding {
                name,
                value,
                mutable,
                ..
            } => {
                let val = self.eval_expression(value)?;
                self.env.define(name, val, *mutable);
                Ok(Value::None)
            }

            Statement::ConstBinding { name, value, .. } => {
                let val = self.eval_expression(value)?;
                self.env.define(name, val, false);
                Ok(Value::None)
            }

            Statement::Assignment { target, value } => {
                let val = self.eval_expression(value)?;
                match target {
                    Expression::Identifier(name) => {
                        self.env.set(name, val).map_err(|msg| {
                            RuntimeError::Error(msg)
                        })?;
                    }
                    Expression::FieldAccess { object, field } => {
                        // Struct field assignment
                        if let Expression::Identifier(obj_name) = object.as_ref() {
                            if let Some(Value::Struct { type_name, fields }) =
                                self.env.get(obj_name).cloned()
                            {
                                let mut new_fields = fields;
                                new_fields.insert(field.clone(), val);
                                self.env
                                    .set(
                                        obj_name,
                                        Value::Struct {
                                            type_name,
                                            fields: new_fields,
                                        },
                                    )
                                    .map_err(|msg| RuntimeError::Error(msg))?;
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::Error(
                            "invalid assignment target".to_string(),
                        ));
                    }
                }
                Ok(Value::None)
            }

            Statement::If {
                condition,
                body,
                elif_clauses,
                else_body,
            } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    return self.exec_block(body);
                }

                for (elif_cond, elif_body) in elif_clauses {
                    let cond = self.eval_expression(elif_cond)?;
                    if cond.is_truthy() {
                        return self.exec_block(elif_body);
                    }
                }

                if let Some(else_stmts) = else_body {
                    return self.exec_block(else_stmts);
                }

                Ok(Value::None)
            }

            Statement::WhileLoop { condition, body } => {
                loop {
                    let cond = self.eval_expression(condition)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.exec_block(body) {
                        Err(RuntimeError::Break) => break,
                        Err(RuntimeError::Continue) => continue,
                        Err(e) => return Err(e),
                        _ => {}
                    }
                }
                Ok(Value::None)
            }

            Statement::ForLoop {
                variable,
                iterable,
                body,
            } => {
                let iter_val = self.eval_expression(iterable)?;
                let items = match iter_val {
                    Value::List(items) => items,
                    Value::Str(s) => s.chars().map(Value::Char).collect(),
                    _ => {
                        return Err(RuntimeError::TypeError {
                            message: "cannot iterate over this type".to_string(),
                        })
                    }
                };

                self.env.push_scope();
                self.env.define(variable, Value::None, true);

                for item in items {
                    self.env
                        .set(variable, item)
                        .map_err(|msg| RuntimeError::Error(msg))?;

                    match self.exec_block(body) {
                        Err(RuntimeError::Break) => break,
                        Err(RuntimeError::Continue) => continue,
                        Err(e) => {
                            self.env.pop_scope();
                            return Err(e);
                        }
                        _ => {}
                    }
                }

                self.env.pop_scope();
                Ok(Value::None)
            }

            Statement::Return(expr) => {
                let val = if let Some(e) = expr {
                    self.eval_expression(e)?
                } else {
                    Value::None
                };
                Err(RuntimeError::Return(val))
            }

            Statement::Break => Err(RuntimeError::Break),
            Statement::Continue => Err(RuntimeError::Continue),

            // Skip declarations in execution pass
            Statement::FunctionDef { .. }
            | Statement::StructDef { .. }
            | Statement::EnumDef { .. }
            | Statement::TraitDef { .. }
            | Statement::ImplBlock { .. }
            | Statement::Import { .. }
            | Statement::ForeignImport { .. }
            | Statement::Match { .. } => Ok(Value::None),
        }
    }

    /// Execute a block of statements in a new scope
    fn exec_block(&mut self, block: &Block) -> IResult {
        self.env.push_scope();
        let mut result = Value::None;
        for stmt in block {
            match self.exec_statement(stmt) {
                Ok(val) => result = val,
                Err(e) => {
                    self.env.pop_scope();
                    return Err(e);
                }
            }
        }
        self.env.pop_scope();
        Ok(result)
    }

    // ── Expression evaluation ────────────────────────────────

    fn eval_expression(&mut self, expr: &Expression) -> IResult {
        match expr {
            Expression::IntLiteral(n) => Ok(Value::Int(*n)),
            Expression::FloatLiteral(n) => Ok(Value::Float(*n)),
            Expression::StringLiteral(s) => Ok(Value::Str(s.clone())),
            Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),
            Expression::NoneLiteral => Ok(Value::None),
            Expression::FString(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => result.push_str(s),
                        FStringPart::Expression(e) => {
                            let val = self.eval_expression(e)?;
                            result.push_str(&val.to_string());
                        }
                    }
                }
                Ok(Value::Str(result))
            }

            Expression::Identifier(name) => {
                self.env.get(name).cloned().ok_or_else(|| {
                    RuntimeError::Undefined { name: name.clone() }
                })
            }

            Expression::BinaryOp { left, op, right } => {
                let lval = self.eval_expression(left)?;
                // Short-circuit for and/or
                match op {
                    BinaryOperator::And => {
                        if !lval.is_truthy() {
                            return Ok(lval);
                        }
                        return self.eval_expression(right);
                    }
                    BinaryOperator::Or => {
                        if lval.is_truthy() {
                            return Ok(lval);
                        }
                        return self.eval_expression(right);
                    }
                    _ => {}
                }
                let rval = self.eval_expression(right)?;
                self.eval_binary_op(&lval, *op, &rval)
            }

            Expression::UnaryOp { op, operand } => {
                let val = self.eval_expression(operand)?;
                match op {
                    UnaryOperator::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(n) => Ok(Value::Float(-n)),
                        _ => Err(RuntimeError::TypeError {
                            message: "cannot negate this type".to_string(),
                        }),
                    },
                    UnaryOperator::Not => Ok(Value::Bool(!val.is_truthy())),
                }
            }

            Expression::Call { function, args } => {
                // Special handling for filter/map with lambda args
                if let Expression::Identifier(name) = function.as_ref() {
                    if (name == "filter" || name == "map") && args.len() == 1 {
                        // These are partially applied — will be resolved in pipe
                        let arg = self.eval_expression(&args[0])?;
                        if let Value::Function(inner) = arg {
                            return Ok(Value::Function(NovaFunction::Partial {
                                func: Box::new(NovaFunction::Builtin {
                                    name: name.clone(),
                                    func: |_| Ok(Value::None),
                                }),
                                applied_args: vec![Value::Function(inner)],
                            }));
                        }
                    }
                }

                let func_val = self.eval_expression(function)?;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expression(arg)?);
                }

                match func_val {
                    Value::Function(func) => self.call_function(&func, arg_vals),
                    _ => Err(RuntimeError::TypeError {
                        message: format!("cannot call {func_val}"),
                    }),
                }
            }

            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                let obj = self.eval_expression(object)?;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expression(arg)?);
                }
                self.eval_method_call(&obj, method, arg_vals)
            }

            Expression::FieldAccess { object, field } => {
                let obj = self.eval_expression(object)?;
                match &obj {
                    Value::Struct { fields, .. } => fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| RuntimeError::Error(format!("no field `{field}`"))),
                    _ => Err(RuntimeError::TypeError {
                        message: format!("{obj} has no field `{field}`"),
                    }),
                }
            }

            Expression::Index { object, index } => {
                let obj = self.eval_expression(object)?;
                let idx = self.eval_expression(index)?;
                match (&obj, &idx) {
                    (Value::List(items), Value::Int(idx)) => {
                        let i = if *idx < 0 {
                            (items.len() as i64 + idx) as usize
                        } else {
                            *idx as usize
                        };
                        items.get(i).cloned().ok_or(RuntimeError::IndexOutOfBounds {
                            index: i as i64,
                            length: items.len(),
                        })
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let i = *i as usize;
                        s.chars()
                            .nth(i)
                            .map(Value::Char)
                            .ok_or(RuntimeError::IndexOutOfBounds {
                                index: i as i64,
                                length: s.len(),
                            })
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "invalid index operation".to_string(),
                    }),
                }
            }

            Expression::Pipe { left, right } => {
                let input = self.eval_expression(left)?;
                self.eval_pipe(input, right)
            }

            Expression::Lambda { params, body } => {
                let closure = self.env.capture();
                Ok(Value::Function(NovaFunction::Lambda {
                    params: params.clone(),
                    body: *body.clone(),
                    closure_env: Some(closure),
                }))
            }

            Expression::List(elements) => {
                let mut items = Vec::new();
                for elem in elements {
                    items.push(self.eval_expression(elem)?);
                }
                Ok(Value::List(items))
            }

            Expression::Dict(entries) => {
                let mut items = Vec::new();
                for (k, v) in entries {
                    let key = self.eval_expression(k)?;
                    let val = self.eval_expression(v)?;
                    items.push((key, val));
                }
                Ok(Value::Dict(items))
            }

            Expression::StructInit { name, fields } => {
                let mut field_map = HashMap::new();
                for (fname, fexpr) in fields {
                    let val = self.eval_expression(fexpr)?;
                    field_map.insert(fname.clone(), val);
                }
                Ok(Value::Struct {
                    type_name: name.clone(),
                    fields: field_map,
                })
            }

            Expression::Await(inner) => {
                // In the interpreter, await just evaluates the expression
                self.eval_expression(inner)
            }

            Expression::ResultExpr { value, .. } => self.eval_expression(value),
        }
    }

    // ── Pipe evaluation ──────────────────────────────────────

    fn eval_pipe(&mut self, input: Value, right: &Expression) -> IResult {
        // Evaluate the right side to get the function
        let func_val = self.eval_expression(right)?;

        match func_val {
            // Partial application: filter(predicate) or map(transform)
            Value::Function(NovaFunction::Partial {
                func,
                applied_args,
            }) => {
                if let NovaFunction::Builtin { name, .. } = *func {
                    match name.as_str() {
                        "filter" => {
                            if let Some(Value::Function(pred)) = applied_args.first() {
                                if let Value::List(items) = input {
                                    let mut result = Vec::new();
                                    for item in items {
                                        let keep =
                                            self.call_function(pred, vec![item.clone()])?;
                                        if keep.is_truthy() {
                                            result.push(item);
                                        }
                                    }
                                    return Ok(Value::List(result));
                                }
                            }
                        }
                        "map" => {
                            if let Some(Value::Function(transform)) = applied_args.first()
                            {
                                if let Value::List(items) = input {
                                    let mut result = Vec::new();
                                    for item in items {
                                        let mapped = self.call_function(
                                            transform,
                                            vec![item],
                                        )?;
                                        result.push(mapped);
                                    }
                                    return Ok(Value::List(result));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(RuntimeError::TypeError {
                    message: "invalid pipe operation".to_string(),
                })
            }

            // Direct function: data |> sort, data |> sum
            Value::Function(func) => self.call_function(&func, vec![input]),

            _ => Err(RuntimeError::TypeError {
                message: format!("cannot pipe into {func_val}"),
            }),
        }
    }

    // ── Function calls ───────────────────────────────────────

    fn call_function(&mut self, func: &NovaFunction, args: Vec<Value>) -> IResult {
        match func {
            NovaFunction::UserDefined {
                params, body, ..
            } => {
                self.env.push_scope();

                // Bind parameters
                for (param, arg) in params.iter().zip(args.iter()) {
                    self.env.define(&param.name, arg.clone(), false);
                }

                // Execute body
                let mut result = Value::None;
                for stmt in body {
                    match self.exec_statement(stmt) {
                        Ok(val) => result = val,
                        Err(RuntimeError::Return(val)) => {
                            self.env.pop_scope();
                            return Ok(val);
                        }
                        Err(e) => {
                            self.env.pop_scope();
                            return Err(e);
                        }
                    }
                }

                self.env.pop_scope();
                Ok(result)
            }

            NovaFunction::Lambda {
                params,
                body,
                closure_env,
            } => {
                self.env.push_scope();

                // Restore closure environment
                if let Some(captured) = closure_env {
                    for (name, val) in captured {
                        self.env.define(name, val.clone(), false);
                    }
                }

                // Bind parameters
                for (name, arg) in params.iter().zip(args.iter()) {
                    self.env.define(name, arg.clone(), false);
                }

                let result = self.eval_expression(body)?;
                self.env.pop_scope();
                Ok(result)
            }

            NovaFunction::Builtin { name, func: f } => {
                // Special handling for print — capture output
                if name == "print" {
                    let output: Vec<String> =
                        args.iter().map(|a| a.to_string()).collect();
                    let line = output.join(" ");
                    self.output.push(line);
                    return Ok(Value::None);
                }
                f(args)
            }

            NovaFunction::Partial {
                func,
                applied_args,
            } => {
                let mut all_args = applied_args.clone();
                all_args.extend(args);
                self.call_function(func, all_args)
            }
        }
    }

    // ── Binary operators ─────────────────────────────────────

    fn eval_binary_op(
        &self,
        left: &Value,
        op: BinaryOperator,
        right: &Value,
    ) -> IResult {
        match op {
            BinaryOperator::Add => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                (Value::List(a), Value::List(b)) => {
                    let mut result = a.clone();
                    result.extend(b.clone());
                    Ok(Value::List(result))
                }
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot add {left} and {right}"),
                }),
            },
            BinaryOperator::Sub => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot subtract {right} from {left}"),
                }),
            },
            BinaryOperator::Mul => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
                (Value::Str(s), Value::Int(n)) | (Value::Int(n), Value::Str(s)) => {
                    Ok(Value::Str(s.repeat(*n as usize)))
                }
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot multiply {left} and {right}"),
                }),
            },
            BinaryOperator::Div => match (left, right) {
                (_, Value::Int(0)) => Err(RuntimeError::DivisionByZero),
                (_, Value::Float(n)) if *n == 0.0 => Err(RuntimeError::DivisionByZero),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot divide {left} by {right}"),
                }),
            },
            BinaryOperator::IntDiv => match (left, right) {
                (_, Value::Int(0)) => Err(RuntimeError::DivisionByZero),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
                _ => Err(RuntimeError::TypeError {
                    message: "integer division requires integers".to_string(),
                }),
            },
            BinaryOperator::Mod => match (left, right) {
                (_, Value::Int(0)) => Err(RuntimeError::DivisionByZero),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot modulo {left} by {right}"),
                }),
            },
            BinaryOperator::Power => match (left, right) {
                (Value::Int(a), Value::Int(b)) => {
                    Ok(Value::Int((*a as f64).powi(*b as i32) as i64))
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                (Value::Int(a), Value::Float(b)) => {
                    Ok(Value::Float((*a as f64).powf(*b)))
                }
                (Value::Float(a), Value::Int(b)) => {
                    Ok(Value::Float(a.powi(*b as i32)))
                }
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot raise {left} to {right}"),
                }),
            },
            BinaryOperator::Eq => Ok(Value::Bool(self.values_equal(left, right))),
            BinaryOperator::NotEq => Ok(Value::Bool(!self.values_equal(left, right))),
            BinaryOperator::Lt => self.compare_values(left, right, |ord| {
                ord == std::cmp::Ordering::Less
            }),
            BinaryOperator::Gt => self.compare_values(left, right, |ord| {
                ord == std::cmp::Ordering::Greater
            }),
            BinaryOperator::LtEq => self.compare_values(left, right, |ord| {
                ord != std::cmp::Ordering::Greater
            }),
            BinaryOperator::GtEq => self.compare_values(left, right, |ord| {
                ord != std::cmp::Ordering::Less
            }),
            BinaryOperator::In => match right {
                Value::List(items) => Ok(Value::Bool(
                    items.iter().any(|item| self.values_equal(left, item)),
                )),
                Value::Str(s) => {
                    if let Value::Str(needle) = left {
                        Ok(Value::Bool(s.contains(needle.as_str())))
                    } else {
                        Ok(Value::Bool(false))
                    }
                }
                _ => Ok(Value::Bool(false)),
            },
            BinaryOperator::Is => {
                Ok(Value::Bool(std::mem::discriminant(left) == std::mem::discriminant(right)))
            }
            // And/Or handled via short-circuit above
            BinaryOperator::And | BinaryOperator::Or => unreachable!(),
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::None, Value::None) => true,
            _ => false,
        }
    }

    fn compare_values(
        &self,
        a: &Value,
        b: &Value,
        pred: impl Fn(std::cmp::Ordering) -> bool,
    ) -> IResult {
        let ord = match (a, b) {
            (Value::Int(x), Value::Int(y)) => x.cmp(y),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(x), Value::Str(y)) => x.cmp(y),
            _ => {
                return Err(RuntimeError::TypeError {
                    message: format!("cannot compare {a} and {b}"),
                })
            }
        };
        Ok(Value::Bool(pred(ord)))
    }

    // ── Method calls ─────────────────────────────────────────

    fn eval_method_call(
        &mut self,
        object: &Value,
        method: &str,
        args: Vec<Value>,
    ) -> IResult {
        match (object, method) {
            (Value::Str(s), "len") => Ok(Value::Int(s.len() as i64)),
            (Value::Str(s), "upper") => Ok(Value::Str(s.to_uppercase())),
            (Value::Str(s), "lower") => Ok(Value::Str(s.to_lowercase())),
            (Value::Str(s), "trim") => Ok(Value::Str(s.trim().to_string())),
            (Value::Str(s), "contains") => {
                if let Some(Value::Str(needle)) = args.first() {
                    Ok(Value::Bool(s.contains(needle.as_str())))
                } else {
                    Err(RuntimeError::TypeError {
                        message: "contains() requires a string argument".to_string(),
                    })
                }
            }
            (Value::Str(s), "starts_with") => {
                if let Some(Value::Str(prefix)) = args.first() {
                    Ok(Value::Bool(s.starts_with(prefix.as_str())))
                } else {
                    Err(RuntimeError::TypeError {
                        message: "starts_with() requires a string".to_string(),
                    })
                }
            }
            (Value::Str(s), "ends_with") => {
                if let Some(Value::Str(suffix)) = args.first() {
                    Ok(Value::Bool(s.ends_with(suffix.as_str())))
                } else {
                    Err(RuntimeError::TypeError {
                        message: "ends_with() requires a string".to_string(),
                    })
                }
            }
            (Value::Str(s), "split") => {
                let sep = match args.first() {
                    Some(Value::Str(sep)) => sep.as_str(),
                    _ => " ",
                };
                Ok(Value::List(
                    s.split(sep).map(|part| Value::Str(part.to_string())).collect(),
                ))
            }
            (Value::Str(s), "replace") => {
                if let (Some(Value::Str(from)), Some(Value::Str(to))) =
                    (args.first(), args.get(1))
                {
                    Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
                } else {
                    Err(RuntimeError::TypeError {
                        message: "replace() requires two string arguments".to_string(),
                    })
                }
            }
            (Value::List(items), "len") => Ok(Value::Int(items.len() as i64)),
            (Value::List(items), "contains") => {
                if let Some(needle) = args.first() {
                    Ok(Value::Bool(
                        items.iter().any(|item| self.values_equal(item, needle)),
                    ))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            _ => Err(RuntimeError::Error(format!(
                "`{object}` has no method `{method}`"
            ))),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
