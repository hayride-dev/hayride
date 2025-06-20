use super::errors::ErrorCode;

pub trait ModelRepositoryInner: Send + Sync {
    fn download(&mut self, name: String) -> Result<String, ErrorCode>;
}
