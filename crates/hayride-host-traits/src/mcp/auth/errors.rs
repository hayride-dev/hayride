use std::fmt;

/// Host side model-loader error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    AuthUrlFailed,
    RegistrationFailed,
    ExchangeCodeFailed,
    ValidateFailed,
    Unknown,
}

// Implement Display for ErrorCode
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ErrorCode::AuthUrlFailed => "Auth URL failed",
            ErrorCode::RegistrationFailed => "Registration failed",
            ErrorCode::ExchangeCodeFailed => "Exchange code failed",
            ErrorCode::ValidateFailed => "Validate failed",
            ErrorCode::Unknown => "Unknown",
        };
        write!(f, "{}", description)
    }
}
