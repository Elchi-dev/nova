// chumsky's Simple<Token> error type is inherently large — boxing adds overhead.
#![allow(clippy::result_large_err)]

use chumsky::prelude::*;
use nova_lexer::{Span, Token};

use crate::ast::*;

// ── Type aliases ──────────────────────────────────────────────────────────────

pub type ParseErr = Simple<Token>;

// ── Span helpers ──────────────────────────────────────────────────────────────

fn s(r: std::ops::Range<usize>) -> Span {
    Span::from(r)
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, sp)
        | Expr::Float(_, sp)
        | Expr::Str(_, sp)
        | Expr::Bool(_, sp)
        | Expr::Ident(_, sp)
        | Expr::Array(_, sp)
        | Expr::FStr(_, sp) => *sp,
        Expr::BinOp { span, .. }
        | Expr::UnaryOp { span, .. }
        | Expr::Call { span, .. }
        | Expr::Field { span, .. }
        | Expr::Index { span, .. } => *span,
    }
}

// ── Basic building blocks ─────────────────────────────────────────────────────

/// Match and discard zero or more Newline tokens.
fn nl() -> impl Parser<Token, (), Error = ParseErr> + Clone {
    just(Token::Newline).repeated().ignored()
}

/// Match an identifier token and return its string value.
fn ident_tok() -> impl Parser<Token, String, Error = ParseErr> + Clone {
    filter_map(|span, tok: Token| match tok {
        Token::Ident(name) => Ok(name),
        other => Err(Simple::expected_input_found(span, [], Some(other))),
    })
}

/// Match any primitive type keyword and return it as a `TypeExpr::Named`.
fn primitive_type() -> impl Parser<Token, TypeExpr, Error = ParseErr> + Clone {
    filter_map(|span, tok: Token| {
        let name = match tok {
            Token::TypeI8 => "i8",
            Token::TypeI16 => "i16",
            Token::TypeI32 => "i32",
            Token::TypeI64 => "i64",
            Token::TypeU8 => "u8",
            Token::TypeU16 => "u16",
            Token::TypeU32 => "u32",
            Token::TypeU64 => "u64",
            Token::TypeF32 => "f32",
            Token::TypeF64 => "f64",
            Token::TypeBool => "bool",
            Token::TypeStr => "str",
            Token::TypeVoid => "void",
            other => return Err(Simple::expected_input_found(span, [], Some(other))),
        };
        Ok(TypeExpr::Named(name.to_string(), s(span)))
    })
}

/// Match a type expression: `[]T` or a named type.
fn type_expr() -> impl Parser<Token, TypeExpr, Error = ParseErr> + Clone {
    let named =
        primitive_type().or(ident_tok().map_with_span(|name, sp| TypeExpr::Named(name, s(sp))));

    // []T  (array type)
    let array = just(Token::LBracket)
        .then_ignore(just(Token::RBracket))
        .ignore_then(named.clone())
        .map_with_span(|inner, sp| TypeExpr::Array(Box::new(inner), s(sp)));

    array.or(named)
}

/// Match zero or more `@decorator[(args)]` lines.
fn decorators() -> impl Parser<Token, Vec<Decorator>, Error = ParseErr> + Clone {
    let kv_arg = ident_tok()
        .then_ignore(just(Token::Eq))
        .then(filter_map(|span, tok: Token| match tok {
            Token::Str(v) => Ok(v),
            Token::Ident(v) => Ok(v),
            other => Err(Simple::expected_input_found(span, [], Some(other))),
        }))
        .map(|(key, value)| DecoratorArg::KeyValue { key, value });

    let bare_arg = ident_tok().map(DecoratorArg::Ident);

    let arg_list = kv_arg
        .or(bare_arg)
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .delimited_by(just(Token::LParen), just(Token::RParen));

    filter_map(|span, tok: Token| match tok {
        Token::Decorator(name) => Ok((name, s(span))),
        other => Err(Simple::expected_input_found(span, [], Some(other))),
    })
    .then(arg_list.or_not().map(|a| a.unwrap_or_default()))
    .then_ignore(just(Token::Newline).or_not())
    .map(|((name, span), args)| Decorator { name, args, span })
    .repeated()
}

// ── Expression parser ─────────────────────────────────────────────────────────

pub fn expr_parser() -> impl Parser<Token, Expr, Error = ParseErr> + Clone {
    recursive(|expr| {
        // ── Atoms ─────────────────────────────────────────────────────────────

        let int_lit = filter_map(|sp, tok: Token| match tok {
            Token::Int(n) => Ok(Expr::Int(n, s(sp))),
            other => Err(Simple::expected_input_found(sp, [], Some(other))),
        });

        let float_lit = filter_map(|sp, tok: Token| match tok {
            Token::Float(f) => Ok(Expr::Float(f, s(sp))),
            other => Err(Simple::expected_input_found(sp, [], Some(other))),
        });

        let str_lit = filter_map(|sp, tok: Token| match tok {
            Token::Str(st) => Ok(Expr::Str(st, s(sp))),
            other => Err(Simple::expected_input_found(sp, [], Some(other))),
        });

        let bool_lit = just(Token::True)
            .map_with_span(|_, sp| Expr::Bool(true, s(sp)))
            .or(just(Token::False).map_with_span(|_, sp| Expr::Bool(false, s(sp))));

        let ident_expr = ident_tok().map_with_span(|name, sp| Expr::Ident(name, s(sp)));

        // [elem, elem, ...]
        let array_lit = expr
            .clone()
            .separated_by(just(Token::Comma).padded_by(nl()))
            .allow_trailing()
            .delimited_by(
                just(Token::LBracket).then_ignore(nl()),
                nl().ignore_then(just(Token::RBracket)),
            )
            .map_with_span(|elems, sp| Expr::Array(elems, s(sp)));

        // (expr)
        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let primary = int_lit
            .or(float_lit)
            .or(str_lit)
            .or(bool_lit)
            .or(array_lit)
            .or(paren_expr)
            .or(ident_expr);

        // ── Postfix: call / field / index ─────────────────────────────────────

        let arg_list = expr
            .clone()
            .separated_by(just(Token::Comma).padded_by(nl()))
            .allow_trailing()
            .delimited_by(
                just(Token::LParen).then_ignore(nl()),
                nl().ignore_then(just(Token::RParen)),
            );

        let postfix = primary
            .then(
                choice((
                    arg_list.map_with_span(|args, sp| PostfixOp::Call(args, s(sp))),
                    just(Token::Dot)
                        .ignore_then(ident_tok())
                        .map_with_span(|field, sp| PostfixOp::Field(field, s(sp))),
                    expr.clone()
                        .delimited_by(just(Token::LBracket), just(Token::RBracket))
                        .map_with_span(|idx, sp| PostfixOp::Index(Box::new(idx), s(sp))),
                ))
                .repeated(),
            )
            .foldl(|obj, op| {
                let obj_sp = expr_span(&obj);
                match op {
                    PostfixOp::Call(args, sp) => Expr::Call {
                        callee: Box::new(obj),
                        args,
                        span: obj_sp.merge(sp),
                    },
                    PostfixOp::Field(field, sp) => Expr::Field {
                        object: Box::new(obj),
                        field,
                        span: obj_sp.merge(sp),
                    },
                    PostfixOp::Index(index, sp) => Expr::Index {
                        object: Box::new(obj),
                        index,
                        span: obj_sp.merge(sp),
                    },
                }
            });

        // ── Unary: -x  !x ────────────────────────────────────────────────────

        let unary_op = just(Token::Minus)
            .to(UnaryOp::Neg)
            .or(just(Token::Bang).to(UnaryOp::Not));

        let unary = unary_op
            .repeated()
            .then(postfix)
            .map_with_span(|(ops, expr), sp| {
                ops.into_iter().rev().fold(expr, |inner, op| Expr::UnaryOp {
                    op,
                    expr: Box::new(inner),
                    span: s(sp.clone()),
                })
            });

        // ── Binary operators (lowest to highest precedence) ───────────────────

        macro_rules! binop_level {
            ($operand:expr, $op_parser:expr) => {
                $operand
                    .clone()
                    .then($op_parser.then($operand).repeated())
                    .foldl(|lhs, (op, rhs)| {
                        let sp = expr_span(&lhs).merge(expr_span(&rhs));
                        Expr::BinOp {
                            op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span: sp,
                        }
                    })
            };
        }

        let mul_op = just(Token::Star)
            .to(BinOp::Mul)
            .or(just(Token::Slash).to(BinOp::Div))
            .or(just(Token::Percent).to(BinOp::Mod));

        let add_op = just(Token::Plus)
            .to(BinOp::Add)
            .or(just(Token::Minus).to(BinOp::Sub));

        let cmp_op = just(Token::LtEq)
            .to(BinOp::LtEq)
            .or(just(Token::GtEq).to(BinOp::GtEq))
            .or(just(Token::Lt).to(BinOp::Lt))
            .or(just(Token::Gt).to(BinOp::Gt));

        let eq_op = just(Token::EqEq)
            .to(BinOp::Eq)
            .or(just(Token::BangEq).to(BinOp::NotEq));

        let mul = binop_level!(unary, mul_op);
        let add = binop_level!(mul, add_op);
        let cmp = binop_level!(add, cmp_op);
        let eq = binop_level!(cmp, eq_op);
        let and = binop_level!(eq, just(Token::AmpAmp).to(BinOp::And));
        binop_level!(and, just(Token::PipePipe).to(BinOp::Or))
    })
}

/// Internal enum used during postfix parsing.
enum PostfixOp {
    Call(Vec<Expr>, Span),
    Field(String, Span),
    Index(Box<Expr>, Span),
}

// ── Statement parser ──────────────────────────────────────────────────────────

pub fn stmt_parser() -> impl Parser<Token, Stmt, Error = ParseErr> + Clone {
    recursive(|stmt| {
        let expr = expr_parser();
        let block = block_parser(stmt.clone());

        // let name [: type] = expr
        let let_stmt = just(Token::Let)
            .ignore_then(ident_tok())
            .then(just(Token::Colon).ignore_then(type_expr()).or_not())
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .map_with_span(|((name, ty), value), sp| Stmt::Let {
                name,
                ty,
                value,
                span: s(sp),
            });

        // var name [: type] = expr
        let var_stmt = just(Token::Var)
            .ignore_then(ident_tok())
            .then(just(Token::Colon).ignore_then(type_expr()).or_not())
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .map_with_span(|((name, ty), value), sp| Stmt::Var {
                name,
                ty,
                value,
                span: s(sp),
            });

        // return [expr]
        let return_stmt = just(Token::Return)
            .ignore_then(expr.clone().or_not())
            .map_with_span(|value, sp| Stmt::Return { value, span: s(sp) });

        // break / continue / pass
        let break_stmt = just(Token::Break).map_with_span(|_, sp| Stmt::Break { span: s(sp) });
        let continue_stmt =
            just(Token::Continue).map_with_span(|_, sp| Stmt::Continue { span: s(sp) });
        let pass_stmt = just(Token::Pass).map_with_span(|_, sp| Stmt::Pass { span: s(sp) });

        // if expr: block [elif expr: block]* [else: block]
        //
        // nl() before elif/else because the block emits a BlockEnd, then the
        // preprocessor emits a Newline (same-level indent), and THEN elif/else
        // appears.
        let elif_branch = nl()
            .ignore_then(just(Token::Elif))
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Colon))
            .then(block.clone());

        let else_branch = nl()
            .ignore_then(just(Token::Else))
            .ignore_then(just(Token::Colon))
            .ignore_then(block.clone());

        let if_stmt = just(Token::If)
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Colon))
            .then(block.clone())
            .then(elif_branch.repeated())
            .then(else_branch.or_not())
            .map_with_span(
                |(((condition, then_body), elif_branches), else_body), sp| Stmt::If {
                    condition,
                    then_body,
                    elif_branches,
                    else_body,
                    span: s(sp),
                },
            );

        // while expr: block
        let while_stmt = just(Token::While)
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Colon))
            .then(block.clone())
            .map_with_span(|(condition, body), sp| Stmt::While {
                condition,
                body,
                span: s(sp),
            });

        // for name in expr: block
        let for_stmt = just(Token::For)
            .ignore_then(ident_tok())
            .then_ignore(just(Token::In))
            .then(expr.clone())
            .then_ignore(just(Token::Colon))
            .then(block.clone())
            .map_with_span(|((var, iter), body), sp| Stmt::For {
                var,
                iter,
                body,
                span: s(sp),
            });

        // ── Assignment and expression statements ──────────────────────────────
        //
        // IMPORTANT: We cannot have separate `assign_stmt` and `expr_stmt`
        // because chumsky 0.9 does not backtrack after consuming tokens.
        // If `assign_stmt` parses the LHS expression and then fails to find
        // an `=`, it has already consumed tokens and `expr_stmt` never runs.
        //
        // Solution: parse the expression first, then OPTIONALLY extend it
        // into an assignment.  This way the expression is parsed exactly once.
        let assign_op = choice((
            just(Token::Eq).to(None::<BinOp>),
            just(Token::PlusEq).to(Some(BinOp::Add)),
            just(Token::MinusEq).to(Some(BinOp::Sub)),
            just(Token::StarEq).to(Some(BinOp::Mul)),
            just(Token::SlashEq).to(Some(BinOp::Div)),
        ));

        let expr_or_assign = expr
            .clone()
            .then(assign_op.then(expr.clone()).or_not())
            .map_with_span(|(target, rhs_opt), sp| match rhs_opt {
                // Plain expression statement
                None => Stmt::Expr(target),
                // Assignment: target = rhs  OR  target += rhs (desugared)
                Some((op, rhs)) => {
                    let value = match op {
                        None => rhs,
                        Some(bin_op) => {
                            let tsp = expr_span(&target).merge(expr_span(&rhs));
                            Expr::BinOp {
                                op: bin_op,
                                lhs: Box::new(target.clone()),
                                rhs: Box::new(rhs),
                                span: tsp,
                            }
                        }
                    };
                    Stmt::Assign {
                        target,
                        value,
                        span: s(sp),
                    }
                }
            });

        // All statements, in priority order (keyword-led parsers first).
        let stmt_inner = choice((
            let_stmt,
            var_stmt,
            return_stmt,
            break_stmt,
            continue_stmt,
            pass_stmt,
            if_stmt,
            while_stmt,
            for_stmt,
            expr_or_assign, // handles both expr statements AND assignments
        ));

        // Skip leading/trailing newlines around each statement.
        nl().ignore_then(stmt_inner).then_ignore(nl())
    })
}

/// A block: `BlockStart  stmt*  BlockEnd`
///
/// Statements are separated by `Newline` tokens; leading and trailing
/// newlines inside the block are allowed.
fn block_parser(
    stmt: impl Parser<Token, Stmt, Error = ParseErr> + Clone,
) -> impl Parser<Token, Vec<Stmt>, Error = ParseErr> + Clone {
    stmt.separated_by(just(Token::Newline))
        .allow_leading()
        .allow_trailing()
        .delimited_by(just(Token::BlockStart), just(Token::BlockEnd))
}

// ── Top-level item parsers ────────────────────────────────────────────────────

/// Parse a function parameter: `name [: type]`
fn param_parser() -> impl Parser<Token, Param, Error = ParseErr> + Clone {
    ident_tok()
        .then(just(Token::Colon).ignore_then(type_expr()).or_not())
        .map_with_span(|(name, ty), sp| Param {
            name,
            ty,
            span: s(sp),
        })
}

/// Parse the `@runtime: BlockStart key: value* BlockEnd` config block.
fn runtime_config_parser() -> impl Parser<Token, RuntimeConfig, Error = ParseErr> + Clone {
    let entry = ident_tok()
        .then_ignore(just(Token::Colon))
        .then(ident_tok())
        .then_ignore(just(Token::Newline).or_not());

    just(Token::Decorator("runtime".to_string()))
        .ignore_then(just(Token::Colon))
        .ignore_then(just(Token::BlockStart))
        .ignore_then(entry.repeated())
        .then_ignore(just(Token::BlockEnd))
        .map_with_span(|entries, sp| RuntimeConfig {
            entries,
            span: s(sp),
        })
}

/// Parse a `fn` declaration (with optional leading decorators).
fn function_parser() -> impl Parser<Token, Function, Error = ParseErr> + Clone {
    let stmt = stmt_parser();
    let block = block_parser(stmt);

    let params = param_parser()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .delimited_by(just(Token::LParen), just(Token::RParen));

    decorators()
        .then_ignore(just(Token::Fn))
        .then(ident_tok())
        .then(params)
        .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
        .then_ignore(just(Token::Colon))
        .then(block)
        .map_with_span(
            |((((decorators, name), params), return_ty), body), sp| Function {
                decorators,
                name,
                params,
                return_ty,
                body,
                span: s(sp),
            },
        )
}

/// Parse a `module` declaration (with optional leading decorators).
///
/// Module bodies can contain the same items as the top level.
fn module_parser() -> impl Parser<Token, ModuleDecl, Error = ParseErr> + Clone {
    let item = item_parser();

    let body = item
        .separated_by(just(Token::Newline))
        .allow_leading()
        .allow_trailing()
        .delimited_by(just(Token::BlockStart), just(Token::BlockEnd));

    decorators()
        .then_ignore(just(Token::Module))
        .then(ident_tok())
        .then_ignore(just(Token::Colon))
        .then(body)
        .map_with_span(|((decorators, name), items), sp| ModuleDecl {
            decorators,
            name,
            items,
            span: s(sp),
        })
}

/// Parse a `struct` declaration.
fn struct_parser() -> impl Parser<Token, StructDecl, Error = ParseErr> + Clone {
    let field = ident_tok()
        .then_ignore(just(Token::Colon))
        .then(type_expr())
        .then_ignore(just(Token::Newline).or_not())
        .map_with_span(|(name, ty), sp| StructField {
            name,
            ty,
            span: s(sp),
        });

    let fields = field
        .repeated()
        .delimited_by(just(Token::BlockStart), just(Token::BlockEnd));

    decorators()
        .then_ignore(just(Token::Struct))
        .then(ident_tok())
        .then_ignore(just(Token::Colon))
        .then(fields)
        .map_with_span(|((decorators, name), fields), sp| StructDecl {
            decorators,
            name,
            fields,
            span: s(sp),
        })
}

/// Parse an `import path [as alias]` statement.
fn import_parser() -> impl Parser<Token, ImportDecl, Error = ParseErr> + Clone {
    let path = ident_tok().separated_by(just(Token::Dot)).at_least(1);

    just(Token::Import)
        .ignore_then(path)
        .then(just(Token::As).ignore_then(ident_tok()).or_not())
        .map_with_span(|(path, alias), sp| ImportDecl {
            path,
            alias,
            span: s(sp),
        })
}

// ── Item parser (used at program level AND inside modules) ────────────────────

fn item_parser() -> impl Parser<Token, Item, Error = ParseErr> + Clone {
    // `module` is mutually recursive with `item` (module bodies contain items),
    // so we use chumsky's `recursive()` to break the cycle instead of two
    // separate `impl Trait` functions calling each other.
    recursive(|item| {
        let module_body = item
            .clone()
            .separated_by(just(Token::Newline))
            .allow_leading()
            .allow_trailing()
            .delimited_by(just(Token::BlockStart), just(Token::BlockEnd));

        let module = decorators()
            .then_ignore(just(Token::Module))
            .then(ident_tok())
            .then_ignore(just(Token::Colon))
            .then(module_body)
            .map_with_span(|((decorators, name), items), sp| ModuleDecl {
                decorators,
                name,
                items,
                span: s(sp),
            });

        choice((
            runtime_config_parser().map(Item::RuntimeConfig),
            function_parser().map(Item::Function),
            module.map(Item::Module),
            struct_parser().map(Item::Struct),
            import_parser().map(Item::Import),
            expr_parser().map(Item::Expr),
        ))
    })
}

// ── Top-level program parser ──────────────────────────────────────────────────

pub fn program_parser() -> impl Parser<Token, Vec<Item>, Error = ParseErr> {
    nl().ignore_then(item_parser())
        .then_ignore(nl())
        .repeated()
        .then_ignore(end())
}
