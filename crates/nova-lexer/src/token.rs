use logos::Logos;
use serde::{Deserialize, Serialize};

/// All tokens in the Nova language.
///
/// Variants without a `#[token]` or `#[regex]` attribute (`BlockStart`,
/// `BlockEnd`) are synthetic — they are never produced by the logos lexer
/// directly but are inserted by the pre-processor that converts significant
/// indentation into explicit block markers.
#[derive(Logos, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[logos(skip r"[ \t\r]+")] // skip whitespace (not newlines — significant)
#[logos(skip r"#[^\n]*")] // skip line comments
pub enum Token {
    // ── Literals ────────────────────────────────────────────────────────────
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    Str(String),

    #[token("true")]
    True,

    #[token("false")]
    False,

    // ── Keywords ─────────────────────────────────────────────────────────────
    #[token("fn")]
    Fn,

    #[token("let")]
    Let,

    #[token("var")]
    Var,

    #[token("return")]
    Return,

    #[token("if")]
    If,

    #[token("elif")]
    Elif,

    #[token("else")]
    Else,

    #[token("for")]
    For,

    #[token("while")]
    While,

    #[token("in")]
    In,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("module")]
    Module,

    #[token("import")]
    Import,

    #[token("from")]
    From,

    #[token("as")]
    As,

    #[token("struct")]
    Struct,

    #[token("dist")]
    Dist,

    #[token("parallel")]
    Parallel,

    #[token("map")]
    Map,

    #[token("pass")]
    Pass,

    // ── Types ─────────────────────────────────────────────────────────────────
    #[token("i8")]
    TypeI8,
    #[token("i16")]
    TypeI16,
    #[token("i32")]
    TypeI32,
    #[token("i64")]
    TypeI64,
    #[token("u8")]
    TypeU8,
    #[token("u16")]
    TypeU16,
    #[token("u32")]
    TypeU32,
    #[token("u64")]
    TypeU64,
    #[token("f32")]
    TypeF32,
    #[token("f64")]
    TypeF64,
    #[token("bool")]
    TypeBool,
    #[token("str")]
    TypeStr,
    #[token("void")]
    TypeVoid,

    // ── Decorators ───────────────────────────────────────────────────────────
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice()[1..].to_string())]
    Decorator(String),

    // ── Identifiers ───────────────────────────────────────────────────────────
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Operators ─────────────────────────────────────────────────────────────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<")]
    Lt,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,

    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("!")]
    Bang,

    #[token("=")]
    Eq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,

    #[token("->")]
    Arrow,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,

    // ── Delimiters ────────────────────────────────────────────────────────────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // ── Layout tokens (significant whitespace) ────────────────────────────────
    #[token("\n")]
    Newline,

    #[regex(r"\n[ \t]+", |lex| {
        let s = lex.slice();
        s[1..].chars().map(|c| if c == '\t' { 4usize } else { 1 }).sum::<usize>()
    })]
    Indent(usize),

    // ── Synthetic block tokens (inserted by preprocessor, never by logos) ────
    BlockStart,
    BlockEnd,
}

// ── Eq + Hash ────────────────────────────────────────────────────────────────
//
// `f64` does not implement `Eq` or `Hash`, so we can't derive them.
// We implement them manually. NaN tokens should never appear in practice.

impl Eq for Token {}

impl std::hash::Hash for Token {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the discriminant first so different variants never collide.
        std::mem::discriminant(self).hash(state);
        match self {
            Token::Int(n) => n.hash(state),
            Token::Float(f) => f.to_bits().hash(state),
            Token::Str(s) | Token::Decorator(s) | Token::Ident(s) => s.hash(state),
            Token::Indent(n) => n.hash(state),
            // All unit variants are fully distinguished by their discriminant.
            _ => {}
        }
    }
}
