use std::fmt;

/// Host side model-loader error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    UnexpectedMessageType,
    PushError,
    MessageNotFound,
    Unknown,
}

// Implement Display for ErrorCode
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ErrorCode::UnexpectedMessageType => "Unexpected message type",
            ErrorCode::PushError => "Error pushing message",
            ErrorCode::MessageNotFound => "Message not found",
            ErrorCode::Unknown => "Unknown",
        };
        write!(f, "{}", description)
    }
}
