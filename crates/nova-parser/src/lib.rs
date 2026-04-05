pub mod ast;
pub mod error;
pub mod parser;
pub mod preprocess;

pub use ast::Program;
pub use error::ParseError;

use chumsky::Parser as _;
use nova_lexer::lex;
use preprocess::preprocess;

/// Parse a Nova source string into a `Program` AST.
///
/// Returns the program (possibly partial) and a list of errors. Errors are
/// non-fatal where possible — the parser attempts to recover and continue.
pub fn parse(source: &str, filename: &str) -> (Program, Vec<ParseError>) {
    // ── 1. Lex ────────────────────────────────────────────────────────────────
    let (raw_tokens, lex_errors) = lex(source);

    let mut errors: Vec<ParseError> = lex_errors.into_iter().map(ParseError::Lex).collect();

    // ── 2. Preprocess (Indent → BlockStart/BlockEnd) ──────────────────────────
    let tokens = preprocess(raw_tokens);

    // ── 3. Build the token stream chumsky expects ─────────────────────────────
    let token_stream = chumsky::Stream::from_iter(
        // End-of-input span: one byte past the end of source
        source.len()..source.len() + 1,
        tokens
            .into_iter()
            .map(|(tok, span)| (tok, span.start..span.end)),
    );

    // ── 4. Parse ──────────────────────────────────────────────────────────────
    let (items, parse_errors) = parser::program_parser().parse_recovery(token_stream);

    for e in parse_errors {
        errors.push(ParseError::Chumsky(format!("{e:?}")));
    }

    let program = Program {
        items: items.unwrap_or_default(),
        source_file: filename.to_string(),
    };

    (program, errors)
}

#[cfg(test)]
mod tests;
