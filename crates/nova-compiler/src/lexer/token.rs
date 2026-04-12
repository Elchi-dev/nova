use logos::Logos;

/// A single token with its kind, source location, and text
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    /// (line, column, length)
    pub span: (usize, usize, usize),
    pub text: String,
}

/// All token types in the Nova language
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t]+")]
pub enum TokenKind {
    // ── Keywords ──────────────────────────────────────────────
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("const")]
    Const,
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
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("case")]
    Case,
    #[token("import")]
    Import,
    #[token("from")]
    From,
    #[token("as")]
    As,
    #[token("foreign")]
    Foreign,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("trait")]
    Trait,
    #[token("impl")]
    Impl,
    #[token("self")]
    SelfKw,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("none")]
    None,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("in")]
    In,
    #[token("is")]
    Is,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("spawn")]
    Spawn,
    #[token("yield")]
    Yield,
    #[token("type")]
    Type,
    #[token("pub")]
    Pub,

    // ── Nova-specific keywords ───────────────────────────────
    #[token("require")]
    Require,
    #[token("ensure")]
    Ensure,
    #[token("pure")]
    Pure,

    // ── Literals ──────────────────────────────────────────────
    #[regex(r"[0-9][0-9_]*")]
    IntLiteral,
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*")]
    FloatLiteral,
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,
    #[regex(r#"f"([^"\\]|\\.)*""#)]
    FStringLiteral,
    #[regex(r"'([^'\\]|\\.)'")]
    CharLiteral,

    // ── Identifiers ──────────────────────────────────────────
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // ── Operators ─────────────────────────────────────────────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("//")]
    DoubleSlash,
    #[token("%")]
    Percent,
    #[token("**")]
    Power,

    #[token("=")]
    Assign,
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,

    #[token("==")]
    Eq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,

    #[token("|>")]
    Pipe,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,

    // ── Delimiters ────────────────────────────────────────────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("@")]
    At,
    #[token("&")]
    Ampersand,

    // ── Indentation & structure (emitted by tokenizer) ────────
    Indent,
    Dedent,
    Newline,
    Comment,
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Pipe => write!(f, "|>"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::At => write!(f, "@"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::IntLiteral => write!(f, "integer"),
            TokenKind::FloatLiteral => write!(f, "float"),
            TokenKind::StringLiteral => write!(f, "string"),
            TokenKind::Eof => write!(f, "end of file"),
            _ => write!(f, "{:?}", self),
        }
    }
}
