use std::collections::HashMap;

use crate::value::Value;

pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        Option::None
    }

    /// Set a variable in the current (top) scope.
    pub fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Update an existing variable in the scope where it was defined.
    /// If not found anywhere, set in the current scope.
    pub fn update(&mut self, name: String, value: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if let std::collections::hash_map::Entry::Occupied(mut e) = scope.entry(name.clone()) {
                e.insert(value);
                return;
            }
        }
        // Not found, set in current scope
        self.set(name, value);
    }

    /// Get all variables (for debug info). Returns a flat list
    /// where inner scopes shadow outer ones.
    pub fn all_variables(&self) -> Vec<(String, Value)> {
        let mut merged: HashMap<String, Value> = HashMap::new();
        for scope in &self.scopes {
            for (k, v) in scope {
                merged.insert(k.clone(), v.clone());
            }
        }
        let mut result: Vec<(String, Value)> = merged.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
