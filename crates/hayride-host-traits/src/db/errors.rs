/// Host side error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug)]
pub enum ErrorCode {
    OpenFailed,
    QueryFailed,
    ExecuteFailed,
    PrepareFailed,
    CloseFailed,
    NumberParametersFailed,
    BeginTransactionFailed,
    CommitFailed,
    RollbackFailed,
    NextFailed,
    EndOfRows,
    NotEnabled,
    /// Unsupported operation.
    Unknown,
}
