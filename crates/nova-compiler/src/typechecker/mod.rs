pub mod check;
pub mod env;
pub mod error;
pub mod types;
pub mod unify;

use crate::ast::Program;
use check::{CheckResult, Checker};

/// Type-check a Nova program.
///
/// Returns a CheckResult containing any type errors and warnings.
/// The checker continues past errors where possible to report
/// as many issues as it can in a single pass.
pub fn check(program: &Program) -> CheckResult {
    let mut checker = Checker::new();
    checker.check_program(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;
    use error::TypeError;

    fn check_source(source: &str) -> CheckResult {
        let tokens = lexer::tokenize(source).unwrap();
        let program = parser::parse(tokens).unwrap();
        check(&program)
    }

    #[test]
    fn test_valid_function() {
        let result = check_source("fn add(a: int, b: int) -> int:\n    return a + b");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_type_mismatch_in_let() {
        let result = check_source("let x: int = \"hello\"");
        assert!(
            !result.errors.is_empty(),
            "should error on int = string"
        );
    }

    #[test]
    fn test_undefined_variable() {
        let result = check_source("let x: int = y");
        assert!(result.errors.iter().any(|e| matches!(
            e,
            TypeError::UndefinedVariable { .. }
        )));
    }

    #[test]
    fn test_immutable_assignment() {
        let result = check_source("let x: int = 1\nx = 2");
        assert!(result.errors.iter().any(|e| matches!(
            e,
            TypeError::ImmutableAssignment { .. }
        )));
    }

    #[test]
    fn test_mutable_assignment() {
        let result = check_source("let mut x: int = 1\nx = 2");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_non_bool_condition() {
        let result = check_source("if 42:\n    let x: int = 1");
        assert!(result.errors.iter().any(|e| matches!(
            e,
            TypeError::NonBoolCondition { .. }
        )));
    }

    #[test]
    fn test_break_outside_loop() {
        let result = check_source("break");
        assert!(result.errors.iter().any(|e| matches!(
            e,
            TypeError::BreakOutsideLoop
        )));
    }

    #[test]
    fn test_for_loop_variable_typing() {
        let result = check_source(
            "let items: list[int] = [1, 2, 3]\nfor x in items:\n    let y: int = x",
        );
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_pipe_operator() {
        let result = check_source(
            "let data: list[int] = [1, 2, 3]\nlet result = data |> sum",
        );
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_field_access() {
        let result = check_source(
            "struct Point:\n    let x: int = 0\n    let y: int = 0\nlet p = Point { x: 1, y: 2 }\nlet val: int = p.x",
        );
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_lambda_inference() {
        let result = check_source("let f = x => x + 1");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }
}
