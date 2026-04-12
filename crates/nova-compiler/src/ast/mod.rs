/// Represents a complete Nova source file
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Top-level and block-level statements
#[derive(Debug, Clone)]
pub enum Statement {
    /// `fn name(params) -> ReturnType:`
    FunctionDef {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<TypeExpr>,
        effects: Vec<String>,
        body: Block,
        decorators: Vec<Decorator>,
        is_pub: bool,
    },

    /// `struct Name:`
    StructDef {
        name: String,
        fields: Vec<Field>,
        is_pub: bool,
    },

    /// `enum Name:`
    EnumDef {
        name: String,
        variants: Vec<EnumVariant>,
        is_pub: bool,
    },

    /// `trait Name:`
    TraitDef {
        name: String,
        methods: Vec<TraitMethod>,
        is_pub: bool,
    },

    /// `impl Trait for Type:`
    ImplBlock {
        trait_name: Option<String>,
        target_type: String,
        methods: Vec<Statement>,
    },

    /// `let name: Type = value` or `let mut name = value`
    LetBinding {
        name: String,
        type_annotation: Option<TypeExpr>,
        value: Expression,
        mutable: bool,
    },

    /// `const NAME = value`
    ConstBinding {
        name: String,
        type_annotation: Option<TypeExpr>,
        value: Expression,
    },

    /// `import module` or `from module import name`
    Import {
        path: Vec<String>,
        items: Option<Vec<ImportItem>>,
    },

    /// `import foreign("header.h", lang: "c")`
    ForeignImport {
        path: String,
        lang: String,
        items: Option<Vec<String>>,
    },

    /// `if condition:` / `elif:` / `else:`
    If {
        condition: Expression,
        body: Block,
        elif_clauses: Vec<(Expression, Block)>,
        else_body: Option<Block>,
    },

    /// `for item in iterable:`
    ForLoop {
        variable: String,
        iterable: Expression,
        body: Block,
    },

    /// `while condition:`
    WhileLoop {
        condition: Expression,
        body: Block,
    },

    /// `match value:`
    Match {
        subject: Expression,
        arms: Vec<MatchArm>,
    },

    /// `return value`
    Return(Option<Expression>),

    /// `break`
    Break,

    /// `continue`
    Continue,

    /// Expression used as a statement
    Expression(Expression),

    /// Assignment: `target = value`
    Assignment {
        target: Expression,
        value: Expression,
    },
}

/// Expressions that produce a value
#[derive(Debug, Clone)]
pub enum Expression {
    /// Integer literal: `42`
    IntLiteral(i64),

    /// Float literal: `3.14`
    FloatLiteral(f64),

    /// String literal: `"hello"`
    StringLiteral(String),

    /// F-string: `f"hello {name}"`
    FString(Vec<FStringPart>),

    /// Boolean: `true` / `false`
    BoolLiteral(bool),

    /// None value
    NoneLiteral,

    /// Variable reference: `name`
    Identifier(String),

    /// Binary operation: `a + b`
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },

    /// Unary operation: `-x`, `not x`
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },

    /// Function call: `func(args)`
    Call {
        function: Box<Expression>,
        args: Vec<Expression>,
    },

    /// Method call: `obj.method(args)`
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },

    /// Field access: `obj.field`
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },

    /// Index access: `list[0]`
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },

    /// Pipe expression: `data |> transform |> output`
    Pipe {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Lambda: `x => x * 2`
    Lambda {
        params: Vec<String>,
        body: Box<Expression>,
    },

    /// List literal: `[1, 2, 3]`
    List(Vec<Expression>),

    /// Dict literal: `{"key": value}`
    Dict(Vec<(Expression, Expression)>),

    /// Struct construction: `Point { x: 1, y: 2 }`
    StructInit {
        name: String,
        fields: Vec<(String, Expression)>,
    },

    /// `value or ErrorType` — Result type expression
    ResultExpr {
        value: Box<Expression>,
        error_type: String,
    },

    /// Await expression: `await future`
    Await(Box<Expression>),
}

// ── Supporting types ─────────────────────────────────────────

pub type Block = Vec<Statement>;

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: TypeExpr,
    pub default: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_annotation: TypeExpr,
    pub default: Option<Expression>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Option<Vec<TypeExpr>>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub default_body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Expression),
    Variable(String),
    Variant(String, Vec<Pattern>),
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),
    Generic(String, Vec<TypeExpr>),
    Function(Vec<TypeExpr>, Box<TypeExpr>),
    Optional(Box<TypeExpr>),
    Result(Box<TypeExpr>, Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expression(Expression),
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Power,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    In,
    Is,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Neg,
    Not,
}
