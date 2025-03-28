use super::errors::ErrorCode;
use super::rag::{Connection, RagInner};

#[derive(Default)]
pub struct MockRagInner {}

impl RagInner for MockRagInner {
    fn connect(&mut self, _dsn: String) -> Result<Connection, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }
}
