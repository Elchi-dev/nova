use nova_parser::ParseError;
use nova_typechecker::TypeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("type error: {0}")]
    Type(TypeError),

    #[error("codegen error: {0}")]
    Codegen(String),

    #[error("io error: {0}")]
    Io(String),
}
