use super::errors::ErrorCode;
use super::version::VersionInner;

#[derive(Default)]
pub struct MockVersionInner {}

impl VersionInner for MockVersionInner {
    fn latest(&self) -> Result<String, ErrorCode> {
        Ok("mock-version".into())
    }
}
