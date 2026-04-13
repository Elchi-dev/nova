use std::collections::HashMap;
use std::fmt;

use crate::ast::{Block, Expression, Parameter};

/// A runtime value in the Nova interpreter
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Char(char),
    None,

    /// `[1, 2, 3]`
    List(Vec<Value>),

    /// `{"key": value}`
    Dict(Vec<(Value, Value)>),

    /// A struct instance with its type name and field values
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },

    /// A user-defined or built-in function
    Function(NovaFunction),
}

/// Function representation at runtime
#[derive(Debug, Clone)]
pub enum NovaFunction {
    /// A user-defined function from source code
    UserDefined {
        name: String,
        params: Vec<Parameter>,
        body: Block,
        /// Captured environment for closures
        closure_env: Option<Vec<(String, Value)>>,
    },

    /// A lambda expression: `x => x * 2`
    Lambda {
        params: Vec<String>,
        body: Expression,
        closure_env: Option<Vec<(String, Value)>>,
    },

    /// A built-in function implemented in Rust
    Builtin {
        name: String,
        func: fn(Vec<Value>) -> Result<Value, RuntimeError>,
    },

    /// A partially applied function (from pipes with args)
    Partial {
        func: Box<NovaFunction>,
        applied_args: Vec<Value>,
    },
}

/// Runtime errors
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Division by zero
    DivisionByZero,
    /// Index out of bounds
    IndexOutOfBounds { index: i64, length: usize },
    /// Type error at runtime (shouldn't happen with type checker, but safety net)
    TypeError { message: String },
    /// Undefined variable (shouldn't happen with type checker)
    Undefined { name: String },
    /// Assertion failed (require/ensure)
    ContractViolation { message: String },
    /// Break signal (used internally for loop control)
    Break,
    /// Continue signal (used internally for loop control)
    Continue,
    /// Return signal with value (used internally for function returns)
    Return(Value),
    /// General error
    Error(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(n) => {
                if *n == n.floor() && n.is_finite() {
                    write!(f, "{n:.1}")
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Str(s) => write!(f, "{s}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::None => write!(f, "none"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match item {
                        Value::Str(s) => write!(f, "\"{s}\"")?,
                        other => write!(f, "{other}")?,
                    }
                }
                write!(f, "]")
            }
            Value::Dict(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Struct { type_name, fields } => {
                write!(f, "{type_name} {{ ")?;
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {val}")?;
                }
                write!(f, " }}")
            }
            Value::Function(func) => match func {
                NovaFunction::UserDefined { name, .. } => write!(f, "<fn {name}>"),
                NovaFunction::Lambda { .. } => write!(f, "<lambda>"),
                NovaFunction::Builtin { name, .. } => write!(f, "<builtin {name}>"),
                NovaFunction::Partial { func, .. } => write!(f, "<partial {func:?}>"),
            },
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::IndexOutOfBounds { index, length } => {
                write!(f, "index {index} out of bounds (length {length})")
            }
            RuntimeError::TypeError { message } => write!(f, "type error: {message}"),
            RuntimeError::Undefined { name } => write!(f, "undefined: {name}"),
            RuntimeError::ContractViolation { message } => {
                write!(f, "contract violation: {message}")
            }
            RuntimeError::Break => write!(f, "break outside loop"),
            RuntimeError::Continue => write!(f, "continue outside loop"),
            RuntimeError::Return(val) => write!(f, "return {val}"),
            RuntimeError::Error(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl Value {
    /// Check truthiness (Python-like)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(items) => !items.is_empty(),
            Value::None => false,
            _ => true,
        }
    }

    /// Try to extract as integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Float(n) => Some(*n as i64),
            _ => None,
        }
    }

    /// Try to extract as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(n) => Some(*n),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }
}
