use std::fmt;

/// The complete set of types in Nova's type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // ── Integer types ─────────────────────────────────────────────────────────
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,

    // ── Float types ───────────────────────────────────────────────────────────
    F32,
    F64,

    // ── Primitive types ───────────────────────────────────────────────────────
    Bool,
    Str,
    Void,

    // ── Compound types ────────────────────────────────────────────────────────
    /// `[]T` — a homogeneous array of `T`
    Array(Box<Type>),

    /// A callable: parameters and return type.
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },

    // ── Inference helpers ─────────────────────────────────────────────────────
    /// An unresolved integer literal (`42`). Compatible with any integer type.
    /// Defaults to `i64` when forced to resolve.
    IntLiteral,

    /// An unresolved float literal (`2.5`). Compatible with any float type.
    /// Defaults to `f64` when forced to resolve.
    FloatLiteral,

    /// Type is not yet known (e.g. empty array `[]`, or pending inference).
    /// Compatible with everything — prevents cascading errors.
    Unknown,

    /// A type error occurred. Compatible with everything to stop error storms.
    Error,

    /// The type of `return`, `break`, `continue` — never produces a value.
    Never,
}

impl Type {
    /// Resolve an `IntLiteral` to its default concrete type (`i64`).
    /// Resolve a `FloatLiteral` to its default (`f64`).
    /// All other types are returned as-is.
    pub fn resolve_default(self) -> Type {
        match self {
            Type::IntLiteral => Type::I64,
            Type::FloatLiteral => Type::F64,
            other => other,
        }
    }

    /// Whether this type is any integer variant (including `IntLiteral`).
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::IntLiteral
        )
    }

    /// Whether this type is any float variant (including `FloatLiteral`).
    pub fn is_float(&self) -> bool {
        matches!(self, Type::F32 | Type::F64 | Type::FloatLiteral)
    }

    /// Whether this type is numeric (integer or float).
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// `true` for `Error` and `Unknown` — these suppress further errors.
    pub fn is_poisoned(&self) -> bool {
        matches!(self, Type::Error | Type::Unknown)
    }
}

/// Two types are compatible if:
/// - They are equal, OR
/// - Either side is `Error`/`Unknown` (error recovery), OR
/// - An `IntLiteral` is matched against any integer type, OR
/// - A `FloatLiteral` is matched against any float type.
///
/// Note: Nova does NOT do implicit numeric widening (`i32` ≠ `i64`).
/// The only flexibility is for unsolved literal types.
pub fn types_compatible(expected: &Type, found: &Type) -> bool {
    // Error / Unknown on either side: skip the check.
    if expected.is_poisoned() || found.is_poisoned() {
        return true;
    }
    // Exact structural equality.
    if expected == found {
        return true;
    }
    // Integer literal ↔ any concrete integer type.
    if matches!(found, Type::IntLiteral) && expected.is_integer() {
        return true;
    }
    if matches!(expected, Type::IntLiteral) && found.is_integer() {
        return true;
    }
    // Float literal ↔ any concrete float type.
    if matches!(found, Type::FloatLiteral) && expected.is_float() {
        return true;
    }
    if matches!(expected, Type::FloatLiteral) && found.is_float() {
        return true;
    }
    false
}

/// Convert a `TypeExpr` from the AST into a `Type`.
pub fn type_expr_to_type(te: &nova_parser::ast::TypeExpr) -> Type {
    use nova_parser::ast::TypeExpr;
    match te {
        TypeExpr::Named(name, _) => match name.as_str() {
            "i8" => Type::I8,
            "i16" => Type::I16,
            "i32" => Type::I32,
            "i64" => Type::I64,
            "u8" => Type::U8,
            "u16" => Type::U16,
            "u32" => Type::U32,
            "u64" => Type::U64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            "bool" => Type::Bool,
            "str" => Type::Str,
            "void" => Type::Void,
            _ => Type::Unknown, // user-defined struct types — resolved in v0.4
        },
        TypeExpr::Array(inner, _) => Type::Array(Box::new(type_expr_to_type(inner))),
        TypeExpr::Optional(inner, _) => {
            // TODO: proper optional type in v0.5 — use Unknown for now
            let _ = type_expr_to_type(inner);
            Type::Unknown
        }
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Void => write!(f, "void"),
            Type::Array(inner) => write!(f, "[]{inner}"),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::IntLiteral => write!(f, "{{integer}}"),
            Type::FloatLiteral => write!(f, "{{float}}"),
            Type::Unknown => write!(f, "{{unknown}}"),
            Type::Error => write!(f, "{{error}}"),
            Type::Never => write!(f, "!"),
        }
    }
}
