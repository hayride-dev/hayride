use hayride_host_traits::core::{Config, Error};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CoreBackend {
    config: Arc<Mutex<Option<Config>>>,
}

impl CoreBackend {
    pub fn new(config: Option<Config>) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
        }
    }

    pub fn get_config(&self) -> Option<Config> {
        match self.config.lock() {
            Ok(c) => c.clone(),
            Err(_) => None,
        }
    }
}

impl hayride_host_traits::core::ConfigTrait for CoreBackend {
    fn set_config(&mut self, config: Config) -> Result<(), Error> {
        if let Ok(mut cfg) = self.config.lock() {
            *cfg = Some(config);
            return Ok(());
        } else {
            return Err(Error {
                code: hayride_host_traits::core::ErrorCode::SetFailed,
                data: anyhow::anyhow!("Failed to acquire lock to set config"),
            });
        }
    }

    fn get_config(&mut self) -> Result<Config, Error> {
        if let Ok(config) = self.config.lock() {
            match config.clone() {
                Some(c) => {
                    return Ok(c);
                }
                None => {
                    return Err(Error {
                        code: hayride_host_traits::core::ErrorCode::ConfigNotSet,
                        data: anyhow::anyhow!("Config not set"),
                    });
                }
            }
        } else {
            return Err(Error {
                code: hayride_host_traits::core::ErrorCode::GetFailed,
                data: anyhow::anyhow!("Failed to acquire lock to get config"),
            });
        }
    }
}
