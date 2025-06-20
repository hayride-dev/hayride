use super::model::{ModelRepositoryInner};
use super::errors::ErrorCode;

#[derive(Default)]
pub struct MockModelRepositoryInner {}

impl ModelRepositoryInner for MockModelRepositoryInner {
    fn download(&mut self, _name: String) -> Result<String, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }
}
