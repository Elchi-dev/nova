use nova_lexer::{Span, Token};

/// Convert the raw lexer output (which uses `Indent(n)` and `Newline` tokens
/// for significant whitespace) into a stream with explicit `BlockStart` and
/// `BlockEnd` markers that the chumsky parser can work with directly.
///
/// # Rules
///
/// - `Newline` at the same indent level → kept as a statement separator.
/// - `Indent(n)` where `n > current` → emit `BlockStart`, push `n`.
/// - `Indent(n)` where `n < current` → emit one `BlockEnd` per popped level,
///   then a `Newline` as statement separator.
/// - `Indent(n)` where `n == current` → emit `Newline` (statement separator).
/// - At EOF, emit `BlockEnd` for every still-open level.
pub fn preprocess(tokens: Vec<(Token, Span)>) -> Vec<(Token, Span)> {
    let mut out: Vec<(Token, Span)> = Vec::with_capacity(tokens.len() + 8);
    let mut stack: Vec<usize> = vec![0];

    for (tok, span) in tokens {
        match tok {
            Token::Newline => {
                out.push((Token::Newline, span));
            }

            Token::Indent(level) => {
                let current = *stack.last().unwrap();

                if level > current {
                    // Deeper indent → open a new block.
                    stack.push(level);
                    out.push((Token::BlockStart, span));
                } else if level < current {
                    // Shallower indent → close blocks until we match.
                    while *stack.last().unwrap() > level {
                        stack.pop();
                        out.push((Token::BlockEnd, span));
                    }
                    // Emit a statement separator at the new level.
                    out.push((Token::Newline, span));
                } else {
                    // Same level → statement separator.
                    out.push((Token::Newline, span));
                }
            }

            other => {
                out.push((other, span));
            }
        }
    }

    // Close any blocks still open at EOF.
    let dummy_span = Span::default();
    while stack.len() > 1 {
        stack.pop();
        out.push((Token::BlockEnd, dummy_span));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_lexer::lex;

    fn pp(src: &str) -> Vec<Token> {
        let (toks, _) = lex(src);
        preprocess(toks).into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn flat_tokens_unchanged() {
        let result = pp("let x = 5");
        assert_eq!(
            result,
            vec![
                Token::Let,
                Token::Ident("x".into()),
                Token::Eq,
                Token::Int(5)
            ]
        );
    }

    #[test]
    fn single_block() {
        // fn main():\n    pass
        let result = pp("fn main():\n    pass");
        assert!(result.contains(&Token::BlockStart), "expected BlockStart");
        assert!(result.contains(&Token::BlockEnd), "expected BlockEnd");
        assert!(result.contains(&Token::Pass));
    }

    #[test]
    fn nested_blocks() {
        let src = "if x:\n    if y:\n        pass\n    return";
        let result = pp(src);
        let starts = result.iter().filter(|t| **t == Token::BlockStart).count();
        let ends = result.iter().filter(|t| **t == Token::BlockEnd).count();
        assert_eq!(starts, ends, "BlockStart/BlockEnd must be balanced");
    }

    #[test]
    fn blocks_balanced_at_eof() {
        let src = "fn f():\n    let x = 1\n    return x";
        let result = pp(src);
        let starts = result.iter().filter(|t| **t == Token::BlockStart).count();
        let ends = result.iter().filter(|t| **t == Token::BlockEnd).count();
        assert_eq!(starts, ends);
    }
}
