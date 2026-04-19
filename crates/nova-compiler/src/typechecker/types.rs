use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static TYPE_VAR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique identifier for type variables during inference
pub type TypeVarId = u64;

/// Concrete types in the Nova type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // ── Primitives ──────────────────────────────────────
    Int,
    Float,
    Bool,
    Str,
    Char,
    None,

    // ── Compound types ──────────────────────────────────
    /// `list[T]`
    List(Box<Type>),
    /// `dict[K, V]`
    Dict(Box<Type>, Box<Type>),
    /// `(A, B, C)`
    Tuple(Vec<Type>),
    /// `T?` — optional/nullable
    Optional(Box<Type>),
    /// `T or E` — result type
    Result(Box<Type>, Box<Type>),

    // ── Function type ───────────────────────────────────
    /// `(params) -> return [effects]`
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        effects: Vec<Effect>,
    },

    // ── User-defined types ──────────────────────────────
    /// A struct type with its fields
    Struct(StructType),
    /// An enum type with its variants
    Enum(EnumType),
    /// Reference to a named type (before resolution)
    Named(String),

    // ── Type inference ──────────────────────────────────
    /// An unresolved type variable (placeholder during inference)
    Var(TypeVarId),

    // ── Special ─────────────────────────────────────────
    /// The type of an expression that never returns (e.g. `break`, `return`)
    Never,
    /// Error placeholder — allows type checking to continue after errors
    Error,
}

/// A declared effect that a function may have
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    IO,
    Error,
    Net,
    Custom(String),
}

/// A struct type definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub is_pub: bool,
}

/// An enum type definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<(String, Vec<Type>)>,
    pub is_pub: bool,
}

/// Information about a function for type checking
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub effects: Vec<Effect>,
    pub is_pure: bool,
    pub is_pub: bool,
}

impl Type {
    /// Generate a fresh type variable for inference
    pub fn fresh_var() -> Type {
        Type::Var(TYPE_VAR_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Check if this type contains any unresolved type variables
    pub fn has_vars(&self) -> bool {
        match self {
            Type::Var(_) => true,
            Type::List(t) | Type::Optional(t) => t.has_vars(),
            Type::Dict(k, v) | Type::Result(k, v) => k.has_vars() || v.has_vars(),
            Type::Tuple(ts) => ts.iter().any(|t| t.has_vars()),
            Type::Function {
                params,
                return_type,
                ..
            } => params.iter().any(|t| t.has_vars()) || return_type.has_vars(),
            _ => false,
        }
    }

    /// Substitute type variables using a substitution map
    pub fn apply_substitution(&self, subst: &HashMap<TypeVarId, Type>) -> Type {
        match self {
            Type::Var(id) => {
                if let Some(t) = subst.get(id) {
                    t.apply_substitution(subst)
                } else {
                    self.clone()
                }
            }
            Type::List(t) => Type::List(Box::new(t.apply_substitution(subst))),
            Type::Optional(t) => Type::Optional(Box::new(t.apply_substitution(subst))),
            Type::Dict(k, v) => Type::Dict(
                Box::new(k.apply_substitution(subst)),
                Box::new(v.apply_substitution(subst)),
            ),
            Type::Result(t, e) => Type::Result(
                Box::new(t.apply_substitution(subst)),
                Box::new(e.apply_substitution(subst)),
            ),
            Type::Tuple(ts) => {
                Type::Tuple(ts.iter().map(|t| t.apply_substitution(subst)).collect())
            }
            Type::Function {
                params,
                return_type,
                effects,
            } => Type::Function {
                params: params.iter().map(|t| t.apply_substitution(subst)).collect(),
                return_type: Box::new(return_type.apply_substitution(subst)),
                effects: effects.clone(),
            },
            _ => self.clone(),
        }
    }

    /// Check if this type is numeric (int or float)
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    /// Check if this type is a primitive
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Float | Type::Bool | Type::Str | Type::Char | Type::None
        )
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Char => write!(f, "char"),
            Type::None => write!(f, "none"),
            Type::List(t) => write!(f, "list[{t}]"),
            Type::Dict(k, v) => write!(f, "dict[{k}, {v}]"),
            Type::Tuple(ts) => {
                let inner: Vec<String> = ts.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", inner.join(", "))
            }
            Type::Optional(t) => write!(f, "{t}?"),
            Type::Result(t, e) => write!(f, "{t} or {e}"),
            Type::Function {
                params,
                return_type,
                effects,
            } => {
                let ps: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "({}) -> {}", ps.join(", "), return_type)?;
                if !effects.is_empty() {
                    let es: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
                    write!(f, " [{}]", es.join(", "))?;
                }
                Ok(())
            }
            Type::Struct(s) => write!(f, "{}", s.name),
            Type::Enum(e) => write!(f, "{}", e.name),
            Type::Named(n) => write!(f, "{n}"),
            Type::Var(id) => write!(f, "?T{id}"),
            Type::Never => write!(f, "never"),
            Type::Error => write!(f, "<error>"),
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::IO => write!(f, "io"),
            Effect::Error => write!(f, "error"),
            Effect::Net => write!(f, "net"),
            Effect::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl Effect {
    pub fn parse(s: &str) -> Effect {
        match s {
            "io" => Effect::IO,
            "error" => Effect::Error,
            "net" => Effect::Net,
            other => Effect::Custom(other.to_string()),
        }
    }
}
