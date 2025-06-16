use super::model::{ModelLoaderInner};
use super::errors::ErrorCode;

#[derive(Default)]
pub struct MockModelLoaderInner {}

impl ModelLoaderInner for MockModelLoaderInner {
    fn load(&mut self, _name: String) -> Result<String, ErrorCode> {
        return Err(ErrorCode::NotEnabled);
    }
}
