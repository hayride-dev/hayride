use super::errors::ErrorCode;
use super::model::ModelRepositoryInner;

#[derive(Default)]
pub struct MockModelRepositoryInner {}

impl ModelRepositoryInner for MockModelRepositoryInner {
    fn download(&mut self, _name: String) -> Result<String, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }

    fn get(&self, _name: String) -> Result<String, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }

    fn delete(&mut self, _name: String) -> Result<(), ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }

    fn list(&self) -> Result<Vec<String>, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }
}
