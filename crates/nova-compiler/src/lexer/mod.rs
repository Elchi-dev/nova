mod token;

pub use token::{Token, TokenKind};

use logos::Logos;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// A lexer error with source location
#[derive(Debug, Error, Diagnostic)]
#[error("unexpected character")]
pub struct LexError {
    #[label("here")]
    pub span: SourceSpan,
}

/// Tokens that start a continuation line (indentation ignored when line begins with these)
fn is_continuation_start(content: &str) -> bool {
    let t = content.trim_start();
    t.starts_with("|>")
        || t.starts_with("+ ")
        || t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("/ ")
        || t.starts_with("and ")
        || t.starts_with("or ")
        || t.starts_with(".")
        || t.starts_with(",")
        || t.starts_with(")")
        || t.starts_with("]")
        || t.starts_with("}")
}

/// Tokenize Nova source code into a list of tokens.
///
/// Handles indentation-based scoping by emitting Indent/Dedent tokens.
/// Supports line continuation in two cases:
///   1. Inside open brackets (), [], {} — indentation is ignored
///   2. When next line starts with a continuation operator (|>, +, -, etc.)
pub fn tokenize(source: &str) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
    let mut tokens = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut bracket_depth: i32 = 0;

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Handle blank lines and comment-only lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            // For doc comments, respect indentation — emit dedents if needed
            // so they're attached to the right scope
            let is_doc = trimmed.starts_with("##");
            if is_doc && bracket_depth == 0 {
                let indent = line.len() - line.trim_start().len();
                while indent < *indent_stack.last().unwrap() {
                    indent_stack.pop();
                    tokens.push(Token {
                        kind: TokenKind::Dedent,
                        span: (line_num, 0, 0),
                        text: String::new(),
                    });
                }
            }

            if is_doc {
                tokens.push(Token {
                    kind: TokenKind::DocComment,
                    span: (line_num, 0, line.len()),
                    text: trimmed.trim_start_matches('#').trim().to_string(),
                });
            } else if trimmed.starts_with('#') {
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    span: (line_num, 0, line.len()),
                    text: trimmed.to_string(),
                });
            }
            // Only emit newline if not inside brackets
            if bracket_depth == 0 {
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    span: (line_num, line.len(), 1),
                    text: "\n".to_string(),
                });
            }
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        // Determine if this line is a continuation of the previous
        let is_continuation = bracket_depth > 0 || is_continuation_start(line);

        if !is_continuation {
            // Normal indentation handling
            let current_indent = *indent_stack.last().unwrap();

            if indent > current_indent {
                indent_stack.push(indent);
                tokens.push(Token {
                    kind: TokenKind::Indent,
                    span: (line_num, 0, indent),
                    text: String::new(),
                });
            } else {
                while indent < *indent_stack.last().unwrap() {
                    indent_stack.pop();
                    tokens.push(Token {
                        kind: TokenKind::Dedent,
                        span: (line_num, 0, 0),
                        text: String::new(),
                    });
                }
            }
        }

        // Lex the actual content
        let content = line.trim_start();
        let offset = indent;
        let mut lexer = TokenKind::lexer(content);

        while let Some(result) = lexer.next() {
            let slice = lexer.slice();
            let logo_span = lexer.span();

            match result {
                Ok(kind) => {
                    // Track bracket depth for continuation
                    match kind {
                        TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                            bracket_depth += 1;
                        }
                        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                            bracket_depth -= 1;
                            if bracket_depth < 0 {
                                bracket_depth = 0;
                            }
                        }
                        _ => {}
                    }

                    tokens.push(Token {
                        kind,
                        span: (line_num, offset + logo_span.start, logo_span.len()),
                        text: slice.to_string(),
                    });
                }
                Err(()) => {
                    return Err(Box::new(LexError {
                        span: (offset + logo_span.start, logo_span.len()).into(),
                    }));
                }
            }
        }

        // Only emit newline if not inside brackets
        if bracket_depth == 0 {
            tokens.push(Token {
                kind: TokenKind::Newline,
                span: (line_num, line.len(), 1),
                text: "\n".to_string(),
            });
        }
    }

    // Emit remaining dedents at EOF
    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token {
            kind: TokenKind::Dedent,
            span: (0, 0, 0),
            text: String::new(),
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: (0, 0, 0),
        text: String::new(),
    });

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_function() {
        let source = "fn hello(name: str) -> str:\n    return name";
        let tokens = tokenize(source).unwrap();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::Fn));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Return));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Indent));
    }

    #[test]
    fn test_pipe_operator() {
        let source = "data |> transform |> output";
        let tokens = tokenize(source).unwrap();

        let pipes: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Pipe)
            .collect();
        assert_eq!(pipes.len(), 2);
    }

    #[test]
    fn test_decorator() {
        let source = "@cached\nfn compute() -> int:\n    return 42";
        let tokens = tokenize(source).unwrap();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::At));
    }

    #[test]
    fn test_inline_braces() {
        let source = "fn double(x: int) -> int { return x * 2; }";
        let tokens = tokenize(source).unwrap();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::LBrace));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Semicolon));
    }

    #[test]
    fn test_multiline_pipe() {
        let source = "let x = data\n    |> filter\n    |> sort";
        let tokens = tokenize(source).unwrap();

        // Should not emit Indent for the continuation lines
        let indents: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Indent)
            .collect();
        assert_eq!(indents.len(), 0);

        let pipes: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Pipe)
            .collect();
        assert_eq!(pipes.len(), 2);
    }

    #[test]
    fn test_multiline_list() {
        let source = "let x = [\n    1,\n    2,\n    3\n]";
        let tokens = tokenize(source).unwrap();

        // Should not emit Indent inside brackets
        let indents: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Indent)
            .collect();
        assert_eq!(indents.len(), 0);
    }
}
