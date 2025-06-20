use std::fmt;

/// Host side model-loader error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    ModelNotFound,
    InvalidModelName,
    RuntimeError,
    NotEnabled,
    Unknown,
}

// Implement Display for ErrorCode
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ErrorCode::ModelNotFound => "ModelNotFound",
            ErrorCode::InvalidModelName => "InvalidModelName",
            ErrorCode::RuntimeError => "RuntimeError",
            ErrorCode::NotEnabled => "NotEnabled",
            ErrorCode::Unknown => "Unknown",
        };
        write!(f, "{}", description)
    }
}