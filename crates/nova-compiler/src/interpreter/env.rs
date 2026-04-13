use std::collections::HashMap;

use super::value::Value;

/// A scope in the runtime environment
#[derive(Debug, Clone)]
struct Scope {
    bindings: HashMap<String, (Value, bool)>, // name → (value, is_mutable)
}

/// Runtime environment with lexical scoping
#[derive(Debug, Clone)]
pub struct RuntimeEnv {
    scopes: Vec<Scope>,
}

impl Scope {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

impl RuntimeEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
        }
    }

    /// Enter a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Leave the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope
    pub fn define(&mut self, name: &str, value: Value, mutable: bool) {
        self.scopes
            .last_mut()
            .expect("scope stack empty")
            .bindings
            .insert(name.to_string(), (value, mutable));
    }

    /// Look up a variable, searching from innermost to outermost scope
    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some((value, _)) = scope.bindings.get(name) {
                return Some(value);
            }
        }
        None
    }

    /// Set a variable's value (must already exist and be mutable)
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((stored, mutable)) = scope.bindings.get_mut(name) {
                if !*mutable {
                    return Err(format!("cannot assign to immutable variable `{name}`"));
                }
                *stored = value;
                return Ok(());
            }
        }
        Err(format!("undefined variable `{name}`"))
    }

    /// Capture current environment for closures (flattened snapshot)
    pub fn capture(&self) -> Vec<(String, Value)> {
        let mut captured = HashMap::new();
        for scope in &self.scopes {
            for (name, (value, _)) in &scope.bindings {
                captured.insert(name.clone(), value.clone());
            }
        }
        captured.into_iter().collect()
    }
}

impl Default for RuntimeEnv {
    fn default() -> Self {
        Self::new()
    }
}
