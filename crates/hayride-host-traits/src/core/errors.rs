/// Host side error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

/// The list of error codes available to hayride-core.
#[derive(Debug)]
pub enum ErrorCode {
    SetFailed,
    GetFailed,
    ConfigNotSet,
    /// Unsupported operation.
    Unknown,
}
