use std::fmt;

#[derive(Debug, Clone)]
pub enum SimErrorKind {
    /// Type mismatch in an operation (e.g., adding int + string).
    TypeError,
    /// Division by zero.
    DivisionByZero,
    /// Index out of bounds on a list.
    IndexOutOfBounds,
    /// Key not found in a dict.
    KeyNotFound,
    /// Referenced entity does not exist.
    EntityNotFound,
    /// Stack underflow (compiler bug or corrupted IR).
    StackUnderflow,
    /// Variable slot out of range.
    InvalidVariable,
    /// Call stack overflow (too many nested calls).
    StackOverflow,
    /// Exceeded per-tick instruction limit.
    StepLimitExceeded,
    /// Generic runtime error.
    Runtime,
}

#[derive(Debug, Clone)]
pub struct SimError {
    pub kind: SimErrorKind,
    pub message: String,
}

impl SimError {
    pub fn new(kind: SimErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn type_error(msg: impl Into<String>) -> Self {
        Self::new(SimErrorKind::TypeError, msg)
    }

    pub fn division_by_zero() -> Self {
        Self::new(SimErrorKind::DivisionByZero, "division by zero")
    }

    pub fn index_out_of_bounds(index: i64, len: usize) -> Self {
        Self::new(
            SimErrorKind::IndexOutOfBounds,
            format!("index {index} out of bounds for list of length {len}"),
        )
    }

    pub fn key_not_found(key: &str) -> Self {
        Self::new(SimErrorKind::KeyNotFound, format!("key not found: \"{key}\""))
    }

    pub fn entity_not_found(id: u64) -> Self {
        Self::new(
            SimErrorKind::EntityNotFound,
            format!("entity {id} not found"),
        )
    }

    pub fn stack_underflow() -> Self {
        Self::new(SimErrorKind::StackUnderflow, "stack underflow")
    }

    pub fn invalid_variable(slot: usize) -> Self {
        Self::new(
            SimErrorKind::InvalidVariable,
            format!("invalid variable slot {slot}"),
        )
    }

    pub fn stack_overflow() -> Self {
        Self::new(SimErrorKind::StackOverflow, "call stack overflow")
    }

    pub fn step_limit() -> Self {
        Self::new(
            SimErrorKind::StepLimitExceeded,
            "exceeded per-tick instruction limit",
        )
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SimError {}
