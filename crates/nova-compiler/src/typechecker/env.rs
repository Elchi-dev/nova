use std::collections::HashMap;

use super::types::{Effect, FunctionSig, Type};

/// A scope in the type environment
#[derive(Debug, Clone)]
struct Scope {
    /// Variable name → (type, is_mutable)
    variables: HashMap<String, (Type, bool)>,
    /// Function name → signature
    functions: HashMap<String, FunctionSig>,
    /// Type name → type definition
    types: HashMap<String, Type>,
    /// Whether this scope is a function body (for return type checking)
    return_type: Option<Type>,
    /// Effects declared for the current function
    declared_effects: Vec<Effect>,
    /// Whether the current function is pure
    is_pure: bool,
}

/// The type environment tracks all bindings across nested scopes
#[derive(Debug)]
pub struct TypeEnv {
    scopes: Vec<Scope>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            return_type: None,
            declared_effects: Vec::new(),
            is_pure: false,
        }
    }
}

impl TypeEnv {
    /// Create a new environment with built-in types pre-loaded
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope::new()],
        };
        env.register_builtins();
        env
    }

    /// Register built-in types and functions
    fn register_builtins(&mut self) {
        // Built-in types
        self.define_type("int", Type::Int);
        self.define_type("float", Type::Float);
        self.define_type("bool", Type::Bool);
        self.define_type("str", Type::Str);
        self.define_type("char", Type::Char);
        self.define_type("none", Type::None);

        // Built-in functions
        self.define_function(FunctionSig {
            name: "print".to_string(),
            params: vec![("value".to_string(), Type::Str)],
            return_type: Type::None,
            effects: vec![Effect::IO],
            is_pure: false,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "len".to_string(),
            params: vec![("collection".to_string(), Type::fresh_var())],
            return_type: Type::Int,
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "range".to_string(),
            params: vec![("n".to_string(), Type::Int)],
            return_type: Type::List(Box::new(Type::Int)),
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "str".to_string(),
            params: vec![("value".to_string(), Type::fresh_var())],
            return_type: Type::Str,
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        // Collection functions used in pipes
        self.define_function(FunctionSig {
            name: "filter".to_string(),
            params: vec![(
                "predicate".to_string(),
                Type::Function {
                    params: vec![Type::fresh_var()],
                    return_type: Box::new(Type::Bool),
                    effects: vec![],
                },
            )],
            return_type: Type::fresh_var(),
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "map".to_string(),
            params: vec![(
                "transform".to_string(),
                Type::Function {
                    params: vec![Type::fresh_var()],
                    return_type: Box::new(Type::fresh_var()),
                    effects: vec![],
                },
            )],
            return_type: Type::fresh_var(),
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "sort".to_string(),
            params: vec![],
            return_type: Type::fresh_var(),
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });

        self.define_function(FunctionSig {
            name: "sum".to_string(),
            params: vec![],
            return_type: Type::Int,
            effects: vec![],
            is_pure: true,
            is_pub: true,
        });
    }

    // ── Scope management ─────────────────────────────────────

    /// Enter a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Enter a function scope with a return type and effects
    pub fn push_function_scope(&mut self, return_type: Type, effects: Vec<Effect>, is_pure: bool) {
        let mut scope = Scope::new();
        scope.return_type = Some(return_type);
        scope.declared_effects = effects;
        scope.is_pure = is_pure;
        self.scopes.push(scope);
    }

    /// Leave the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    #[allow(dead_code)]
    fn current_scope(&self) -> &Scope {
        self.scopes.last().expect("scope stack empty")
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scope stack empty")
    }

    // ── Variable operations ──────────────────────────────────

    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: &str, ty: Type, mutable: bool) {
        self.current_scope_mut()
            .variables
            .insert(name.to_string(), (ty, mutable));
    }

    /// Look up a variable, searching from innermost to outermost scope
    pub fn lookup_variable(&self, name: &str) -> Option<&(Type, bool)> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.variables.get(name) {
                return Some(binding);
            }
        }
        None
    }

    /// Check if a variable is mutable
    pub fn is_mutable(&self, name: &str) -> Option<bool> {
        self.lookup_variable(name).map(|(_, m)| *m)
    }

    // ── Function operations ──────────────────────────────────

    /// Define a function in the current scope
    pub fn define_function(&mut self, sig: FunctionSig) {
        let name = sig.name.clone();
        self.current_scope_mut().functions.insert(name, sig);
    }

    /// Look up a function signature
    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSig> {
        for scope in self.scopes.iter().rev() {
            if let Some(sig) = scope.functions.get(name) {
                return Some(sig);
            }
        }
        None
    }

    // ── Type operations ──────────────────────────────────────

    /// Define a named type
    pub fn define_type(&mut self, name: &str, ty: Type) {
        self.current_scope_mut().types.insert(name.to_string(), ty);
    }

    /// Look up a named type
    pub fn lookup_type(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.types.get(name) {
                return Some(ty);
            }
        }
        None
    }

    // ── Function context queries ─────────────────────────────

    /// Get the expected return type of the enclosing function
    pub fn expected_return_type(&self) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ref rt) = scope.return_type {
                return Some(rt);
            }
        }
        None
    }

    /// Get the declared effects of the enclosing function
    pub fn declared_effects(&self) -> &[Effect] {
        for scope in self.scopes.iter().rev() {
            if scope.return_type.is_some() {
                return &scope.declared_effects;
            }
        }
        &[]
    }

    /// Check if the enclosing function is declared pure
    pub fn is_in_pure_function(&self) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.return_type.is_some() {
                return scope.is_pure;
            }
        }
        false
    }

    /// Resolve a TypeExpr from the AST into a concrete Type
    pub fn resolve_type_expr(&self, expr: &crate::ast::TypeExpr) -> Type {
        match expr {
            crate::ast::TypeExpr::Named(name) => match name.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "bool" => Type::Bool,
                "str" => Type::Str,
                "char" => Type::Char,
                "none" => Type::None,
                _ => {
                    if let Some(ty) = self.lookup_type(name) {
                        ty.clone()
                    } else {
                        Type::Named(name.clone())
                    }
                }
            },
            crate::ast::TypeExpr::Generic(name, args) => {
                let resolved_args: Vec<Type> =
                    args.iter().map(|a| self.resolve_type_expr(a)).collect();

                match name.as_str() {
                    "list" if resolved_args.len() == 1 => {
                        Type::List(Box::new(resolved_args[0].clone()))
                    }
                    "dict" if resolved_args.len() == 2 => Type::Dict(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    ),
                    _ => Type::Named(name.clone()), // Generic user type — resolve later
                }
            }
            crate::ast::TypeExpr::Function(params, ret) => {
                let param_types: Vec<Type> =
                    params.iter().map(|p| self.resolve_type_expr(p)).collect();
                let return_type = self.resolve_type_expr(ret);
                Type::Function {
                    params: param_types,
                    return_type: Box::new(return_type),
                    effects: vec![],
                }
            }
            crate::ast::TypeExpr::Optional(inner) => {
                Type::Optional(Box::new(self.resolve_type_expr(inner)))
            }
            crate::ast::TypeExpr::Result(ok, err) => Type::Result(
                Box::new(self.resolve_type_expr(ok)),
                Box::new(self.resolve_type_expr(err)),
            ),
            crate::ast::TypeExpr::Tuple(elements) => {
                Type::Tuple(elements.iter().map(|e| self.resolve_type_expr(e)).collect())
            }
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
