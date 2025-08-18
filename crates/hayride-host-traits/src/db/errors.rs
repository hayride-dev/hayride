/// Host side error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug)]
pub enum ErrorCode {
    ConnectionFailed,
    QueryFailed,
    ExecuteFailed,
    CloseFailed,
    NotEnabled,
    /// Unsupported operation.
    Unknown,
}
