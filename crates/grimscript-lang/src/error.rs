#[derive(Debug, Clone)]
pub struct GrimScriptError {
    pub kind: ErrorKind,
    pub message: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    SyntaxError,
    RuntimeError,
    TypeError,
    NameError,
    IndexError,
    StepLimitExceeded,
    Stopped,
}

impl GrimScriptError {
    pub fn syntax(line: u32, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::SyntaxError,
            message: message.into(),
            line,
        }
    }

    pub fn runtime(line: u32, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::RuntimeError,
            message: message.into(),
            line,
        }
    }

    pub fn type_error(line: u32, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::TypeError,
            message: message.into(),
            line,
        }
    }

    pub fn name_error(line: u32, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::NameError,
            message: message.into(),
            line,
        }
    }

    pub fn index_error(line: u32, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::IndexError,
            message: message.into(),
            line,
        }
    }

    pub fn step_limit(line: u32) -> Self {
        Self {
            kind: ErrorKind::StepLimitExceeded,
            message: "Step limit exceeded".into(),
            line,
        }
    }

    pub fn stopped(line: u32) -> Self {
        Self {
            kind: ErrorKind::Stopped,
            message: "Execution stopped".into(),
            line,
        }
    }
}

impl std::fmt::Display for GrimScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} at line {}: {}", self.kind, self.line, self.message)
    }
}

impl std::error::Error for GrimScriptError {}
