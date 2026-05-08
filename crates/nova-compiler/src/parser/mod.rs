use crate::ast::{
    self, BinaryOperator, Block, Decorator, Expression, Parameter, Program, Statement, TypeExpr,
};
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
            || self.current_kind() == TokenKind::DocComment
        {
            self.advance();
        }
    }

    /// Skip only whitespace/comments but keep DocComments for collection.
    fn skip_non_doc_noise(&mut self) {
        while self.current_kind() == TokenKind::Newline || self.current_kind() == TokenKind::Comment
        {
            self.advance();
        }
    }

    /// Collect consecutive doc comments. Stops at any non-DocComment/Newline token.
    /// A blank line between doc and declaration means the doc is file-level and gets dropped.
    fn collect_doc_comments(&mut self) -> Option<String> {
        let mut doc_lines: Vec<String> = Vec::new();

        loop {
            match self.current_kind() {
                TokenKind::DocComment => {
                    if let Some(tok) = self.current() {
                        doc_lines.push(tok.text.clone());
                    }
                    self.advance();
                }
                TokenKind::Newline => {
                    // Peek ahead: if next non-newline is another DocComment, keep collecting
                    // If it's anything else (declaration, blank), stop
                    self.advance();
                    match self.peek_past_newlines() {
                        Some(TokenKind::DocComment) => {
                            // Check if there was a blank line (2+ consecutive newlines before next doc)
                            let mut blank = false;
                            let mut i = self.pos;
                            let mut nl_count = 0;
                            while i < self.tokens.len() {
                                match self.tokens[i].kind {
                                    TokenKind::Newline => {
                                        nl_count += 1;
                                        if nl_count >= 1 {
                                            blank = true;
                                        }
                                    }
                                    TokenKind::Comment => {}
                                    _ => break,
                                }
                                i += 1;
                            }
                            if blank {
                                // Blank line separates doc blocks — drop current and start fresh
                                doc_lines.clear();
                                self.skip_non_doc_noise();
                            }
                            // Continue loop to collect next doc comment
                        }
                        _ => break,
                    }
                }
                TokenKind::Comment => {
                    self.advance();
                }
                _ => break,
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    /// Peek past newlines/comments to check if a continuation operator follows.
    /// Returns the index of the continuation operator, or None.
    fn peek_past_newlines(&self) -> Option<TokenKind> {
        let mut i = self.pos;
        while i < self.tokens.len() {
            let kind = self.tokens[i].kind;
            if kind == TokenKind::Newline || kind == TokenKind::Comment {
                i += 1;
                continue;
            }
            return Some(kind);
        }
        None
    }

    /// Skip newlines if the next non-newline token is a continuation operator.
    fn skip_newlines_if_continuation(&mut self) {
        let next = self.peek_past_newlines();
        if matches!(
            next,
            Some(TokenKind::Pipe)
                | Some(TokenKind::Plus)
                | Some(TokenKind::Minus)
                | Some(TokenKind::Star)
                | Some(TokenKind::Slash)
                | Some(TokenKind::And)
                | Some(TokenKind::Or)
                | Some(TokenKind::Dot)
        ) {
            while self.current_kind() == TokenKind::Newline
                || self.current_kind() == TokenKind::Comment
            {
                self.advance();
            }
        }
    }

    // ── Top-level parsing ────────────────────────────────────

    pub fn parse_program(&mut self) -> Result<Program, Box<dyn std::error::Error>> {
        let mut statements = Vec::new();

        self.skip_non_doc_noise();
        while !self.at(TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            self.skip_non_doc_noise();
        }

        Ok(Program { statements })
    }

    // ── Statement parsing ────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Statement, Box<dyn std::error::Error>> {
        self.skip_non_doc_noise();

        // Doc comments precede declarations
        let doc = if self.at(TokenKind::DocComment) {
            self.collect_doc_comments()
        } else {
            None
        };

        match self.current_kind() {
            TokenKind::At => self.parse_decorated(doc),
            TokenKind::Fn => self.parse_function_def(Vec::new(), false, false, doc),
            TokenKind::Pure => {
                self.advance(); // skip `pure`
                self.parse_function_def(Vec::new(), false, true, doc)
            }
            TokenKind::Pub => self.parse_pub_item(doc),
            TokenKind::Let => self.parse_let_binding(false),
            TokenKind::Const => self.parse_const_binding(),
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::While => self.parse_while(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Require => {
                self.advance();
                Ok(Statement::Require(self.parse_expression()?))
            }
            TokenKind::Ensure => {
                self.advance();
                Ok(Statement::Ensure(self.parse_expression()?))
            }
            TokenKind::Struct => self.parse_struct(false, doc),
            TokenKind::Enum => self.parse_enum(false, doc),
            TokenKind::Trait => self.parse_trait(false, doc),
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

    fn parse_decorated(
        &mut self,
        doc: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
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
            TokenKind::Fn => self.parse_function_def(decorators, false, false, doc),
            TokenKind::Pure => {
                self.advance();
                self.parse_function_def(decorators, false, true, doc)
            }
            TokenKind::Struct => self.parse_struct(false, doc),
            TokenKind::Pub => {
                // pub after decorators
                self.advance();
                match self.current_kind() {
                    TokenKind::Fn => self.parse_function_def(decorators, true, false, doc),
                    TokenKind::Pure => {
                        self.advance();
                        self.parse_function_def(decorators, true, true, doc)
                    }
                    TokenKind::Struct => self.parse_struct(true, doc),
                    _ => Err(Box::new(ParseError::InvalidSyntax {
                        line: self.current().map(|t| t.span.0).unwrap_or(0),
                        message: "decorator + pub must be followed by fn or struct".to_string(),
                    })),
                }
            }
            _ => Err(Box::new(ParseError::InvalidSyntax {
                line: self.current().map(|t| t.span.0).unwrap_or(0),
                message: "decorator must be followed by fn or struct".to_string(),
            })),
        }
    }

    fn parse_pub_item(
        &mut self,
        doc: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.advance(); // skip `pub`
        match self.current_kind() {
            TokenKind::Fn => self.parse_function_def(Vec::new(), true, false, doc),
            TokenKind::Pure => {
                self.advance();
                self.parse_function_def(Vec::new(), true, true, doc)
            }
            TokenKind::Struct => self.parse_struct(true, doc),
            TokenKind::Enum => self.parse_enum(true, doc),
            TokenKind::Trait => self.parse_trait(true, doc),
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
        _is_pure: bool,
        doc_comment: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Fn)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();

        // Parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) {
            // Allow `self` keyword as parameter name
            let pname = if self.at(TokenKind::SelfKw) {
                self.advance();
                "self".to_string()
            } else {
                self.expect(TokenKind::Identifier)?.text.clone()
            };
            // `self` alone has no type annotation
            let ptype = if self.at(TokenKind::Colon) {
                self.advance();
                self.parse_type()?
            } else {
                TypeExpr::Named("Self".to_string())
            };
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

        // Body — either `:` + indented block, `{` inline `}`, or absent (trait signature)
        let body = if self.at(TokenKind::Colon) {
            self.advance();
            self.parse_indented_block()?
        } else if self.at(TokenKind::LBrace) {
            self.parse_brace_block()?
        } else if matches!(
            self.current_kind(),
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
        ) {
            // Trait method signature — no body
            Vec::new()
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
            doc_comment,
        })
    }

    fn parse_let_binding(
        &mut self,
        require_value: bool,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
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

        // Value is optional in struct field declarations
        let value = if self.at(TokenKind::Assign) {
            self.advance();
            self.parse_expression()?
        } else if !require_value {
            Expression::NoneLiteral
        } else {
            return Err(Box::new(ParseError::InvalidSyntax {
                line: self.current().map(|t| t.span.0).unwrap_or(0),
                message: "expected '=' in let binding".to_string(),
            }));
        };

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

    fn parse_struct(
        &mut self,
        is_pub: bool,
        doc_comment: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Struct)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::Colon)?;
        let body = self.parse_indented_block()?;

        let fields = body
            .into_iter()
            .filter_map(|s| {
                if let Statement::LetBinding {
                    name,
                    type_annotation,
                    value,
                    ..
                } = s
                {
                    // NoneLiteral signals "no default" for struct fields without `= value`
                    let default = match value {
                        Expression::NoneLiteral => None,
                        other => Some(other),
                    };
                    Some(ast::Field {
                        name,
                        type_annotation: type_annotation.unwrap_or(TypeExpr::Named("any".into())),
                        default,
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
            doc_comment,
        })
    }

    fn parse_enum(
        &mut self,
        is_pub: bool,
        doc_comment: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Enum)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();

        // Optional generic type parameters: enum Result[T, E]:
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at(TokenKind::RBracket) {
                self.expect(TokenKind::Identifier)?;
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
        }

        self.expect(TokenKind::Colon)?;
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;
        self.skip_newlines();

        let mut variants = Vec::new();
        while !self.at(TokenKind::Dedent) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Case) {
                self.advance();
                let vname = self.expect(TokenKind::Identifier)?.text.clone();
                // Optional payload: case Some(T) or case Ok(value: T)
                let fields = if self.at(TokenKind::LParen) {
                    self.advance();
                    let mut types = Vec::new();
                    while !self.at(TokenKind::RParen) {
                        // Allow `name: Type` or just `Type`
                        if self.at(TokenKind::Identifier) {
                            let saved_pos = self.pos;
                            let _ = self.advance(); // consume identifier
                            if self.at(TokenKind::Colon) {
                                self.advance(); // consume colon, parse type
                                types.push(self.parse_type()?);
                            } else {
                                // It was a type name, not a field name
                                self.pos = saved_pos;
                                types.push(self.parse_type()?);
                            }
                        } else {
                            types.push(self.parse_type()?);
                        }
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Some(types)
                } else {
                    None
                };
                variants.push(ast::EnumVariant {
                    name: vname,
                    fields,
                });
            }
            self.skip_newlines();
        }
        if self.at(TokenKind::Dedent) {
            self.advance();
        }

        Ok(Statement::EnumDef {
            name,
            variants,
            is_pub,
            doc_comment,
        })
    }

    fn parse_trait(
        &mut self,
        is_pub: bool,
        doc_comment: Option<String>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        self.expect(TokenKind::Trait)?;
        let name = self.expect(TokenKind::Identifier)?.text.clone();
        self.expect(TokenKind::Colon)?;
        // Parse the trait body — collect method signatures
        // We reuse parse_indented_block which handles fn defs fine
        let _body = self.parse_indented_block()?;

        Ok(Statement::TraitDef {
            name,
            methods: Vec::new(),
            is_pub,
            doc_comment,
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

        // Check for foreign import: import foreign("header.h", lang: "c", items: [...])
        if self.at(TokenKind::Foreign) {
            self.advance();
            self.expect(TokenKind::LParen)?;
            let path = self.expect(TokenKind::StringLiteral)?.text.clone();
            let path = path.trim_matches('"').to_string();

            let mut lang = "c".to_string();
            let mut items: Option<Vec<String>> = None;

            // Parse optional named arguments
            while self.at(TokenKind::Comma) {
                self.advance();
                let key = self.expect(TokenKind::Identifier)?.text.clone();
                self.expect(TokenKind::Colon)?;
                match key.as_str() {
                    "lang" => {
                        lang = self.expect(TokenKind::StringLiteral)?.text.clone();
                        lang = lang.trim_matches('"').to_string();
                    }
                    "items" => {
                        self.expect(TokenKind::LBracket)?;
                        let mut item_list = Vec::new();
                        while !self.at(TokenKind::RBracket) {
                            item_list.push(
                                self.expect(TokenKind::StringLiteral)?
                                    .text
                                    .trim_matches('"')
                                    .to_string(),
                            );
                            if self.at(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RBracket)?;
                        items = Some(item_list);
                    }
                    _ => {
                        self.parse_expression()?;
                    }
                }
            }

            self.expect(TokenKind::RParen)?;
            return Ok(Statement::ForeignImport { path, lang, items });
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
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.at(TokenKind::Dedent) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Case) {
                self.advance();
                // Parse pattern: wildcard `_`, or expression (identifier, dotted, etc.)
                let pattern = if self.at(TokenKind::Identifier)
                    && self.current().map(|t| t.text.as_str()) == Some("_")
                {
                    self.advance();
                    ast::Pattern::Wildcard
                } else {
                    // Parse as expression and convert to pattern
                    let expr = self.parse_expression()?;
                    ast::Pattern::Literal(expr)
                };
                self.expect(TokenKind::Colon)?;
                let body = self.parse_indented_block()?;
                arms.push(ast::MatchArm {
                    pattern,
                    guard: None,
                    body,
                });
            }
            self.skip_newlines();
        }
        if self.at(TokenKind::Dedent) {
            self.advance();
        }

        Ok(Statement::Match { subject, arms })
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
        self.skip_non_doc_noise();
        self.expect(TokenKind::Indent)?;

        let mut stmts = Vec::new();
        // Inside blocks, also skip doc comments - they only attach to top-level declarations
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

        self.skip_non_doc_noise();
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

        // Allow pipe continuation across newlines
        self.skip_newlines_if_continuation();

        while self.at(TokenKind::Pipe) {
            self.advance();
            // Skip any newlines/comments after the pipe operator
            while self.current_kind() == TokenKind::Newline
                || self.current_kind() == TokenKind::Comment
            {
                self.advance();
            }
            let right = self.parse_or()?;
            left = Expression::Pipe {
                left: Box::new(left),
                right: Box::new(right),
            };
            self.skip_newlines_if_continuation();
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
        let mut left = self.parse_power()?;

        loop {
            let op = match self.current_kind() {
                TokenKind::Star => BinaryOperator::Mul,
                TokenKind::Slash => BinaryOperator::Div,
                TokenKind::DoubleSlash => BinaryOperator::IntDiv,
                TokenKind::Percent => BinaryOperator::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expression, Box<dyn std::error::Error>> {
        let base = self.parse_unary()?;

        // ** is right-associative: 2 ** 3 ** 2 = 2 ** (3 ** 2)
        if self.at(TokenKind::Power) {
            self.advance();
            let exponent = self.parse_power()?; // right-recursive for right-associativity
            Ok(Expression::BinaryOp {
                left: Box::new(base),
                op: BinaryOperator::Power,
                right: Box::new(exponent),
            })
        } else {
            Ok(base)
        }
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
                TokenKind::LBrace => {
                    // Struct init: Name { field: value, ... }
                    // Only parse as struct init if the expression is an identifier
                    if let Expression::Identifier(name) = &expr {
                        let name = name.clone();
                        self.advance(); // skip {
                        let mut fields = Vec::new();
                        while !self.at(TokenKind::RBrace) {
                            let field_name = self.expect(TokenKind::Identifier)?.text.clone();
                            self.expect(TokenKind::Colon)?;
                            let field_value = self.parse_expression()?;
                            fields.push((field_name, field_value));
                            if self.at(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RBrace)?;
                        expr = Expression::StructInit { name, fields };
                    } else {
                        break;
                    }
                }
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
            TokenKind::SelfKw => {
                self.advance();
                Ok(Expression::Identifier("self".to_string()))
            }
            TokenKind::FStringLiteral => {
                let text = self.advance().unwrap().text.clone();
                // text is like f"...content..." - skip first 2 chars and last 1
                let inner = if text.len() >= 3 {
                    &text[2..text.len() - 1]
                } else {
                    ""
                };
                let parts = parse_fstring_parts(inner);
                Ok(Expression::FString(parts))
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
        // Allow keyword types: none, Self
        let name = match self.current_kind() {
            TokenKind::None => {
                self.advance();
                "none".to_string()
            }
            TokenKind::SelfKw => {
                self.advance();
                "Self".to_string()
            }
            _ => self.expect(TokenKind::Identifier)?.text.clone(),
        };

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

/// Parse the inner content of an f-string into literal and expression parts.
fn parse_fstring_parts(inner: &str) -> Vec<crate::ast::FStringPart> {
    use crate::ast::FStringPart;
    use crate::lexer;

    let mut parts = Vec::new();
    let bytes = inner.as_bytes();
    let mut literal_start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Flush literal before this brace
            if i > literal_start {
                parts.push(FStringPart::Literal(inner[literal_start..i].to_string()));
            }
            let expr_start = i + 1;
            let mut depth = 1usize;
            let mut expr_end = expr_start;
            let mut j = expr_start;
            while j < bytes.len() {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            expr_end = j;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let expr_src = &inner[expr_start..expr_end];
            i = expr_end + 1;
            literal_start = i;

            // Parse the expression inside {}
            let parsed = lexer::tokenize(expr_src)
                .ok()
                .and_then(|tokens| crate::parser::parse(tokens).ok())
                .and_then(|mut prog| prog.statements.pop())
                .and_then(|stmt| {
                    if let crate::ast::Statement::Expression(e) = stmt {
                        Some(e)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| crate::ast::Expression::Identifier(expr_src.trim().to_string()));
            parts.push(FStringPart::Expression(parsed));
        } else {
            i += 1;
        }
    }

    // Remaining literal
    if literal_start < inner.len() {
        parts.push(FStringPart::Literal(inner[literal_start..].to_string()));
    }

    parts
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
