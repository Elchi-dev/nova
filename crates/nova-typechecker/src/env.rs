use std::collections::HashMap;

use crate::types::Type;

/// A lexical scope: maps names to their types.
type Scope = HashMap<String, Binding>;

/// A binding in the symbol table.
#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Type,
    /// Whether the binding was declared `var` (mutable) or `let` (immutable).
    pub mutable: bool,
}

impl Binding {
    pub fn immutable(ty: Type) -> Self {
        Self { ty, mutable: false }
    }

    pub fn mutable(ty: Type) -> Self {
        Self { ty, mutable: true }
    }
}

/// A stack of lexical scopes.
///
/// - `push_scope` / `pop_scope` bracket a new block.
/// - `define` adds a name to the innermost scope.
/// - `lookup` searches from innermost to outermost.
#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<Scope>,

    /// Return type expected by the current function.
    /// `None` at the top level (not inside a function).
    pub current_return_ty: Option<Type>,

    /// Whether we are inside a loop (for `break`/`continue` checking).
    pub in_loop: bool,
}

impl Env {
    /// Create an empty environment with just the global scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
            current_return_ty: None,
            in_loop: false,
        }
    }

    /// Create an environment pre-populated with Nova's built-in functions.
    pub fn with_builtins() -> Self {
        let mut env = Self::new();

        // print / println — accept any single argument, return void.
        // We register them with `Unknown` params so they accept anything.
        let print_ty = Type::Function {
            params: vec![Type::Unknown],
            ret: Box::new(Type::Void),
        };
        env.define_builtin("print", print_ty.clone());
        env.define_builtin("println", print_ty);

        // len(arr | str) -> i64
        env.define_builtin(
            "len",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::I64),
            },
        );

        // sum([]numeric) -> f64  (simplified; proper generics in v0.5)
        env.define_builtin(
            "sum",
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::F64),
            },
        );

        // min / max — two unknowns, return the same (simplified)
        for name in ["min", "max"] {
            env.define_builtin(
                name,
                Type::Function {
                    params: vec![Type::Unknown, Type::Unknown],
                    ret: Box::new(Type::Unknown),
                },
            );
        }

        // assert(bool) -> void
        env.define_builtin(
            "assert",
            Type::Function {
                params: vec![Type::Bool],
                ret: Box::new(Type::Void),
            },
        );

        env
    }

    fn define_builtin(&mut self, name: &str, ty: Type) {
        self.scopes
            .first_mut()
            .unwrap()
            .insert(name.to_string(), Binding::immutable(ty));
    }

    /// Push a new inner scope (e.g. entering a block or function body).
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Pop the innermost scope. Panics if there is only the global scope.
    pub fn pop_scope(&mut self) {
        assert!(self.scopes.len() > 1, "cannot pop the global scope");
        self.scopes.pop();
    }

    /// Define a new immutable binding in the current scope.
    pub fn define_let(&mut self, name: String, ty: Type) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, Binding::immutable(ty));
    }

    /// Define a new mutable binding in the current scope.
    pub fn define_var(&mut self, name: String, ty: Type) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, Binding::mutable(ty));
    }

    /// Look up a name, searching from the innermost scope outward.
    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding);
            }
        }
        None
    }

    /// Check if a name is mutable.
    pub fn is_mutable(&self, name: &str) -> bool {
        self.lookup(name).map(|b| b.mutable).unwrap_or(false)
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::with_builtins()
    }
}
