pub mod ast;
pub mod error;

pub use ast::Program;
pub use error::ParseError;

use nova_lexer::{lex, Span, Token};

/// Parse Nova source into a Program AST.
/// Returns the program and any parse errors (errors are non-fatal where possible).
pub fn parse(source: &str, filename: &str) -> (Program, Vec<ParseError>) {
    let (tokens, lex_errors) = lex(source);

    let mut errors: Vec<ParseError> = lex_errors.into_iter().map(ParseError::Lex).collect();

    // Parser is work-in-progress — will be implemented with chumsky
    // For now return an empty program so the pipeline compiles end-to-end
    let program = Program {
        items: vec![],
        source_file: filename.to_string(),
    };

    // Silence unused variable warning during bootstrap
    let _ = tokens;

    (program, errors)
}
