use super::errors::ErrorCode;
pub trait WacTrait: Send + Sync {
    fn compose(&mut self, path: String) -> Result<Vec<u8>, ErrorCode>;
    fn plug(&mut self, socket_path: String, plug_paths: Vec<String>) -> Result<Vec<u8>, ErrorCode>;
}
