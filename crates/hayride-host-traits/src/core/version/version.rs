use super::errors::ErrorCode;
pub trait VersionInner: Send + Sync {
    fn latest(&self) -> Result<String, ErrorCode>;
}
