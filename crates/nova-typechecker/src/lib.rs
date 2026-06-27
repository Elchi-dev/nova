pub mod checker;
pub mod env;
pub mod error;
pub mod types;

pub use checker::Checker;
pub use env::Env;
pub use error::TypeError;
pub use types::{type_expr_to_type, types_compatible, Type};

use nova_parser::Program;

/// Result of a type-checking pass.
pub struct TypeCheckResult {
    pub errors: Vec<TypeError>,
}

impl TypeCheckResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Print all errors using ariadne (rich source-aware diagnostics).
    pub fn render_errors(&self, filename: &str, source: &str) {
        for err in &self.errors {
            err.render(filename, source);
        }
    }
}

/// Type-check a parsed program.
///
/// Pass a fresh `Env::with_builtins()` to get built-in functions pre-registered.
pub fn type_check(program: &Program, env: &mut Env) -> TypeCheckResult {
    let mut checker = Checker::new();
    checker.check_program(program, env);
    TypeCheckResult {
        errors: checker.errors,
    }
}

#[cfg(test)]
mod tests;
