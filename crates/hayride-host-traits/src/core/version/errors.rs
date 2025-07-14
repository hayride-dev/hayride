use std::fmt;

/// Host side model-loader error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    GetVersionFailed,
    Unknown,
}

// Implement Display for ErrorCode
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ErrorCode::GetVersionFailed => "GetVersionFailed",
            ErrorCode::Unknown => "Unknown",
        };
        write!(f, "{}", description)
    }
}
