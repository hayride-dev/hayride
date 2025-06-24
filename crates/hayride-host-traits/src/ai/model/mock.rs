use super::errors::ErrorCode;
use super::model::ModelRepositoryInner;

#[derive(Default)]
pub struct MockModelRepositoryInner {}

impl ModelRepositoryInner for MockModelRepositoryInner {
    fn download(&mut self, _name: String) -> Result<String, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }
}
