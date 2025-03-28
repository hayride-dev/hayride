use std::fmt;

/// The list of error codes available to the `wasi-nn` API; this should match
/// what is specified in WIT.
#[derive(Debug)]
pub enum ErrorCode {
    /// Caller module passed an invalid argument.
    InvalidArgument,
    /// Invalid encoding.
    InvalidEncoding,
    /// The operation timed out.
    Timeout,
    /// Runtime error.
    RuntimeError,
    /// Unsupported operation.
    UnsupportedOperation,
    /// Graph is too large.
    TooLarge,
    /// Graph not found.
    NotFound,
}

/// Host side error.
#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub data: anyhow::Error,
}

#[derive(Debug)]
pub enum BackendError {
    FailedTokenization,
    FailedToLoadModel,
    FailedToInitContext,
    FailedDecoding,
    FailedTensorNotSet,
    FailedContextTooLarge,
    FailedResultNotSet,
    FailedToWriteOutput,
    Unknown,
}

// Implement Display for BackendError
impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            BackendError::FailedTokenization => "FailedTokenization",
            BackendError::FailedToLoadModel => "FailedToLoadModel",
            BackendError::FailedToInitContext => "FailedToInitContext",
            BackendError::FailedDecoding => "FailedDecoding",
            BackendError::FailedTensorNotSet => "FailedTensorNotSet",
            BackendError::FailedContextTooLarge => "FailedContextTooLarge",
            BackendError::FailedResultNotSet => "FailedResultNotSet",
            BackendError::FailedToWriteOutput => "FailedToWriteOutput",
            BackendError::Unknown => "Unknown",
        };
        write!(f, "{}", description)
    }
}

// Implement std::error::Error for BackendError
impl std::error::Error for BackendError {}
