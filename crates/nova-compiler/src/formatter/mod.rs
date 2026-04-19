use crate::ast::*;

/// Format a Nova program into consistently styled source code
pub fn format(program: &Program) -> String {
    let mut f = Formatter::new();
    f.format_program(program);
    f.output
}

struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    // ── Program ──────────────────────────────────────────────

    fn format_program(&mut self, program: &Program) {
        let mut prev_was_def = false;

        for (i, stmt) in program.statements.iter().enumerate() {
            let is_def = matches!(
                stmt,
                Statement::FunctionDef { .. }
                    | Statement::StructDef { .. }
                    | Statement::EnumDef { .. }
                    | Statement::TraitDef { .. }
                    | Statement::ImplBlock { .. }
            );

            // Blank line between definitions and before first def
            if (is_def && i > 0) || (prev_was_def && !is_def) {
                self.newline();
            }

            self.format_statement(stmt);
            prev_was_def = is_def;
        }
    }

    // ── Statements ───────────────────────────────────────────

    fn format_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FunctionDef {
                name,
                params,
                return_type,
                effects,
                body,
                decorators,
                is_pub,
                doc_comment,
            } => {
                if let Some(doc) = doc_comment {
                    for line in doc.lines() {
                        self.writeln(&format!("## {line}"));
                    }
                }
                for dec in decorators {
                    self.write_indent();
                    self.write(&format!("@{}", dec.name));
                    if !dec.args.is_empty() {
                        self.write("(");
                        self.write_expr_list(&dec.args);
                        self.write(")");
                    }
                    self.newline();
                }

                self.write_indent();
                if *is_pub {
                    self.write("pub ");
                }
                self.write(&format!("fn {name}("));
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&format!(
                        "{}: {}",
                        param.name,
                        self.format_type(&param.type_annotation)
                    ));
                }
                self.write(")");

                if let Some(ret) = return_type {
                    self.write(&format!(" -> {}", self.format_type(ret)));
                }

                if !effects.is_empty() {
                    self.write(&format!(" [{}]", effects.join(", ")));
                }

                self.write(":\n");
                self.indent += 1;
                self.format_block(body);
                self.indent -= 1;
            }

            Statement::StructDef {
                name,
                fields,
                is_pub,
                doc_comment,
            } => {
                if let Some(doc) = doc_comment {
                    for line in doc.lines() {
                        self.writeln(&format!("## {line}"));
                    }
                }
                self.write_indent();
                if *is_pub {
                    self.write("pub ");
                }
                self.write(&format!("struct {name}:\n"));
                self.indent += 1;
                for field in fields {
                    self.write_indent();
                    if field.is_pub {
                        self.write("pub ");
                    }
                    self.write(&format!(
                        "let {}: {}",
                        field.name,
                        self.format_type(&field.type_annotation)
                    ));
                    if let Some(default) = &field.default {
                        self.write(" = ");
                        self.write(&self.format_expr(default));
                    }
                    self.newline();
                }
                self.indent -= 1;
            }

            Statement::EnumDef {
                name,
                variants,
                is_pub,
                doc_comment,
            } => {
                if let Some(doc) = doc_comment {
                    for line in doc.lines() {
                        self.writeln(&format!("## {line}"));
                    }
                }
                self.write_indent();
                if *is_pub {
                    self.write("pub ");
                }
                self.write(&format!("enum {name}:\n"));
                self.indent += 1;
                for variant in variants {
                    self.write_indent();
                    self.write(&format!("case {}", variant.name));
                    if let Some(fields) = &variant.fields {
                        self.write("(");
                        let types: Vec<String> =
                            fields.iter().map(|t| self.format_type(t)).collect();
                        self.write(&types.join(", "));
                        self.write(")");
                    }
                    self.newline();
                }
                self.indent -= 1;
            }

            Statement::TraitDef {
                name,
                methods,
                is_pub,
                doc_comment,
            } => {
                if let Some(doc) = doc_comment {
                    for line in doc.lines() {
                        self.writeln(&format!("## {line}"));
                    }
                }
                self.write_indent();
                if *is_pub {
                    self.write("pub ");
                }
                self.write(&format!("trait {name}:\n"));
                self.indent += 1;
                for method in methods {
                    self.write_indent();
                    self.write(&format!("fn {}(", method.name));
                    let params: Vec<String> = method
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, self.format_type(&p.type_annotation)))
                        .collect();
                    self.write(&params.join(", "));
                    self.write(")");
                    if let Some(ret) = &method.return_type {
                        self.write(&format!(" -> {}", self.format_type(ret)));
                    }
                    self.newline();
                }
                self.indent -= 1;
            }

            Statement::ImplBlock {
                trait_name,
                target_type,
                methods,
            } => {
                self.write_indent();
                if let Some(trait_n) = trait_name {
                    self.write(&format!("impl {trait_n} for {target_type}:\n"));
                } else {
                    self.write(&format!("impl {target_type}:\n"));
                }
                self.indent += 1;
                for method in methods {
                    self.format_statement(method);
                }
                self.indent -= 1;
            }

            Statement::LetBinding {
                name,
                type_annotation,
                value,
                mutable,
            } => {
                self.write_indent();
                if *mutable {
                    self.write("let mut ");
                } else {
                    self.write("let ");
                }
                self.write(name);
                if let Some(ty) = type_annotation {
                    self.write(&format!(": {}", self.format_type(ty)));
                }
                self.write(" = ");
                self.write(&self.format_expr(value));
                self.newline();
            }

            Statement::ConstBinding {
                name,
                type_annotation,
                value,
            } => {
                self.write_indent();
                self.write(&format!("const {name}"));
                if let Some(ty) = type_annotation {
                    self.write(&format!(": {}", self.format_type(ty)));
                }
                self.write(" = ");
                self.write(&self.format_expr(value));
                self.newline();
            }

            Statement::Assignment { target, value } => {
                self.write_indent();
                self.write(&self.format_expr(target));
                self.write(" = ");
                self.write(&self.format_expr(value));
                self.newline();
            }

            Statement::If {
                condition,
                body,
                elif_clauses,
                else_body,
            } => {
                self.write_indent();
                self.write(&format!("if {}:\n", self.format_expr(condition)));
                self.indent += 1;
                self.format_block(body);
                self.indent -= 1;

                for (cond, block) in elif_clauses {
                    self.write_indent();
                    self.write(&format!("elif {}:\n", self.format_expr(cond)));
                    self.indent += 1;
                    self.format_block(block);
                    self.indent -= 1;
                }

                if let Some(else_stmts) = else_body {
                    self.writeln("else:");
                    self.indent += 1;
                    self.format_block(else_stmts);
                    self.indent -= 1;
                }
            }

            Statement::ForLoop {
                variable,
                iterable,
                body,
            } => {
                self.write_indent();
                self.write(&format!(
                    "for {variable} in {}:\n",
                    self.format_expr(iterable)
                ));
                self.indent += 1;
                self.format_block(body);
                self.indent -= 1;
            }

            Statement::WhileLoop { condition, body } => {
                self.write_indent();
                self.write(&format!("while {}:\n", self.format_expr(condition)));
                self.indent += 1;
                self.format_block(body);
                self.indent -= 1;
            }

            Statement::Return(expr) => {
                self.write_indent();
                if let Some(e) = expr {
                    self.write(&format!("return {}", self.format_expr(e)));
                } else {
                    self.write("return");
                }
                self.newline();
            }

            Statement::Break => self.writeln("break"),
            Statement::Continue => self.writeln("continue"),

            Statement::Expression(expr) => {
                self.write_indent();
                self.write(&self.format_expr(expr));
                self.newline();
            }

            Statement::Import { path, items } => {
                self.write_indent();
                if let Some(items) = items {
                    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
                    self.write(&format!(
                        "from {} import {}",
                        path.join("."),
                        names.join(", ")
                    ));
                } else {
                    self.write(&format!("import {}", path.join(".")));
                }
                self.newline();
            }

            Statement::ForeignImport { path, lang, .. } => {
                self.write_indent();
                self.write(&format!("import foreign(\"{path}\", lang: \"{lang}\")"));
                self.newline();
            }

            Statement::Match { subject, arms } => {
                self.write_indent();
                self.write(&format!("match {}:\n", self.format_expr(subject)));
                self.indent += 1;
                for arm in arms {
                    self.write_indent();
                    self.write(&format!("case {}:\n", self.format_pattern(&arm.pattern)));
                    self.indent += 1;
                    self.format_block(&arm.body);
                    self.indent -= 1;
                }
                self.indent -= 1;
            }
        }
    }

    fn format_block(&mut self, block: &Block) {
        if block.is_empty() {
            self.writeln("pass");
        } else {
            for stmt in block {
                self.format_statement(stmt);
            }
        }
    }

    // ── Expressions ──────────────────────────────────────────

    fn format_expr(&self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(n) => {
                if *n == n.floor() && n.is_finite() {
                    format!("{n:.1}")
                } else {
                    n.to_string()
                }
            }
            Expression::StringLiteral(s) => format!("\"{s}\""),
            Expression::BoolLiteral(b) => (if *b { "true" } else { "false" }).to_string(),
            Expression::NoneLiteral => "none".to_string(),
            Expression::Identifier(name) => name.clone(),

            Expression::BinaryOp { left, op, right } => {
                let op_str = match op {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Sub => "-",
                    BinaryOperator::Mul => "*",
                    BinaryOperator::Div => "/",
                    BinaryOperator::IntDiv => "//",
                    BinaryOperator::Mod => "%",
                    BinaryOperator::Power => "**",
                    BinaryOperator::Eq => "==",
                    BinaryOperator::NotEq => "!=",
                    BinaryOperator::Lt => "<",
                    BinaryOperator::Gt => ">",
                    BinaryOperator::LtEq => "<=",
                    BinaryOperator::GtEq => ">=",
                    BinaryOperator::And => "and",
                    BinaryOperator::Or => "or",
                    BinaryOperator::In => "in",
                    BinaryOperator::Is => "is",
                };
                format!(
                    "{} {op_str} {}",
                    self.format_expr(left),
                    self.format_expr(right)
                )
            }

            Expression::UnaryOp { op, operand } => {
                let op_str = match op {
                    UnaryOperator::Neg => "-",
                    UnaryOperator::Not => "not ",
                };
                format!("{op_str}{}", self.format_expr(operand))
            }

            Expression::Call { function, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                format!("{}({})", self.format_expr(function), args_str.join(", "))
            }

            Expression::MethodCall {
                object,
                method,
                args,
            } => {
                let args_str: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                format!(
                    "{}.{method}({})",
                    self.format_expr(object),
                    args_str.join(", ")
                )
            }

            Expression::FieldAccess { object, field } => {
                format!("{}.{field}", self.format_expr(object))
            }

            Expression::Index { object, index } => {
                format!("{}[{}]", self.format_expr(object), self.format_expr(index))
            }

            Expression::Pipe { left, right } => {
                format!("{} |> {}", self.format_expr(left), self.format_expr(right))
            }

            Expression::Lambda { params, body } => {
                if params.len() == 1 {
                    format!("{} => {}", params[0], self.format_expr(body))
                } else {
                    format!("({}) => {}", params.join(", "), self.format_expr(body))
                }
            }

            Expression::List(elements) => {
                let items: Vec<String> = elements.iter().map(|e| self.format_expr(e)).collect();
                format!("[{}]", items.join(", "))
            }

            Expression::Dict(entries) => {
                let items: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", self.format_expr(k), self.format_expr(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }

            Expression::StructInit { name, fields } => {
                let items: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", self.format_expr(v)))
                    .collect();
                format!("{name} {{ {} }}", items.join(", "))
            }

            Expression::FString(parts) => {
                let mut result = String::from("f\"");
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => result.push_str(s),
                        FStringPart::Expression(e) => {
                            result.push('{');
                            result.push_str(&self.format_expr(e));
                            result.push('}');
                        }
                    }
                }
                result.push('"');
                result
            }

            Expression::Await(inner) => format!("await {}", self.format_expr(inner)),

            Expression::ResultExpr {
                value, error_type, ..
            } => {
                format!("{} or {error_type}", self.format_expr(value))
            }
        }
    }

    fn write_expr_list(&mut self, exprs: &[Expression]) {
        let items: Vec<String> = exprs.iter().map(|e| self.format_expr(e)).collect();
        self.write(&items.join(", "));
    }

    // ── Types ────────────────────────────────────────────────

    fn format_type(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Named(name) => name.clone(),
            TypeExpr::Generic(name, args) => {
                let args_str: Vec<String> = args.iter().map(|a| self.format_type(a)).collect();
                format!("{name}[{}]", args_str.join(", "))
            }
            TypeExpr::Function(params, ret) => {
                let params_str: Vec<String> = params.iter().map(|p| self.format_type(p)).collect();
                format!("({}) -> {}", params_str.join(", "), self.format_type(ret))
            }
            TypeExpr::Optional(inner) => format!("{}?", self.format_type(inner)),
            TypeExpr::Result(ok, err) => {
                format!("{} or {}", self.format_type(ok), self.format_type(err))
            }
            TypeExpr::Tuple(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| self.format_type(e)).collect();
                format!("({})", elems.join(", "))
            }
        }
    }

    // ── Patterns ─────────────────────────────────────────────

    fn format_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Literal(expr) => self.format_expr(expr),
            Pattern::Variable(name) => name.clone(),
            Pattern::Variant(name, fields) => {
                let fields_str: Vec<String> =
                    fields.iter().map(|p| self.format_pattern(p)).collect();
                if fields_str.is_empty() {
                    name.clone()
                } else {
                    format!("{name}({})", fields_str.join(", "))
                }
            }
            Pattern::Wildcard => "_".to_string(),
        }
    }
}
