use super::errors::ErrorCode;

pub trait ModelRepositoryInner: Send + Sync {
    fn download(&mut self, name: String) -> Result<String, ErrorCode>;
    fn get(&self, name: String) -> Result<String, ErrorCode>;
    fn delete(&mut self, name: String) -> Result<(), ErrorCode>;
    fn list(&self) -> Result<Vec<String>, ErrorCode>;
}
