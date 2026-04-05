#[cfg(test)]
mod parser_tests {
    use crate::{ast::*, parse};

    // ── Helper ────────────────────────────────────────────────────────────────

    /// Parse source and assert no errors. Return the item list.
    fn ok(src: &str) -> Vec<Item> {
        let (prog, errs) = parse(src, "test.nv");
        assert!(
            errs.is_empty(),
            "unexpected parse errors in {:?}:\n{:#?}",
            src,
            errs
        );
        prog.items
    }

    /// Parse source and assert at least one error.
    #[allow(dead_code)]
    fn err(src: &str) {
        let (_, errs) = parse(src, "test.nv");
        assert!(!errs.is_empty(), "expected parse error in {:?}", src);
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    #[test]
    fn parse_int_literal() {
        let items = ok("42");
        assert!(matches!(items[0], Item::Expr(Expr::Int(42, _))));
    }

    #[test]
    fn parse_float_literal() {
        let items = ok("2.5");
        assert!(matches!(items[0], Item::Expr(Expr::Float(_, _))));
    }

    #[test]
    fn parse_string_literal() {
        let items = ok(r#""hello""#);
        assert!(matches!(&items[0], Item::Expr(Expr::Str(s, _)) if s == "hello"));
    }

    #[test]
    fn parse_bool_literals() {
        let items = ok("true");
        assert!(matches!(items[0], Item::Expr(Expr::Bool(true, _))));
        let items = ok("false");
        assert!(matches!(items[0], Item::Expr(Expr::Bool(false, _))));
    }

    #[test]
    fn parse_binary_add() {
        let items = ok("1 + 2");
        assert!(matches!(
            items[0],
            Item::Expr(Expr::BinOp { op: BinOp::Add, .. })
        ));
    }

    #[test]
    fn parse_binary_precedence() {
        // 1 + 2 * 3  should parse as  1 + (2 * 3)
        let items = ok("1 + 2 * 3");
        match &items[0] {
            Item::Expr(Expr::BinOp {
                op: BinOp::Add,
                rhs,
                ..
            }) => {
                assert!(matches!(**rhs, Expr::BinOp { op: BinOp::Mul, .. }));
            }
            other => panic!("expected Add at top level, got {other:?}"),
        }
    }

    #[test]
    fn parse_unary_neg() {
        let items = ok("-1");
        assert!(matches!(
            items[0],
            Item::Expr(Expr::UnaryOp {
                op: UnaryOp::Neg,
                ..
            })
        ));
    }

    #[test]
    fn parse_call_no_args() {
        let items = ok("foo()");
        assert!(matches!(
            &items[0],
            Item::Expr(Expr::Call { args, .. }) if args.is_empty()
        ));
    }

    #[test]
    fn parse_call_with_args() {
        let items = ok("add(1, 2)");
        assert!(matches!(
            &items[0],
            Item::Expr(Expr::Call { args, .. }) if args.len() == 2
        ));
    }

    #[test]
    fn parse_field_access() {
        let items = ok("obj.field");
        assert!(matches!(
            &items[0],
            Item::Expr(Expr::Field { field, .. }) if field == "field"
        ));
    }

    #[test]
    fn parse_array_literal() {
        let items = ok("[1, 2, 3]");
        assert!(matches!(
            &items[0],
            Item::Expr(Expr::Array(elems, _)) if elems.len() == 3
        ));
    }

    #[test]
    fn parse_comparison() {
        let items = ok("x == 5");
        assert!(matches!(
            items[0],
            Item::Expr(Expr::BinOp { op: BinOp::Eq, .. })
        ));
    }

    // ── Statements ────────────────────────────────────────────────────────────

    #[test]
    fn parse_let_stmt() {
        let items = ok("fn f():\n    let x = 5");
        match &items[0] {
            Item::Function(f) => {
                assert!(matches!(f.body[0], Stmt::Let { .. }));
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_var_stmt_with_type() {
        let items = ok("fn f():\n    var count: i64 = 0");
        match &items[0] {
            Item::Function(f) => {
                assert!(matches!(f.body[0], Stmt::Var { .. }));
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_return_stmt() {
        let items = ok("fn f():\n    return 42");
        match &items[0] {
            Item::Function(f) => {
                assert!(matches!(f.body[0], Stmt::Return { .. }));
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_if_stmt() {
        let src = "fn f():\n    if x:\n        pass";
        let items = ok(src);
        match &items[0] {
            Item::Function(f) => {
                assert!(matches!(f.body[0], Stmt::If { .. }));
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_for_stmt() {
        let src = "fn f():\n    for x in xs:\n        pass";
        let items = ok(src);
        match &items[0] {
            Item::Function(f) => {
                assert!(matches!(f.body[0], Stmt::For { .. }));
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    // ── Functions ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_simple_function() {
        let items = ok("fn main():\n    pass");
        assert!(matches!(&items[0], Item::Function(f) if f.name == "main"));
    }

    #[test]
    fn parse_function_with_params_and_return() {
        let src = "fn add(a: i64, b: i64) -> i64:\n    return a";
        let items = ok(src);
        match &items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert!(f.return_ty.is_some());
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_function_with_decorator() {
        let src = "@dist\nfn crunch(data: []f64) -> f64:\n    pass";
        let items = ok(src);
        match &items[0] {
            Item::Function(f) => {
                assert_eq!(f.decorators.len(), 1);
                assert_eq!(f.decorators[0].name, "dist");
            }
            other => panic!("expected Function, got {other:?}"),
        }
    }

    // ── Imports ───────────────────────────────────────────────────────────────

    #[test]
    fn parse_import() {
        let items = ok("import std.io");
        assert!(matches!(&items[0], Item::Import(i) if i.path == ["std", "io"]));
    }

    #[test]
    fn parse_import_with_alias() {
        let items = ok("import std.io as io");
        match &items[0] {
            Item::Import(i) => {
                assert_eq!(i.alias.as_deref(), Some("io"));
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    // ── Structs ───────────────────────────────────────────────────────────────

    #[test]
    fn parse_struct() {
        let src = "struct Point:\n    x: f64\n    y: f64";
        let items = ok(src);
        match &items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
            }
            other => panic!("expected Struct, got {other:?}"),
        }
    }
}
