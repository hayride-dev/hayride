/// Host side error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug)]
pub enum ErrorCode {
    FileNotFound,
    ResolveFailed,
    ComposeFailed,
    EncodeFailed,
    /// Unsupported operation.
    Unknown,
}
