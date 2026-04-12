use crate::ast::{self, Program, Statement, Expression, Parameter, TypeExpr, Decorator, Block, BinaryOperator};
use crate::lexer::{Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected token: expected {expected}, got {got} at line {line}")]
    UnexpectedToken {
        expected: String,
        got: String,
        line: usize,
    },

    #[error("unexpected end of file")]
    UnexpectedEof,

    #[error("invalid syntax at line {line}: {message}")]
    InvalidSyntax { line: usize, message: String },
}

/// The Nova parser — transforms a token stream into an AST
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

/// Parse a token stream into a Nova AST
pub fn parse(tokens: Vec<Token>) -> Result<Program, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── Helpers ──────────────────────────────────────────────

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn current_kind(&self) -> TokenKind {
        self.current().map(|t| t.kind).unwrap_or(TokenKind::Eof)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, Box<dyn std::error::Error>> {
        if self.current_kind() == kind {
            Ok(self.advance().unwrap())
        } else {
            Err(Box::new(ParseError::UnexpectedToken {
                expected: format!("{}", kind),
                got: format!("{}", self.current_kind()),
                line: self.current().map(|t| t.span.0).unwrap_or(0),
            }))
        }
    }

    fn skip_newlines(&mut self) {
        while self.current_kind() == TokenKind::Newline
            || self.current_kind() == TokenKind::Comment
        {
            self.advance();
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    // ── Top-level parsing ────────────────────────────────────

    pub fn parse_program(&mut self) -> Result<Program, Box<dyn std::error::Error>> {
        let mut statements = Vec::new();

        self.skip_newlines();
        while !self.at(TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            self.skip_newlines();
        }

        Ok(Program { statements })
    }

    // ── Statement parsing ────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.skip_newlines();

        match self.current_kind() {
            TokenKind::At => self.parse_decorated(),
            TokenKind::Fn => self.parse_function_def(Vec::new(), false),
            TokenKind::Pub => self.parse_pub_item(),
            TokenKind::Let => self.parse_let_binding(),
            TokenKind::Const => self.parse_const_binding(),
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::While => self.parse_while(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Struct => self.parse_struct(false),
            TokenKind::Enum => self.parse_enum(false),
            TokenKind::Trait => self.parse_trait(false),
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Match => self.parse_match(),
            TokenKind::Break => {
                self.advance();
                Ok(Statement::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Statement::Continue)
            }
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_decorated(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        let mut decorators = Vec::new();

        while self.at(TokenKind::At) {
            self.advance(); // skip @
            let name = self.expect(TokenKind::Identifier)?.text.clone();

            let mut args = Vec::new();
            if self.at(TokenKind::LParen) {
                self.advance();
                while !self.at(TokenKind::RParen) {
                    args.push(self.parse_expression()?);
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen)?;
            }

            decorators.push(Decorator { name, args });
            self.skip_newlines();
        }

        match self.current_kind() {
            TokenKind::Fn => self.parse_function_def(decorators, false),
            TokenKind::Struct => self.parse_struct(false), // TODO: pass decorators
            _ => Err(Box::new(ParseError::InvalidSyntax {
                line: self.current().map(|t| t.span.0).unwrap_or(0),
                message: "decorator must be followed by fn or struct".to_string(),
            })),
        }
    }

    fn parse_pub_item(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.advance(); // skip `pub`
        match self.current_kind() {
            TokenKind::Fn => self.parse_function_def(Vec::new(), true),
            TokenKind::Struct => self.parse_struct(true),
            TokenKind::Enum => self.parse_enum(true),
            TokenKind::Trait => self.parse_trait(true),
            _ => Err(Box::new(ParseError::InvalidSyntax {
                line: self.current().map(|t| t.span.0).unwrap_or(0),
                message: "pub must be followed by fn, struct, enum, or trait".to_string(),
            })),
        }
    }

    fn parse_function_def(
        &mut self,
        decorators: Vec<Decorator>,
        is_pub: bool,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Fn)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();

        // Parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) {
            let pname = self.expect(TokenKind::Identifier)?.text.clone();
            self.expect(TokenKind::Colon)?;
            let ptype = self.parse_type()?;
            params.push(Parameter {
                name: pname,
                type_annotation: ptype,
                default: None,
            });
            if self.at(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen)?;

        // Return type
        let return_type = if self.at(TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Effect annotations [io, error]
        let mut effects = Vec::new();
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at(TokenKind::RBracket) {
                effects.push(self.expect(TokenKind::Identifier)?.text.clone());
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
        }

        // Body — either `:` + indented block or `{` inline `}`
        let body = if self.at(TokenKind::Colon) {
            self.advance();
            self.parse_indented_block()?
        } else if self.at(TokenKind::LBrace) {
            self.parse_brace_block()?
        } else {
            return Err(Box::new(ParseError::InvalidSyntax {
                line: self.current().map(|t| t.span.0).unwrap_or(0),
                message: "expected ':' or '{' after function signature".to_string(),
            }));
        };

        Ok(Statement::FunctionDef {
            name,
            params,
            return_type,
            effects,
            body,
            decorators,
            is_pub,
        })
    }

    fn parse_let_binding(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Let)?;

        let mutable = if self.at(TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect(TokenKind::Identifier)?.text.clone();

        let type_annotation = if self.at(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Assign)?;
        let value = self.parse_expression()?;

        Ok(Statement::LetBinding {
            name,
            type_annotation,
            value,
            mutable,
        })
    }

    fn parse_const_binding(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Const)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();

        let type_annotation = if self.at(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Assign)?;
        let value = self.parse_expression()?;

        Ok(Statement::ConstBinding {
            name,
            type_annotation,
            value,
        })
    }

    fn parse_if(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        let mut elif_clauses = Vec::new();
        let mut else_body = None;

        self.skip_newlines();
        while self.at(TokenKind::Elif) {
            self.advance();
            let elif_cond = self.parse_expression()?;
            self.expect(TokenKind::Colon)?;
            let elif_body = self.parse_indented_block()?;
            elif_clauses.push((elif_cond, elif_body));
            self.skip_newlines();
        }

        if self.at(TokenKind::Else) {
            self.advance();
            self.expect(TokenKind::Colon)?;
            else_body = Some(self.parse_indented_block()?);
        }

        Ok(Statement::If {
            condition,
            body,
            elif_clauses,
            else_body,
        })
    }

    fn parse_for(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::For)?;
        let variable = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expression()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        Ok(Statement::ForLoop {
            variable,
            iterable,
            body,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        Ok(Statement::WhileLoop { condition, body })
    }

    fn parse_return(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.advance(); // skip `return`
        if self.at(TokenKind::Newline) || self.at(TokenKind::Eof) || self.at(TokenKind::Dedent) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expression()?)))
        }
    }

    fn parse_struct(&mut self, is_pub: bool) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Struct)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        let fields = body
            .into_iter()
            .filter_map(|s| {
                if let Statement::LetBinding {
                    name, type_annotation, value, ..
                } = s
                {
                    Some(ast::Field {
                        name,
                        type_annotation: type_annotation.unwrap_or(TypeExpr::Named("any".into())),
                        default: Some(value),
                        is_pub: true,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(Statement::StructDef {
            name,
            fields,
            is_pub,
        })
    }

    fn parse_enum(&mut self, is_pub: bool) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Enum)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::Colon)?;

        // Simplified: just parse the block
        let _body = self.parse_indented_block()?;

        Ok(Statement::EnumDef {
            name,
            variants: Vec::new(), // TODO: proper variant parsing
            is_pub,
        })
    }

    fn parse_trait(&mut self, is_pub: bool) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Trait)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::Colon)?;
        let _body = self.parse_indented_block()?;

        Ok(Statement::TraitDef {
            name,
            methods: Vec::new(), // TODO: proper method parsing
            is_pub,
        })
    }

    fn parse_impl(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Impl)?;
        let first = self.expect(TokenKind::Identifier)?.text.clone();

        let (trait_name, target_type) = if self.at(TokenKind::For) {
            self.advance();
            let target = self.expect(TokenKind::Identifier)?.text.clone();
            (Some(first), target)
        } else {
            (None, first)
        };

        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        Ok(Statement::ImplBlock {
            trait_name,
            target_type,
            methods: body,
        })
    }

    fn parse_import(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Import)?;

        // Check for foreign import
        if self.at(TokenKind::Foreign) {
            self.advance();
            self.expect(TokenKind::LParen)?;
            let path = self.expect(TokenKind::StringLiteral)?.text.clone();
            let path = path.trim_matches('"').to_string();
            // TODO: parse lang parameter
            self.expect(TokenKind::RParen)?;
            return Ok(Statement::ForeignImport {
                path,
                lang: "c".to_string(),
                items: None,
            });
        }

        let mut path = vec![self.expect(TokenKind::Identifier)?.text.clone()];
        while self.at(TokenKind::Dot) {
            self.advance();
            path.push(self.expect(TokenKind::Identifier)?.text.clone());
        }

        Ok(Statement::Import { path, items: None })
    }

    fn parse_match(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Match)?;
        let subject = self.parse_expression()?;
        self.expect(TokenKind::Colon)?;
        let _body = self.parse_indented_block()?;

        Ok(Statement::Match {
            subject,
            arms: Vec::new(), // TODO: proper match arm parsing
        })
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        let expr = self.parse_expression()?;

        // Check for assignment
        if self.at(TokenKind::Assign) {
            self.advance();
            let value = self.parse_expression()?;
            return Ok(Statement::Assignment {
                target: expr,
                value,
            });
        }

        Ok(Statement::Expression(expr))
    }

    // ── Block parsing ────────────────────────────────────────

    fn parse_indented_block(&mut self) -> Result<Block, Box<dyn std::error::Error>> {
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;

        let mut stmts = Vec::new();
        self.skip_newlines();

        while !self.at(TokenKind::Dedent) && !self.at(TokenKind::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }

        if self.at(TokenKind::Dedent) {
            self.advance();
        }

        Ok(stmts)
    }

    fn parse_brace_block(&mut self) -> Result<Block, Box<dyn std::error::Error>> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();

        self.skip_newlines();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            stmts.push(self.parse_statement()?);
            // In brace mode, semicolons separate statements
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(TokenKind::RBrace)?;
        Ok(stmts)
    }

    // ── Expression parsing (precedence climbing) ─────────────

    fn parse_expression(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_or()?;

        while self.at(TokenKind::Pipe) {
            self.advance();
            let right = self.parse_or()?;
            left = Expression::Pipe {
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_and()?;

        while self.at(TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_comparison()?;

        while self.at(TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Eq => BinaryOperator::Eq,
                TokenKind::NotEq => BinaryOperator::NotEq,
                TokenKind::Lt => BinaryOperator::Lt,
                TokenKind::Gt => BinaryOperator::Gt,
                TokenKind::LtEq => BinaryOperator::LtEq,
                TokenKind::GtEq => BinaryOperator::GtEq,
                TokenKind::In => BinaryOperator::In,
                TokenKind::Is => BinaryOperator::Is,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Star => BinaryOperator::Mul,
                TokenKind::Slash => BinaryOperator::Div,
                TokenKind::DoubleSlash => BinaryOperator::IntDiv,
                TokenKind::Percent => BinaryOperator::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        match self.current_kind() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: ast::UnaryOperator::Neg,
                    operand: Box::new(operand),
                })
            }
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: ast::UnaryOperator::Not,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current_kind() {
                TokenKind::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.at(TokenKind::RParen) {
                        args.push(self.parse_expression()?);
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    expr = Expression::Call {
                        function: Box::new(expr),
                        args,
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect(TokenKind::Identifier)?.text.clone();

                    if self.at(TokenKind::LParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.at(TokenKind::RParen) {
                            args.push(self.parse_expression()?);
                            if self.at(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                        expr = Expression::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                        };
                    } else {
                        expr = Expression::FieldAccess {
                            object: Box::new(expr),
                            field,
                        };
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expression::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        match self.current_kind() {
            TokenKind::IntLiteral => {
                let text = self.advance().unwrap().text.clone();
                let val: i64 = text.replace('_', "").parse()?;
                Ok(Expression::IntLiteral(val))
            }
            TokenKind::FloatLiteral => {
                let text = self.advance().unwrap().text.clone();
                let val: f64 = text.replace('_', "").parse()?;
                Ok(Expression::FloatLiteral(val))
            }
            TokenKind::StringLiteral => {
                let text = self.advance().unwrap().text.clone();
                Ok(Expression::StringLiteral(
                    text.trim_matches('"').to_string(),
                ))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::BoolLiteral(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::BoolLiteral(false))
            }
            TokenKind::None => {
                self.advance();
                Ok(Expression::NoneLiteral)
            }
            TokenKind::Identifier => {
                let name = self.advance().unwrap().text.clone();

                // Check for lambda: `x => expr`
                if self.at(TokenKind::FatArrow) {
                    self.advance();
                    let body = self.parse_expression()?;
                    return Ok(Expression::Lambda {
                        params: vec![name],
                        body: Box::new(body),
                    });
                }

                Ok(Expression::Identifier(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !self.at(TokenKind::RBracket) {
                    elements.push(self.parse_expression()?);
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expression::List(elements))
            }
            TokenKind::Await => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expression::Await(Box::new(expr)))
            }
            _ => Err(Box::new(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                got: format!("{}", self.current_kind()),
                line: self.current().map(|t| t.span.0).unwrap_or(0),
            })),
        }
    }

    // ── Type parsing ─────────────────────────────────────────

    fn parse_type(&mut self) -> Result<TypeExpr, Box<dyn std::error::Error>> {
        let name = self.expect(TokenKind::Identifier)?.text.clone();

        // Generic types: list[int], dict[str, int]
        if self.at(TokenKind::LBracket) {
            self.advance();
            let mut type_args = vec![self.parse_type()?];
            while self.at(TokenKind::Comma) {
                self.advance();
                type_args.push(self.parse_type()?);
            }
            self.expect(TokenKind::RBracket)?;
            return Ok(TypeExpr::Generic(name, type_args));
        }

        Ok(TypeExpr::Named(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    #[test]
    fn test_parse_simple_function() {
        let source = "fn greet(name: str) -> str:\n    return name";
        let tokens = lexer::tokenize(source).unwrap();
        let program = parse(tokens).unwrap();
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(
            &program.statements[0],
            Statement::FunctionDef { name, .. } if name == "greet"
        ));
    }

    #[test]
    fn test_parse_let_binding() {
        let source = "let x: int = 42";
        let tokens = lexer::tokenize(source).unwrap();
        let program = parse(tokens).unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::LetBinding { name, mutable: false, .. } if name == "x"
        ));
    }

    #[test]
    fn test_parse_pipe() {
        let source = "data |> transform |> output";
        let tokens = lexer::tokenize(source).unwrap();
        let program = parse(tokens).unwrap();
        assert!(matches!(
            &program.statements[0],
            Statement::Expression(Expression::Pipe { .. })
        ));
    }
}
