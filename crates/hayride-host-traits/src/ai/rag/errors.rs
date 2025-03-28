/// Host side Rag error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

/// The list of error codes available to the `rag` API; this should match
/// what is specified in WIT.
#[derive(Debug)]
pub enum ErrorCode {
    ConnectionFailed,
    CreateTableFailed,
    QueryFailed,
    EmbedFailed,
    RegisterFailed,
    MissingTable,
    InvalidOption,
    NotEnabled,
    Unknown,
}
