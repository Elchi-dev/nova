use super::types::{Effect, Type};
use thiserror::Error;

/// All possible type checking errors
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("type mismatch: expected `{expected}`, found `{found}`")]
    Mismatch { expected: Type, found: Type },

    #[error("undefined variable `{name}`")]
    UndefinedVariable { name: String },

    #[error("undefined function `{name}`")]
    UndefinedFunction { name: String },

    #[error("undefined type `{name}`")]
    UndefinedType { name: String },

    #[error("cannot assign to immutable variable `{name}`")]
    ImmutableAssignment { name: String },

    #[error("`{name}` is already defined in this scope")]
    AlreadyDefined { name: String },

    #[error("function `{name}` expects {expected} arguments, got {found}")]
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
    },

    #[error("cannot call non-function type `{ty}`")]
    NotCallable { ty: Type },

    #[error("type `{ty}` has no field `{field}`")]
    NoSuchField { ty: Type, field: String },

    #[error("type `{ty}` has no method `{method}`")]
    NoSuchMethod { ty: Type, method: String },

    #[error("cannot index into type `{ty}`")]
    NotIndexable { ty: Type },

    #[error("condition must be `bool`, found `{found}`")]
    NonBoolCondition { found: Type },

    #[error("cannot use operator `{op}` with types `{left}` and `{right}`")]
    InvalidOperator {
        op: String,
        left: Type,
        right: Type,
    },

    #[error("cannot negate type `{ty}`")]
    InvalidNegation { ty: Type },

    #[error("pure function `{name}` cannot have side effects")]
    PurityViolation { name: String },

    #[error("effect `{effect}` not declared — add `[{effect}]` to function signature")]
    UndeclaredEffect { effect: Effect },

    #[error("return type mismatch in `{name}`: expected `{expected}`, found `{found}`")]
    ReturnTypeMismatch {
        name: String,
        expected: Type,
        found: Type,
    },

    #[error("missing return value — function expects `{expected}`")]
    MissingReturn { expected: Type },

    #[error("cannot use `break` outside of a loop")]
    BreakOutsideLoop,

    #[error("cannot use `continue` outside of a loop")]
    ContinueOutsideLoop,

    #[error("cannot infer type — add a type annotation")]
    CannotInfer,

    #[error("infinite type detected: `{var}` occurs in `{ty}`")]
    InfiniteType { var: String, ty: Type },

    #[error("duplicate field `{field}` in struct `{struct_name}`")]
    DuplicateField {
        struct_name: String,
        field: String,
    },

    #[error("missing field `{field}` in struct `{struct_name}`")]
    MissingField {
        struct_name: String,
        field: String,
    },

    #[error("match is not exhaustive — missing patterns")]
    NonExhaustiveMatch,
}

/// A type error with source location information
#[derive(Debug)]
pub struct Located<T> {
    pub inner: T,
    pub line: usize,
    pub col: usize,
}

impl<T: std::fmt::Display> std::fmt::Display for Located<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line + 1, self.inner)
    }
}

impl<T: std::error::Error> std::error::Error for Located<T> {}
