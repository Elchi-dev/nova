use ariadne::{Color, Label, Report, ReportKind, Source};
use nova_lexer::Span;

use crate::types::Type;

/// A type error produced by the checker.
#[derive(Debug, Clone)]
pub enum TypeError {
    /// A name was used but not defined in any visible scope.
    Undefined { name: String, span: Span },

    /// An expression had the wrong type for its context.
    Mismatch {
        expected: Type,
        found: Type,
        span: Span,
    },

    /// A binary operator was applied to incompatible types.
    BinOpMismatch {
        op: &'static str,
        lty: Type,
        rty: Type,
        span: Span,
    },

    /// A unary operator was applied to an incompatible type.
    InvalidUnary {
        op: &'static str,
        ty: Type,
        span: Span,
    },

    /// A function was called with the wrong number of arguments.
    ArgCount {
        expected: usize,
        found: usize,
        span: Span,
    },

    /// An expression that is not a function was called.
    NotCallable { ty: Type, span: Span },

    /// An expression that is not an array or string was indexed.
    NotIndexable { ty: Type, span: Span },

    /// A `for … in` loop iterated over a non-iterable type.
    NotIterable { ty: Type, span: Span },

    /// An assignment to an immutable `let` binding.
    ImmutableAssign { name: String, span: Span },
}

impl TypeError {
    /// One-line message for CLI output (no source context needed).
    pub fn message(&self) -> String {
        match self {
            TypeError::Undefined { name, .. } => {
                format!("undefined name `{name}`")
            }
            TypeError::Mismatch {
                expected, found, ..
            } => {
                format!("type mismatch: expected `{expected}`, found `{found}`")
            }
            TypeError::BinOpMismatch { op, lty, rty, .. } => {
                format!("operator `{op}` cannot be applied to `{lty}` and `{rty}`")
            }
            TypeError::InvalidUnary { op, ty, .. } => {
                format!("operator `{op}` cannot be applied to `{ty}`")
            }
            TypeError::ArgCount {
                expected, found, ..
            } => {
                format!("wrong number of arguments: expected {expected}, found {found}")
            }
            TypeError::NotCallable { ty, .. } => {
                format!("`{ty}` is not a function")
            }
            TypeError::NotIndexable { ty, .. } => {
                format!("`{ty}` cannot be indexed")
            }
            TypeError::NotIterable { ty, .. } => {
                format!("`{ty}` is not iterable")
            }
            TypeError::ImmutableAssign { name, .. } => {
                format!("cannot assign to `{name}` — it was declared with `let`")
            }
        }
    }

    /// Source span where the error occurred.
    pub fn span(&self) -> Span {
        match self {
            TypeError::Undefined { span, .. }
            | TypeError::Mismatch { span, .. }
            | TypeError::BinOpMismatch { span, .. }
            | TypeError::InvalidUnary { span, .. }
            | TypeError::ArgCount { span, .. }
            | TypeError::NotCallable { span, .. }
            | TypeError::NotIndexable { span, .. }
            | TypeError::NotIterable { span, .. }
            | TypeError::ImmutableAssign { span, .. } => *span,
        }
    }

    /// Render a rich diagnostic to stderr using `ariadne`.
    ///
    /// `filename` should match what was passed to `parse()`.
    /// `source` is the original Nova source string.
    pub fn render(&self, filename: &str, source: &str) {
        let span = self.span();
        let range = span.start..span.end;

        let label_msg = match self {
            TypeError::Mismatch { expected, .. } => {
                format!("expected `{expected}` here")
            }
            TypeError::BinOpMismatch { lty, rty, .. } => {
                format!("left is `{lty}`, right is `{rty}`")
            }
            _ => self.message(),
        };

        Report::build(ReportKind::Error, filename, span.start)
            .with_message(self.message())
            .with_label(
                Label::new((filename, range))
                    .with_message(label_msg)
                    .with_color(Color::Red),
            )
            .finish()
            .print((filename, Source::from(source)))
            .expect("ariadne render failed");
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for TypeError {}
