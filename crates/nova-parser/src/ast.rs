use nova_lexer::Span;
use serde::{Deserialize, Serialize};

/// A decorator like `@persist`, `@dist(node="worker-1")`, `@runtime`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<DecoratorArg>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecoratorArg {
    KeyValue { key: String, value: String },
    Ident(String),
}

/// Top-level items in a Nova file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Function(Function),
    Module(ModuleDecl),
    Struct(StructDecl),
    Import(ImportDecl),
    RuntimeConfig(RuntimeConfig),
    Expr(Expr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub decorators: Vec<Decorator>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeExpr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDecl {
    pub decorators: Vec<Decorator>,
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDecl {
    pub decorators: Vec<Decorator>,
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub entries: Vec<(String, String)>,
    pub span: Span,
}

// ── Statements ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeExpr>,
        value: Expr,
        span: Span,
    },
    Var {
        name: String,
        ty: Option<TypeExpr>,
        value: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        elif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Pass {
        span: Span,
    },
    Expr(Expr),
}

impl Stmt {
    /// Extract the inner expression, or wrap the statement in a dummy Ident.
    /// Used when a statement appears where an expression is needed (e.g. in
    /// module item lists during bootstrapping).
    pub fn into_expr(self) -> Expr {
        match self {
            Stmt::Expr(e) => e,
            _ => Expr::Ident("__stmt__".to_string(), Span::default()),
        }
    }
}

// ── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Str(String, Span),
    Bool(bool, Span),
    Ident(String, Span),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Field {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Array(Vec<Expr>, Span),
    FStr(Vec<FStrPart>, Span),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FStrPart {
    Literal(String),
    Interpolated(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeExpr {
    Named(String, Span),
    Array(Box<TypeExpr>, Span),
    Optional(Box<TypeExpr>, Span),
}

// ── Top-level program ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
    pub source_file: String,
}
