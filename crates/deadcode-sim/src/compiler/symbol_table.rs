use std::collections::HashMap;

/// Whether a variable is global (absolute slot) or local (offset from var_base).
#[derive(Debug, Clone, Copy)]
pub enum VarLocation {
    Global(usize),
    Local(usize),
}

/// A single lexical scope.
struct Scope {
    vars: HashMap<String, usize>,
    /// Whether this is a function scope (uses local offsets).
    is_function: bool,
    /// Next local offset within this function scope.
    next_local: usize,
}

/// Tracks variable names → slot indices across nested scopes.
///
/// Global scope uses absolute slot indices (LoadVar/StoreVar).
/// Function scopes use local offsets (LoadLocal/StoreLocal).
pub struct SymbolTable {
    scopes: Vec<Scope>,
    /// Next global slot index.
    next_global: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut st = Self {
            scopes: Vec::new(),
            next_global: 0,
        };
        // Push global scope.
        st.scopes.push(Scope {
            vars: HashMap::new(),
            is_function: false,
            next_local: 0,
        });
        // Pre-allocate `self` at slot 0.
        st.declare_global("self");
        st
    }

    /// Allocate a named global variable. Returns its absolute slot.
    pub fn declare_global(&mut self, name: &str) -> usize {
        let slot = self.next_global;
        self.next_global += 1;
        self.scopes[0].vars.insert(name.to_string(), slot);
        slot
    }

    /// Push a function scope. Returns the local offset base (0 for the first local).
    pub fn push_function_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            is_function: true,
            next_local: 0,
        });
    }

    /// Pop a function scope. Returns the number of locals allocated.
    pub fn pop_function_scope(&mut self) -> usize {
        let scope = self.scopes.pop().expect("cannot pop global scope");
        scope.next_local
    }

    /// Declare a variable in the current scope.
    /// In function scope: allocates a local offset.
    /// In global scope: allocates a global slot.
    pub fn declare(&mut self, name: &str) -> VarLocation {
        let scope = self.scopes.last_mut().expect("no scope");
        if scope.is_function {
            let offset = scope.next_local;
            scope.next_local += 1;
            scope.vars.insert(name.to_string(), offset);
            VarLocation::Local(offset)
        } else {
            let slot = self.next_global;
            self.next_global += 1;
            scope.vars.insert(name.to_string(), slot);
            VarLocation::Global(slot)
        }
    }

    /// Resolve a variable name. Searches from innermost scope outward.
    pub fn resolve(&self, name: &str) -> Option<VarLocation> {
        // Search function scope first (if any), then global.
        for scope in self.scopes.iter().rev() {
            if let Some(&idx) = scope.vars.get(name) {
                return Some(if scope.is_function {
                    VarLocation::Local(idx)
                } else {
                    VarLocation::Global(idx)
                });
            }
        }
        None
    }

    /// Resolve a variable, or declare it in the current scope if not found.
    /// Mirrors the interpreter's `env.update()` semantics.
    pub fn resolve_or_declare(&mut self, name: &str) -> VarLocation {
        // First try to find it in any scope.
        if let Some(loc) = self.resolve(name) {
            return loc;
        }
        // Not found — declare in current scope.
        self.declare(name)
    }

    /// Total number of global slots allocated.
    pub fn num_globals(&self) -> usize {
        self.next_global
    }
}
