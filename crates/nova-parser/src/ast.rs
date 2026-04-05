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
    /// key = "value"
    KeyValue { key: String, value: String },
    /// bare identifier
    Ident(String),
}

/// Top-level items in a Nova file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    /// fn name(params) -> ret: body
    Function(Function),
    /// module Name: body
    Module(ModuleDecl),
    /// struct Name: fields
    Struct(StructDecl),
    /// import path [as alias]
    Import(ImportDecl),
    /// @runtime: block — entrypoint config
    RuntimeConfig(RuntimeConfig),
    /// Top-level expression statement
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

/// The @runtime: block at the top of main.nv
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub entries: Vec<(String, String)>,
    pub span: Span,
}

// ── Statements ───────────────────────────────────────────────────────────────

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
    Break { span: Span },
    Continue { span: Span },
    Pass { span: Span },
    Expr(Expr),
    Item(Box<Item>),
}

// ── Expressions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    /// Integer literal
    Int(i64, Span),
    /// Float literal
    Float(f64, Span),
    /// String literal
    Str(String, Span),
    /// Boolean literal
    Bool(bool, Span),
    /// Variable / name reference
    Ident(String, Span),
    /// Binary operation: a + b, a == b, etc.
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Unary operation: !x, -x
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    /// Function call: f(args)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field access: module.field
    Field {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Index: arr[i]
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// Array literal: [a, b, c]
    Array(Vec<Expr>, Span),
    /// f-string: f"Hello, {name}"
    FStr(Vec<FStrPart>, Span),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FStrPart {
    Literal(String),
    Interpolated(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, NotEq, Lt, LtEq, Gt, GtEq,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeExpr {
    Named(String, Span),
    Array(Box<TypeExpr>, Span),
    Optional(Box<TypeExpr>, Span),
}

// ── Top-level program ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
    pub source_file: String,
}
