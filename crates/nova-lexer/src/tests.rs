/// Lexer integration tests — tokenize Nova source snippets
/// and assert the exact token sequence.
///
/// Run with: cargo test -p nova-lexer
#[cfg(test)]
mod lexer_tests {
    use crate::{lex, Token};

    // ── Helper ────────────────────────────────────────────────────────────────

    /// Lex source and return only the tokens (no spans, no errors).
    fn tokens(src: &str) -> Vec<Token> {
        let (toks, errs) = lex(src);
        assert!(errs.is_empty(), "unexpected lex errors: {errs:?}");
        toks.into_iter().map(|(t, _)| t).collect()
    }

    // ── Literals ─────────────────────────────────────────────────────────────

    #[test]
    fn lex_integer() {
        assert_eq!(tokens("42"), vec![Token::Int(42)]);
    }

    #[test]
    fn lex_float() {
        assert_eq!(tokens("2.5"), vec![Token::Float(2.5)]);
    }

    #[test]
    fn lex_string() {
        assert_eq!(tokens(r#""hello""#), vec![Token::Str("hello".to_string())]);
    }

    #[test]
    fn lex_bool_true() {
        assert_eq!(tokens("true"), vec![Token::True]);
    }

    #[test]
    fn lex_bool_false() {
        assert_eq!(tokens("false"), vec![Token::False]);
    }

    // ── Keywords ─────────────────────────────────────────────────────────────

    #[test]
    fn lex_fn_keyword() {
        assert_eq!(tokens("fn"), vec![Token::Fn]);
    }

    #[test]
    fn lex_let_var() {
        assert_eq!(tokens("let var"), vec![Token::Let, Token::Var]);
    }

    #[test]
    fn lex_module_keyword() {
        assert_eq!(tokens("module"), vec![Token::Module]);
    }

    #[test]
    fn lex_dist_keyword() {
        assert_eq!(tokens("dist"), vec![Token::Dist]);
    }

    // ── Decorators ───────────────────────────────────────────────────────────

    #[test]
    fn lex_decorator_persist() {
        assert_eq!(
            tokens("@persist"),
            vec![Token::Decorator("persist".to_string())]
        );
    }

    #[test]
    fn lex_decorator_dist() {
        assert_eq!(tokens("@dist"), vec![Token::Decorator("dist".to_string())]);
    }

    #[test]
    fn lex_decorator_runtime() {
        assert_eq!(
            tokens("@runtime"),
            vec![Token::Decorator("runtime".to_string())]
        );
    }

    #[test]
    fn lex_decorator_checkpoint() {
        assert_eq!(
            tokens("@checkpoint"),
            vec![Token::Decorator("checkpoint".to_string())]
        );
    }

    // ── Operators ────────────────────────────────────────────────────────────

    #[test]
    fn lex_arithmetic_operators() {
        assert_eq!(
            tokens("+ - * / %"),
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Percent,
            ]
        );
    }

    #[test]
    fn lex_comparison_operators() {
        assert_eq!(
            tokens("== != < <= > >="),
            vec![
                Token::EqEq,
                Token::BangEq,
                Token::Lt,
                Token::LtEq,
                Token::Gt,
                Token::GtEq,
            ]
        );
    }

    #[test]
    fn lex_arrow() {
        assert_eq!(tokens("->"), vec![Token::Arrow]);
    }

    #[test]
    fn lex_assignment_ops() {
        assert_eq!(
            tokens("= += -= *= /="),
            vec![
                Token::Eq,
                Token::PlusEq,
                Token::MinusEq,
                Token::StarEq,
                Token::SlashEq,
            ]
        );
    }

    // ── Types ─────────────────────────────────────────────────────────────────

    #[test]
    fn lex_primitive_types() {
        assert_eq!(
            tokens("i64 f64 bool str void"),
            vec![
                Token::TypeI64,
                Token::TypeF64,
                Token::TypeBool,
                Token::TypeStr,
                Token::TypeVoid,
            ]
        );
    }

    // ── Comments ─────────────────────────────────────────────────────────────

    #[test]
    fn lex_comment_is_skipped() {
        assert_eq!(tokens("# this is a comment"), vec![]);
    }

    #[test]
    fn lex_inline_comment() {
        assert_eq!(tokens("42 # the answer"), vec![Token::Int(42)]);
    }

    // ── Full snippets ─────────────────────────────────────────────────────────

    #[test]
    fn lex_function_signature() {
        // fn add(a: i64, b: i64) -> i64:
        let src = "fn add(a: i64, b: i64) -> i64:";
        let toks = tokens(src);
        assert_eq!(toks[0], Token::Fn);
        assert_eq!(toks[1], Token::Ident("add".to_string()));
        assert_eq!(toks[2], Token::LParen);
        assert_eq!(toks[3], Token::Ident("a".to_string()));
        assert_eq!(toks[4], Token::Colon);
        assert_eq!(toks[5], Token::TypeI64);
        assert_eq!(toks[6], Token::Comma);
        assert_eq!(toks[7], Token::Ident("b".to_string()));
        assert_eq!(toks[8], Token::Colon);
        assert_eq!(toks[9], Token::TypeI64);
        assert_eq!(toks[10], Token::RParen);
        assert_eq!(toks[11], Token::Arrow);
        assert_eq!(toks[12], Token::TypeI64);
        assert_eq!(toks[13], Token::Colon);
    }

    #[test]
    fn lex_decorator_with_parens() {
        // @dist(node="worker-1")  — @dist is one token, rest are separate
        let toks = tokens(r#"@dist(node="worker-1")"#);
        assert_eq!(toks[0], Token::Decorator("dist".to_string()));
        assert_eq!(toks[1], Token::LParen);
        assert_eq!(toks[2], Token::Ident("node".to_string()));
        assert_eq!(toks[3], Token::Eq);
        assert_eq!(toks[4], Token::Str("worker-1".to_string()));
        assert_eq!(toks[5], Token::RParen);
    }

    #[test]
    fn lex_hello_nv() {
        let src = r#"fn main():
    let msg = "Hello, Nova!"
    print(msg)"#;

        let toks = tokens(src);

        // fn main():
        assert_eq!(toks[0], Token::Fn);
        assert_eq!(toks[1], Token::Ident("main".to_string()));
        assert_eq!(toks[2], Token::LParen);
        assert_eq!(toks[3], Token::RParen);
        assert_eq!(toks[4], Token::Colon);
        // newline + indent
        assert!(matches!(toks[5], Token::Indent(_)));
        // let msg = "Hello, Nova!"
        assert_eq!(toks[6], Token::Let);
        assert_eq!(toks[7], Token::Ident("msg".to_string()));
        assert_eq!(toks[8], Token::Eq);
        assert_eq!(toks[9], Token::Str("Hello, Nova!".to_string()));
    }

    // ── Error handling ────────────────────────────────────────────────────────

    #[test]
    fn lex_unknown_char_produces_error() {
        let (toks, errs) = lex("42 $ 99");
        assert!(!errs.is_empty(), "expected a lex error for '$'");
        // valid tokens still come through
        assert!(toks.iter().any(|(t, _)| *t == Token::Int(42)));
        assert!(toks.iter().any(|(t, _)| *t == Token::Int(99)));
    }
}
