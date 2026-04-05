use thiserror::Error;
use crate::Span;

#[derive(Debug, Clone, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}' at {span:?}")]
    UnexpectedChar { ch: char, span: Span },
}
