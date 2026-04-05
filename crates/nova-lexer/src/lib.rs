pub mod token;
pub mod error;
pub mod span;

pub use token::Token;
pub use error::LexError;
pub use span::Span;

use logos::Logos;

/// Lex a Nova source string into a vector of (Token, Span) pairs.
/// Errors are collected and returned alongside any successfully lexed tokens.
pub fn lex(source: &str) -> (Vec<(Token, Span)>, Vec<LexError>) {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut lexer = Token::lexer(source);

    while let Some(result) = lexer.next() {
        let span = Span::from(lexer.span());
        match result {
            Ok(token) => tokens.push((token, span)),
            Err(_) => errors.push(LexError::UnexpectedChar {
                ch: lexer.slice().chars().next().unwrap_or('?'),
                span,
            }),
        }
    }

    (tokens, errors)
}
