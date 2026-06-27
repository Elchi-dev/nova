#[cfg(test)]
mod tc_tests {
    use nova_parser::{ast::*, parse};

    use crate::{type_check, Checker, Env, Type};

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Parse + type-check and assert no errors.
    fn check_ok(src: &str) {
        let (prog, parse_errs) = parse(src, "test.nv");
        assert!(
            parse_errs.is_empty(),
            "unexpected parse errors in {src:?}:\n{parse_errs:#?}"
        );
        let mut env = Env::with_builtins();
        let result = type_check(&prog, &mut env);
        assert!(
            result.is_ok(),
            "unexpected type errors in {src:?}:\n{:#?}",
            result.errors
        );
    }

    /// Parse + type-check and assert at least one error.
    fn check_err(src: &str) -> Vec<crate::TypeError> {
        let (prog, parse_errs) = parse(src, "test.nv");
        assert!(
            parse_errs.is_empty(),
            "unexpected parse errors in {src:?}:\n{parse_errs:#?}"
        );
        let mut env = Env::with_builtins();
        let result = type_check(&prog, &mut env);
        assert!(
            !result.is_ok(),
            "expected type errors in {src:?}, but got none"
        );
        result.errors
    }

    /// Infer the type of a bare expression by directly invoking the checker
    /// on the parsed expression node — no scope machinery needed.
    fn infer(src: &str) -> Type {
        // Wrap in a function so the parser can handle it.
        let wrapped = format!("fn __test__():\n    let __x__ = {src}");
        let (prog, _) = parse(&wrapped, "test.nv");

        // Extract the value expression from the let binding.
        if let Some(Item::Function(f)) = prog.items.first() {
            if let Some(Stmt::Let { value, .. }) = f.body.first() {
                let mut env = Env::with_builtins();
                let mut checker = Checker::new();
                // Infer directly — we never push/pop scopes so the result
                // is always available and not affected by scope teardown.
                return checker.infer_expr(value, &mut env).resolve_default();
            }
        }
        Type::Unknown
    }

    // ── Literals ──────────────────────────────────────────────────────────────

    #[test]
    fn infer_int_defaults_to_i64() {
        assert_eq!(infer("42"), Type::I64);
    }

    #[test]
    fn infer_float_defaults_to_f64() {
        assert_eq!(infer("2.5"), Type::F64);
    }

    #[test]
    fn infer_bool() {
        assert_eq!(infer("true"), Type::Bool);
    }

    #[test]
    fn infer_str() {
        assert_eq!(infer(r#""hello""#), Type::Str);
    }

    #[test]
    fn infer_array() {
        assert_eq!(infer("[1, 2, 3]"), Type::Array(Box::new(Type::I64)));
    }

    // ── Arithmetic ────────────────────────────────────────────────────────────

    #[test]
    fn add_ints_ok() {
        check_ok("fn f():\n    let x = 1 + 2");
    }

    #[test]
    fn add_floats_ok() {
        check_ok("fn f():\n    let x = 1.0 + 2.0");
    }

    #[test]
    fn add_int_str_err() {
        check_err(concat!("fn f():\n", "    let x = 1 + \"hello\""));
    }

    #[test]
    fn unary_neg_int_ok() {
        check_ok("fn f():\n    let x = -5");
    }

    #[test]
    fn unary_neg_bool_err() {
        check_err("fn f():\n    let x = -true");
    }

    // ── Variable bindings ─────────────────────────────────────────────────────

    #[test]
    fn let_with_annotation_ok() {
        check_ok("fn f():\n    let x: i64 = 5");
    }

    #[test]
    fn let_with_wrong_annotation_err() {
        check_err("fn f():\n    let x: bool = 5");
    }

    #[test]
    fn var_is_mutable() {
        check_ok("fn f():\n    var x: i64 = 0\n    x = 1");
    }

    #[test]
    fn let_is_immutable_err() {
        check_err("fn f():\n    let x = 0\n    x = 1");
    }

    // ── Undefined names ───────────────────────────────────────────────────────

    #[test]
    fn undefined_name_err() {
        let errs = check_err("fn f():\n    let x = undefined_name");
        assert!(errs
            .iter()
            .any(|e| matches!(e, crate::TypeError::Undefined { .. })));
    }

    // ── Return types ──────────────────────────────────────────────────────────

    #[test]
    fn correct_return_type_ok() {
        check_ok("fn add(a: i64, b: i64) -> i64:\n    return a");
    }

    #[test]
    fn wrong_return_type_err() {
        check_err("fn greet() -> i64:\n    return \"hello\"");
    }

    #[test]
    fn void_return_ok() {
        check_ok("fn f():\n    return");
    }

    // ── If / conditions ───────────────────────────────────────────────────────

    #[test]
    fn if_bool_condition_ok() {
        check_ok("fn f():\n    if true:\n        pass");
    }

    #[test]
    fn if_int_condition_err() {
        check_err("fn f():\n    if 1:\n        pass");
    }

    #[test]
    fn if_else_ok() {
        check_ok("fn f():\n    if true:\n        pass\n    else:\n        pass");
    }

    // ── For loops ─────────────────────────────────────────────────────────────

    #[test]
    fn for_over_array_ok() {
        check_ok("fn f():\n    for x in [1, 2, 3]:\n        pass");
    }

    #[test]
    fn for_over_non_iterable_err() {
        check_err("fn f():\n    for x in 42:\n        pass");
    }

    // ── Function calls ────────────────────────────────────────────────────────

    #[test]
    fn call_builtin_print_ok() {
        check_ok("fn f():\n    print(\"hello\")");
    }

    #[test]
    fn call_user_fn_ok() {
        check_ok("fn add(a: i64, b: i64) -> i64:\n    return a\n\nfn f():\n    let x = add(1, 2)");
    }

    #[test]
    fn call_wrong_arg_count_err() {
        check_err("fn add(a: i64) -> i64:\n    return a\n\nfn f():\n    let x = add(1, 2)");
    }

    #[test]
    fn call_non_function_err() {
        check_err("fn f():\n    let x = 42\n    x()");
    }

    // ── Arrays ────────────────────────────────────────────────────────────────

    #[test]
    fn homogeneous_array_ok() {
        check_ok("fn f():\n    let xs = [1, 2, 3]");
    }

    #[test]
    fn mixed_array_err() {
        check_err("fn f():\n    let xs = [1, \"two\", 3]");
    }

    // ── Forward references ────────────────────────────────────────────────────

    #[test]
    fn forward_reference_ok() {
        // f calls g which is defined after f — two-pass hoisting must handle this.
        check_ok("fn f():\n    g()\n\nfn g():\n    pass");
    }
}
