use nova_lexer::{Span, Token};

/// Convert the raw lexer output (which uses `Indent(n)` and `Newline` tokens
/// for significant whitespace) into a stream with explicit `BlockStart` and
/// `BlockEnd` markers that the chumsky parser can work with directly.
///
/// # Indentation rules
///
/// | Input token      | Condition           | Output                                 |
/// |------------------|---------------------|----------------------------------------|
/// | `Indent(n)`      | n > current level   | `BlockStart`, push n                   |
/// | `Indent(n)`      | n < current level   | `BlockEnd`×k (pop until n), `Newline`  |
/// | `Indent(n)`      | n == current level  | `Newline`                              |
/// | `Newline`        | next non-blank at 0 | `BlockEnd`×k (pop until 0), `Newline`  |
/// | `Newline`        | otherwise           | `Newline`                              |
///
/// **Key subtlety:** A bare `\n` followed immediately by a non-indented,
/// non-blank token (e.g. a second `fn` at column 0) must close any open
/// blocks, because the logos lexer only emits `Indent(n)` when a newline is
/// followed by whitespace — column-0 lines produce bare `Newline` tokens.
/// We therefore look ahead past consecutive `Newline`s to determine the
/// indentation level of the next logical line.
pub fn preprocess(tokens: Vec<(Token, Span)>) -> Vec<(Token, Span)> {
    let mut out: Vec<(Token, Span)> = Vec::with_capacity(tokens.len() + 8);
    let mut stack: Vec<usize> = vec![0];

    let mut i = 0;
    while i < tokens.len() {
        let (ref tok, span) = tokens[i];

        match tok {
            // ── Explicit indentation ──────────────────────────────────────────
            Token::Indent(level) => {
                let level = *level;
                let current = *stack.last().unwrap();

                if level > current {
                    stack.push(level);
                    out.push((Token::BlockStart, span));
                } else if level < current {
                    while *stack.last().unwrap() > level {
                        stack.pop();
                        out.push((Token::BlockEnd, span));
                    }
                    out.push((Token::Newline, span));
                } else {
                    // same level → statement separator
                    out.push((Token::Newline, span));
                }
                i += 1;
            }

            // ── Bare newline (column-0 lines or blank lines) ──────────────────
            //
            // Look ahead past consecutive Newlines to find the indentation of
            // the *next non-blank logical line*.  If that line starts with an
            // `Indent(n)` token, it will handle its own dedenting; we just emit
            // a plain `Newline` here.  If it starts with a non-`Indent` token
            // (meaning the line is at column 0), we must close any open blocks
            // now, before the parser sees those tokens.
            Token::Newline => {
                out.push((Token::Newline, span));
                i += 1;

                // Skip ahead over consecutive blank Newlines.
                let mut j = i;
                while j < tokens.len() && matches!(tokens[j].0, Token::Newline) {
                    j += 1;
                }

                // j now points at the first non-Newline token (or EOF).
                // If it is NOT an Indent token, the next logical line is at
                // column 0 — close any open blocks.
                if j < tokens.len() && !matches!(tokens[j].0, Token::Indent(_)) {
                    let target_level = 0usize;
                    let current = *stack.last().unwrap();
                    if target_level < current {
                        while *stack.last().unwrap() > target_level {
                            stack.pop();
                            out.push((Token::BlockEnd, span));
                        }
                    }
                }
                // Do NOT advance i here; the next iteration will process the
                // remaining Newlines (and the Indent/token) normally.
            }

            other => {
                out.push((other.clone(), span));
                i += 1;
            }
        }
    }

    // Close any blocks still open at EOF.
    let dummy = Span::default();
    while stack.len() > 1 {
        stack.pop();
        out.push((Token::BlockEnd, dummy));
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

    fn count(toks: &[Token], target: &Token) -> usize {
        toks.iter().filter(|t| *t == target).count()
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
        let result = pp("fn main():\n    pass");
        assert!(result.contains(&Token::BlockStart), "expected BlockStart");
        assert!(result.contains(&Token::BlockEnd), "expected BlockEnd");
        assert!(result.contains(&Token::Pass));
    }

    #[test]
    fn blocks_balanced_at_eof() {
        let src = "fn f():\n    let x = 1\n    return x";
        let result = pp(src);
        assert_eq!(
            count(&result, &Token::BlockStart),
            count(&result, &Token::BlockEnd),
            "BlockStart/BlockEnd must be balanced"
        );
    }

    #[test]
    fn nested_blocks_balanced() {
        let src = "if x:\n    if y:\n        pass\n    return";
        let result = pp(src);
        assert_eq!(
            count(&result, &Token::BlockStart),
            count(&result, &Token::BlockEnd)
        );
    }

    #[test]
    fn two_top_level_functions() {
        // The blank line between the two functions must not leave a dangling block.
        let src = "fn f():\n    pass\n\nfn g():\n    pass";
        let result = pp(src);
        // Two functions → two BlockStart and two BlockEnd.
        assert_eq!(count(&result, &Token::BlockStart), 2);
        assert_eq!(count(&result, &Token::BlockEnd), 2);
        // Both Fn tokens must be present.
        assert_eq!(result.iter().filter(|t| **t == Token::Fn).count(), 2);
    }

    #[test]
    fn two_functions_no_blank_line() {
        let src = "fn f():\n    pass\nfn g():\n    pass";
        let result = pp(src);
        assert_eq!(count(&result, &Token::BlockStart), 2);
        assert_eq!(count(&result, &Token::BlockEnd), 2);
    }
}
