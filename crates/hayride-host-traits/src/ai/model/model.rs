use super::errors::ErrorCode;

pub trait ModelLoaderInner: Send + Sync {
    fn load(&mut self, name: String) -> Result<String, ErrorCode>;
}
