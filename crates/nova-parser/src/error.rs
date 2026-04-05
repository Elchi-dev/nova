use nova_lexer::LexError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("lex error: {0}")]
    Lex(#[from] LexError),

    #[error("unexpected token at {span:?}: {message}")]
    UnexpectedToken {
        message: String,
        span: nova_lexer::Span,
    },

    #[error("unexpected end of file")]
    UnexpectedEof,
}
