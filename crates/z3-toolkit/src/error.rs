use std::fmt::Display;

/// Why [`Engine::solve`](crate::Engine::solve) or a readback failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Failure {
    /// `core`: labels of the conflicting labeled constraint. Unlabeled ones not appear.
    Unsat { core: Vec<String> },
    /// Giveup  (timeout, incompletness, ..), or `Sat` without a model.
    Unknown { reason: String },
    /// Reading a value back from the model failed.
    Read(ReadError),
}

impl Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Failure::*;
        match self {
            Unsat { core } if core.is_empty() => write!(f, "unsatisfiable"),
            Unsat { core } => write!(f, "unsatisfiable (core: {})", core.join(", ")),
            Unknown { reason } => write!(f, "unknown ({reason})"),
            Read(e) => write!(f, "readback failed: {e}"),
        }
    }
}

impl From<ReadError> for Failure {
    fn from(e: ReadError) -> Self {
        Failure::Read(e)
    }
}

/// Why [`ModelExt::get`](crate::ModelExt::get) failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadError {
    NoValue { expr: String },
    NotLiteral { expr: String, value: String },
    Convert { value: String, target: &'static str },
}

impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ReadError::*;
        match self {
            NoValue { expr } => write!(f, "no value for `{expr}`"),
            NotLiteral { expr, value } => {
                write!(f, "`{expr}` = `{value}` is not a bool / i64 literal")
            }
            Convert { value, target } => write!(f, "`{value}` doesn't fit in {target}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Failure::Read(e) => Some(e),
            _ => None,
        }
    }
}
