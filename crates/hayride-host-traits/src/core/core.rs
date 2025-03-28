use super::errors::Error;
use super::types::Config;
pub trait ConfigTrait: Send + Sync {
    fn get_config(&mut self) -> Result<Config, Error>;
    fn set_config(&mut self, config: Config) -> Result<(), Error>;
}
