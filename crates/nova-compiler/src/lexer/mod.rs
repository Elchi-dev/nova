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

/// Tokenize Nova source code into a list of tokens.
///
/// Handles indentation-based scoping by emitting Indent/Dedent tokens,
/// similar to Python's tokenization approach.
pub fn tokenize(source: &str) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
    let mut tokens = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];

    for (line_num, line) in source.lines().enumerate() {
        // Skip empty lines and comment-only lines
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if trimmed.starts_with('#') {
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    span: (line_num, 0, line.len()),
                    text: trimmed.to_string(),
                });
            }
            tokens.push(Token {
                kind: TokenKind::Newline,
                span: (line_num, line.len(), 1),
                text: "\n".to_string(),
            });
            continue;
        }

        // Calculate indentation level
        let indent = line.len() - line.trim_start().len();
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

        // Lex the actual content of the line
        let content = line.trim_start();
        let offset = indent;
        let mut lexer = TokenKind::lexer(content);

        while let Some(result) = lexer.next() {
            let slice = lexer.slice();
            let logo_span = lexer.span();

            match result {
                Ok(kind) => {
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

        tokens.push(Token {
            kind: TokenKind::Newline,
            span: (line_num, line.len(), 1),
            text: "\n".to_string(),
        });
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

        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::Fn));
        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::Return));
        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::Indent));
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

        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::At));
    }

    #[test]
    fn test_inline_braces() {
        let source = "fn double(x: int) -> int { return x * 2; }";
        let tokens = tokenize(source).unwrap();

        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::LBrace));
        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::Semicolon));
    }
}
