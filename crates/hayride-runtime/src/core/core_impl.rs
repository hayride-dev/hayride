use super::bindings::types::{Error, ErrorCode};
use super::bindings::{config, types};
use super::core::{CoreImpl, CoreView};

use wasmtime::component::Resource;
use wasmtime::Result;

impl<T> types::Host for CoreImpl<T> where T: CoreView {}

impl<T> types::HostError for CoreImpl<T>
where
    T: CoreView,
{
    fn code(&mut self, error: Resource<Error>) -> Result<ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            hayride_host_traits::core::ErrorCode::SetFailed => Ok(ErrorCode::SetFailed),
            hayride_host_traits::core::ErrorCode::GetFailed => Ok(ErrorCode::GetFailed),
            hayride_host_traits::core::ErrorCode::ConfigNotSet => Ok(ErrorCode::ConfigNotSet),
            hayride_host_traits::core::ErrorCode::Unknown => Ok(ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}

impl<T> config::Host for CoreImpl<T>
where
    T: CoreView,
{
    fn get(&mut self) -> Result<Result<config::Config, Resource<config::Error>>> {
        let result = self.ctx().core_backend.get_config();

        match result {
            Ok(c) => {
                return Ok(Ok(c.into()));
            }
            Err(e) => {
                let r = self.table().push(e)?;
                return Ok(Err(r));
            }
        }
    }

    fn set(&mut self, config: config::Config) -> Result<Result<(), Resource<config::Error>>> {
        let result = self.ctx().core_backend.set_config(config.into());
        match result {
            Ok(_) => {
                return Ok(Ok(()));
            }
            Err(e) => {
                let r = self.table().push(e)?;
                return Ok(Err(r));
            }
        }
    }
}
