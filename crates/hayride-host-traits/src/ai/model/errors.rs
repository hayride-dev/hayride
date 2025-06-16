#[derive(Debug)]
pub enum ErrorCode {
    ModelNotFound,
    InvalidModelName,
    RuntimeError,
    NotEnabled,
    Unknown,
}
