use crate::core::bindings::{version, version::ErrorCode};
use crate::core::{CoreImpl, CoreView};
use hayride_host_traits::core::version::Error;

use wasmtime::component::Resource;
use wasmtime::Result;

use anyhow::anyhow;

impl<T> version::Host for CoreImpl<T>
where
    T: CoreView,
{
    fn latest(&mut self) -> Result<Result<String, Resource<version::Error>>> {
        let result = self.ctx().version_backend.latest();

        match result {
            Ok(version) => Ok(Ok(version)),
            Err(e) => {
                let error = Error {
                    code: e,
                    data: anyhow!("Error retrieving latest version"),
                };
                let id = self.table().push(error)?;
                Ok(Err(id))
            }
        }
    }
}

impl<T> version::HostError for CoreImpl<T>
where
    T: CoreView,
{
    fn code(&mut self, error: Resource<Error>) -> Result<ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            hayride_host_traits::core::version::ErrorCode::GetVersionFailed => {
                Ok(ErrorCode::GetVersionFailed)
            }
            hayride_host_traits::core::version::ErrorCode::Unknown => Ok(ErrorCode::Unknown),
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
